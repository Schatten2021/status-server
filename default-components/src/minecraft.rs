use std::collections::HashMap;
use tokio::time::MissedTickBehavior;
use utils::Never;
use server::{AttributeValue, Component, ComponentHandle};

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub java: HashMap<String, JavaConfig>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JavaConfig {
    pub url: String,
    #[serde(default="java_default_port")]
    pub port: u16,
    #[serde(default="hourly")]
    #[serde(with="utils::duration_parsing")]
    pub interval: chrono::Duration,
}
const fn java_default_port() -> u16 {
    25565
}
const fn hourly() -> chrono::Duration {
    chrono::Duration::hours(1)
}
/// [`Component`] to allow keeping track of minecraft servers.
///
/// # Attributes
/// Sets the following attributes (if provided by the server):
/// - `minecraft.version`: The version of the minecraft server
/// - `minecraft.protocol`: The protocol version of the minecraft server
/// - `minecraft.players.max`: The maximum number of players that can be online at the same time
/// - `minecraft.players.online`: The amount of currently online players
/// - `minecraft.players.sample`: A sample of the players that are online (if any player is online & the server provides it)
/// - `minecraft.description`: The description of the server
/// - `minecraft.favicon`: The favicon of the server
/// - `minecraft.enforces_secure_chat`: whether the server enforces secure chat.
pub struct MinecraftStatus {
    config: Config,
    task_handles: HashMap<String, tokio::task::JoinHandle<()>>,
    state: ComponentHandle,
}
impl Component for MinecraftStatus {
    const ID: &'static str = "minecraft";
    type Config = crate::Status<Config>;
    type ConfigError = Never;

    fn init(server: ComponentHandle, config: Self::Config) -> Result<Self, Self::ConfigError> {
        let config = config.status;
        let handles = config.java.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .map(|(id, conf)| (id.clone(), start_ping(id, conf, server.clone())))
            .collect();
        Ok(Self {
            config,
            task_handles: handles,
            state: server,
        })
    }

    fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::ConfigError> {
        let config = config.status;
        // JAVA
        for id in self.config.java.keys()
            .filter(|k| !config.java.contains_key(*k))
            .cloned()
            .collect::<Vec<_>>() {
            if let Some(old) = self.task_handles.remove(&id) {
                old.abort();
            }
            self.config.java.remove(&id);
        }
        for (id, new_config) in config.java.into_iter()
            .filter(|(id, new_conf)| self.config.java.get(id) != Some(new_conf))
            .collect::<Vec<_>>()
        {
            self.config.java.insert(id.clone(), new_config.clone());
            if let Some(old) = self.task_handles.insert(id.clone(), start_ping(id, new_config, self.state.clone())) {
                old.abort();
            }
        }
        Ok(())
    }
}
fn start_ping(id: String, conf: JavaConfig, state: ComponentHandle) -> tokio::task::JoinHandle<()> {
    let mut ticker = tokio::time::interval(conf.interval.to_std().expect("unable to convert to std time"));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    tokio::spawn(async move {
        loop {
            ticker.tick().await;

            let response = send_ping(&conf.url, conf.port);
            let is_ok = response.is_ok();
            if state.get_online_state(&id) != Some(is_ok) {
                state.change_online_state(&id, is_ok);
            }

            let Ok(response) = response else { continue; };
            macro_rules! set_if_unchanged {
                ($attr_id:literal, $val:expr) => {{
                    let new_val = $val;
                    if !matches!(state.get_attribute(&id, $attr_id), Some(v) if v == new_val) {
                        state.change_attribute(&id, $attr_id, new_val);
                    }
                }};
            }
            set_if_unchanged!("minecraft.last_seen", AttributeValue::Date(chrono::Utc::now()));
            set_if_unchanged!("minecraft.version", AttributeValue::String(response.version.name));
            set_if_unchanged!("minecraft.protocol.version", AttributeValue::Number(response.version.protocol.into()));
            if let Some(players) = response.players {
                set_if_unchanged!("minecraft.players.max", AttributeValue::Number(players.max as i128));
                set_if_unchanged!("minecraft.players.online", AttributeValue::Number(players.online as i128));
                if let Some(sample) = players.sample {
                    set_if_unchanged!("minecraft.players.sample", AttributeValue::List(
                        sample.into_iter()
                        .map(|player| AttributeValue::String(player.name))
                        .collect::<Vec<_>>()
                    ));
                } else {
                    state.delete_attribute(&id, "minecraft.players.sample", true);
                }
            } else {
                state.delete_attribute(&id, "state.players.max", true);
                state.delete_attribute(&id, "state.players.online", true);
                state.delete_attribute(&id, "state.players.sample", true);
            }
            if let Some(description) = response.description {
                set_if_unchanged!("minecraft.description", AttributeValue::String(description));
            } else {
                state.delete_attribute(&id, "minecraft.description", true);
            }
            if let Some(favicon) = response.favicon {
                set_if_unchanged!("minecraft.favicon", AttributeValue::String(favicon));
            } else {
                state.delete_attribute(&id, "minecraft.favicon", true);
            }
            if let Some(secure_chat) = response.enforcesSecureChat {
                set_if_unchanged!("minecraft.enforces_secure_chat", AttributeValue::Boolean(secure_chat));
                if secure_chat {
                    set_if_unchanged!("minecraft.retarded", AttributeValue::Unit);
                } else {
                    state.delete_attribute(&id, "minecraft.retarded", true);
                }
            } else {
                state.delete_attribute(&id, "minecraft.enforces_secure_chat", true);
                state.delete_attribute(&id, "minecraft.retarded", true);
            }
        }
    })
}
fn send_ping(url: &str, port: u16) -> Result<StatusResponse, ()> {
    let mut conn = std::net::TcpStream::connect((url, port))
        .map_err(|e| error!("failed to connect to Minecraft server: {e}"))?;
    minecraft_net::send_packet(
        minecraft_net::packets::handshake::upstream::Handshake::new(url.to_string(), port, 1),
        &mut conn,
        None
    ).map_err(|e| error!("failed to send Handshake to Minecraft server: {e:?}"))?;
    minecraft_net::send_packet(
        minecraft_net::packets::status::upstream::StatusRequest::new(),
        &mut conn,
        None,
    ).map_err(|e| error!("failed to request status to Minecraft server: {e:?}"))?;
    let response = minecraft_net::receive_packet::<minecraft_net::packets::status::downstream::StatusResponse>(conn, false)
        .map_err(|e| error!("failed to read status response from Minecraft server: {e:?}"))?;
    trace!("received Minecraft server status response: \"{}\"", response.status);
    serde_json::from_str::<StatusResponse>(&response.status)
        .map_err(|e| error!("received invalid JSON from minecraft server: {e}"))
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[allow(non_snake_case, reason="this a JSON struct that I don't control.")]
struct StatusResponse {
    version: StatusResponseVersion,
    players: Option<StatusResponsePlayers>,
    description: Option<String>,
    favicon: Option<String>,
    enforcesSecureChat: Option<bool>,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct StatusResponseVersion {
    name: String,
    protocol: u16,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct StatusResponsePlayers {
    max: usize,
    online: usize,
    sample: Option<Vec<StatusResponsePlayerSample>>
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct StatusResponsePlayerSample {
    name: String,
    id: String,
}

#[cfg(test)]
mod test {
    use crate::config_wrappers::Status;
    use super::*;
    use crate::parse_test;
    parse_test!(empty(<MinecraftStatus as Component>::Config): toml::Table::new() => error);
    parse_test!(parse(<MinecraftStatus as Component>::Config): toml!{
        [status.java.foo]
        url = "foo.example"

        [status.java.bar]
        url = "bar.example"
        port = 42
        interval = "5h"

    } => Status::new(Config {
        java: HashMap::from([
            ("foo".to_string(), JavaConfig {
                url: "foo.example".to_string(),
                port: java_default_port(),
                interval: hourly(),
            }),
            ("bar".to_string(), JavaConfig {
                url: "bar.example".to_string(),
                port: 42,
                interval: chrono::Duration::hours(5)
            })
        ])
    }));
}
