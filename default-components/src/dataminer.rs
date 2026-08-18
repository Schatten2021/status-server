use std::collections::HashMap;
use axum::extract::Request;
use tokio::time::MissedTickBehavior;
use utils::Never;
use server::{AttributeValue, Component, ComponentHandle, RequestHandle};


const LAST_SEEN_ID: &str = "miner.last_seen";

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    #[serde(with="utils::duration_parsing")]
    timeout: chrono::Duration,
}

/// [`Component`] for keeping track of dataminers.
///
/// Dataminers are expected to repeatedly ping "/miner/ping?id={miner-id}" to assert that they are,
/// in fact, still running.
pub struct DataminerStatus {
    config: HashMap<String, Config>,
    timeout_handles: HashMap<String, tokio::task::JoinHandle<()>>,
    server: ComponentHandle,
}
impl Component for DataminerStatus {
    const ID: &'static str = "miner";
    type Config = crate::Status<HashMap<String, Config>>;
    type ConfigError = Never;

    fn init(server: ComponentHandle, config: Self::Config) -> Result<Self, Self::ConfigError> {
        let config = config.status;
        let timeout_handles = config.iter()
            .map(|(a, b)| (a.clone(), b.clone()))
            .map(|(a, b)| (a.clone(), spawn_timeout_task(a, b, server.clone())))
            .collect();
        Ok(Self {
            config,
            timeout_handles,
            server,
        })
    }

    fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::ConfigError> {
        let config = config.status;
        for id in self.config.keys()
            .filter(|k| !config.contains_key(*k))
            .cloned()
            .collect::<Vec<_>>() {
            if let Some(old) =self.timeout_handles.remove(&id) {
                old.abort();
            }
            self.config.remove(&id);
        }
        for (id, new_config) in config.into_iter()
            .filter(|(id, new_conf)| self.config.get(id) != Some(new_conf))
            .collect::<Vec<_>>()
        {
            let handle = self.server.clone();
            self.config.insert(id.clone(), new_config.clone());
            if let Some(old) = self.timeout_handles.insert(id.clone(), spawn_timeout_task(id, new_config, handle)) {
                old.abort();
            }
        }
        Ok(())
    }
    fn try_handle(&self, request: Request) -> Result<RequestHandle, Request> {
        if !matches!(request.uri().path(), "/miner/ping") { return Err(request) }
        let Some(args) = request.uri().query() else { return Err(request) };
        if !args.starts_with("id=") || args.contains('&') { return Err(request) }
        let id = args["id=".len()..].to_string();
        let server = self.server.clone();
        Ok(Box::pin(async move {
            server.change_attribute(&id, LAST_SEEN_ID, AttributeValue::Date(chrono::Utc::now()));
            if !matches!(server.get_online_state(&id), Some(true)) {
                server.change_online_state(&id, true);
            }
            axum::response::Response::builder()
                .status(200)
                .body(axum::body::Body::empty()).unwrap()
        }))
    }
}
fn spawn_timeout_task(id: String, config: Config, handle: ComponentHandle) -> tokio::task::JoinHandle<()> {
    let mut ticker = tokio::time::interval(config.timeout.to_std().expect("invalid timeout"));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    tokio::spawn(async move {
        let config = config;
        loop {
            ticker.tick().await;
            let last_seen = if let Some(AttributeValue::Date(dt)) = handle.get_attribute(&id, LAST_SEEN_ID) {
                Some(dt)
            } else {
                trace!("dataminer never seen before");
                None
            };
            let now = chrono::Utc::now();
            let now_std = std::time::Instant::now();
            let is_online = last_seen.is_some_and(|timestamp| (timestamp - now) > config.timeout);
            if handle.get_online_state(&id) != Some(is_online) {
                debug!("miner {id} changed to {}", if is_online { "online" } else { "offline" });
                handle.change_online_state(&id, is_online);
                if let Some(last_ping) = last_seen {
                    let diff = now - last_ping;
                    let diff = diff.to_std().expect("last ping somehow after now?");
                    #[expect(clippy::unchecked_time_subtraction, reason="it is very unlikely that `diff` will be large enough to underflow the Instant.")]
                    let last_ping_std = now_std - diff;
                   ticker.reset_at(tokio::time::Instant::from_std(last_ping_std + config.timeout.to_std().unwrap()));
                }
            }
        }
    })
}
impl Drop for DataminerStatus {
    fn drop(&mut self) {
        self.timeout_handles.values_mut()
            .for_each(|h| h.abort());
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{parse_test, Status};
    parse_test!(empty(<DataminerStatus as Component>::Config): toml::Table::new() => error);
    parse_test!(parse(<DataminerStatus as Component>::Config): toml!{
        [status.foo]
        timeout = "100s"
        [status.bar]
        timeout = "6h"
    } => Status::new(HashMap::from([
        ("foo".to_string(), Config { timeout: chrono::Duration::seconds(100) }),
        ("bar".to_string(), Config { timeout: chrono::Duration::hours(6) }),
    ])));
}