use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use axum::extract::Request;
use axum::extract::ws::Message;
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{Mutex, RwLock};

use utils::Never;
use server::{ComponentHandle, Notification, RequestHandle};
use crate::filters::Filter;

fn default_paths() -> HashSet<String> {
    HashSet::from([
        "/api/ws".to_string(),
        "/api/websocket".to_string(),
        "/api/socket".to_string(),
        "/ws".to_string(),
        "/websocket".to_string(),
        "/socket".to_string(),
    ])
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Config {
    #[serde(default="default_paths")]
    paths: HashSet<String>,
    #[serde(default)]
    filter: Filter,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            paths: default_paths(),
            filter: Filter::default(),
        }
    }
}


struct Socket {
    ws: Mutex<futures_util::stream::SplitSink<axum::extract::ws::WebSocket, axum::extract::ws::Message>>,
    online: AtomicBool,
    #[cfg(feature = "auth")]
    user: RwLock<crate::auth::User>,
}

/// Provides Websockets at the configured path, sending [`Notification`]s via the Socket.
pub struct Websockets {
    sockets: Arc<RwLock<Vec<Arc<Socket>>>>,
    config: Config,
    state: ComponentHandle
}
impl server::Component for Websockets {
    const ID: &'static str = "sockets";
    type Config = crate::Notification<Config>;
    type ConfigError = Never;

    fn init(state: ComponentHandle, config: Self::Config) -> Result<Self, Self::ConfigError> {
        let config = config.notification;
        let websockets = Arc::new(RwLock::new(Vec::<Arc<Socket>>::new()));
        let mut ticker = tokio::time::interval(Duration::from_mins(30));
        let ws = websockets.clone();
        tokio::spawn(async move {
            ticker.tick().await;
            loop {
                ticker.tick().await;
                let mut lock = ws.write().await;
                lock.retain(|socket| socket.online.load(Ordering::Relaxed));
            }
        });
        trace!("loaded websockets with config {config:?}");
        Ok(Self {
            sockets: websockets,
            config,
            state,
        })
    }

    fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::ConfigError> {
        self.config = config.notification;
        Ok(())
    }

    fn try_handle(&self, request: Request) -> Result<RequestHandle, Request> {
        if !self.config.paths.contains(request.uri().path()) { return Err(request) }
        let websockets = self.sockets.clone();
        let state= self.state.clone();
        Ok(Box::pin(async move {
            use axum::extract::{
                ws::WebSocketUpgrade,
                FromRequest,
            };
            match WebSocketUpgrade::from_request(request, &()).await {
                Ok(upgrade) => {
                    upgrade.on_upgrade(move |socket| async move {
                        let (sender, mut receiver) = socket.split();
                        let socket = Arc::new(Socket {
                            ws: Mutex::new(sender),
                            online: AtomicBool::new(true),
                            #[cfg(feature="auth")]
                            user: RwLock::new(crate::auth::User::UNAUTHED),
                        });
                        websockets.write().await.push(socket.clone());
                        tokio::spawn(async move {
                            while let Some(Ok(msg)) = receiver.next().await {
                                match msg {
                                    Message::Text(txt) => {
                                        let content: api_types::websocket::UpstreamMessage = match serde_json::from_str(txt.as_str()) {
                                            Ok(v) => v,
                                            Err(e) => {
                                                error!("received invalid JSON via websocket: {e}");
                                                continue
                                            }
                                        };
                                        match content {
                                            #[cfg(feature = "auth")]
                                            api_types::websocket::UpstreamMessage::Login(session_id) => {
                                                let user = match crate::auth::User::from_session_id(Some(session_id), &state) {
                                                    Ok(u) => u,
                                                    Err(e) => {
                                                        error!("error accessing user: {e}");
                                                        continue;
                                                    }
                                                };
                                                *socket.user.write().await = user;
                                            }
                                            #[cfg(not(feature="auth"))]
                                            api_types::websocket::UpstreamMessage::Login(_) => {}
                                        }
                                    }
                                    _ => continue,
                                }
                            }
                            socket.online.store(false, Ordering::Relaxed);
                        });
                    })
                }
                Err(e) => e.into_response(),
            }
        }))
    }
}
impl server::NotificationProvider for Websockets {
    fn notify(&self, notification: Notification) {
        use axum::extract::ws::{Message, Utf8Bytes};
        let is_allowed = self.config.filter.allows(&notification);
        let sockets = self.sockets.clone();
        let message: Utf8Bytes = match serde_json::to_string(&api_types::websocket::Message::from(notification)) {
            Ok(v) => v,
            Err(e) => {
                error!("couldn't serialize notification: {e}");
                return;
            }
        }.into();
        tokio::spawn(async move {
            let sockets = sockets;
            for socket in sockets.read().await.iter() {
                let user = socket.user.read().await;
                if !socket.online.load(Ordering::Relaxed) ||
                    !is_allowed && !user.is_admin && !user.ignores_default_api_rules {
                    continue;
                }
                let msg = message.clone();
                trace!("sending {message} to websockets");
                if let Err(e) = socket.ws.lock().await
                    .send(Message::Text(msg)).await {
                    error!("error sending to websocket: {e}");
                    socket.online.store(false, Ordering::Relaxed);
                }
            }
        });
    }
}

#[cfg(test)]
mod test {
    use server::Component;
    use super::*;
    use crate::parse_test;
    use crate::filters::{AttributeChange, AttributeEvent, AttributeIdMatcher, FilterPriority, SingleFilter, StateChange};

    parse_test!(empty(<Websockets as Component>::Config): toml::Table::new() => error);
    parse_test!(path(<Websockets as Component>::Config): toml!{
        [notify]
        paths = ["/tmp", "/tmp/ws"]
    } => crate::Notification::new(Config {
        paths: HashSet::from(["/tmp".to_string(), "/tmp/ws".to_string()]),
        filter: Filter::default(),
    }));
    parse_test!(filter(<Websockets as Component>::Config): toml!{
        [notify]
        filter.changes.deny = [ { attribute.id="minecraft.players", attribute.exact=false } ]
    } => crate::Notification::new(Config {
        paths: HashSet::from([
            "/api/ws".to_string(),
            "/api/websocket".to_string(),
            "/api/socket".to_string(),
            "/ws".to_string(),
            "/websocket".to_string(),
            "/socket".to_string(),
        ]),
        filter: Filter {
            component: SingleFilter::default(),
            entity: SingleFilter::default(),
            state_changes: SingleFilter {
                whitelist: vec![],
                blacklist: vec![StateChange::AttributeChange(AttributeChange {
                    id: Some(AttributeIdMatcher {
                        id: "minecraft.players".to_string(),
                        exact: false,
                    }),
                    event: AttributeEvent::Any,
                })],
                priority: FilterPriority::Whitelist,
            },
        },
    }));
    parse_test!(full(<Websockets as Component>::Config): toml!{
        [notify]
        paths = ["/tmp", "/tmp/ws"]
        filter.changes.deny = [ { attribute.id="minecraft.players", attribute.exact=false } ]
    } => crate::Notification::new(Config {
        paths: HashSet::from(["/tmp".to_string(), "/tmp/ws".to_string()]),
        filter: Filter {
            component: SingleFilter::default(),
            entity: SingleFilter::default(),
            state_changes: SingleFilter {
                whitelist: vec![],
                blacklist: vec![StateChange::AttributeChange(AttributeChange {
                    id: Some(AttributeIdMatcher {
                        id: "minecraft.players".to_string(),
                        exact: false,
                    }),
                    event: AttributeEvent::Any,
                })],
                priority: FilterPriority::Whitelist,
            },
        },
    }));
}