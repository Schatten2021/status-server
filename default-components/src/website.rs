use std::collections::{HashMap};
use tokio::time::MissedTickBehavior;
use server::{AttributeValue, ComponentHandle};
use utils::Never;
use crate::filters::SingleFilter;

const LAST_SEEN_ID: &str = "website.last_seen";

const fn hourly() -> chrono::Duration { chrono::Duration::hours(1) }

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Config {
    url: String,
    #[serde(default="hourly")]
    #[serde(with="utils::duration_parsing")]
    interval: chrono::Duration,
    #[serde(default)]
    status: SingleFilter<u16>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            url: "https://example.com".to_string(),
            interval: hourly(),
            status: SingleFilter::default()
        }
    }
}
/// Component for keeping track of the status of websites.
pub struct WebsiteStatuse {
    config: HashMap<String, Config>,
    task_handles: HashMap<String, tokio::task::JoinHandle<()>>,
    state: ComponentHandle,
}
impl server::Component for WebsiteStatuse {
    const ID: &'static str = "website";
    type Config = crate::Status<HashMap<String, Config>>;
    type ConfigError = Never;

    fn init(server: ComponentHandle, config: Self::Config) -> Result<Self, Self::ConfigError> {
        let config = config.status;
        Ok(Self {
            task_handles: config.iter()
                .map(|(a, b)| (a.clone(), b.clone()))
                .map(|(id, config)| {
                    let handle = server.clone();
                    (id.clone(), spawn_listen_task(id, config, handle))
                }).collect(),
            config,
            state: server,
        })
    }

    fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::ConfigError> {
        let config = config.status;
        for id in self.config.keys()
            .filter(|k| !config.contains_key(*k))
            .cloned()
            .collect::<Vec<_>>() {
            if let Some(old) = self.task_handles.remove(&id) {
                old.abort();
            }
            self.config.remove(&id);
        }
        for (id, new_config) in config.into_iter()
            .filter(|(id, new_conf)| self.config.get(id) != Some(new_conf))
            .collect::<Vec<_>>()
        {
            let handle = self.state.clone();
            self.config.insert(id.clone(), new_config.clone());
            if let Some(old) = self.task_handles.insert(id.clone(), spawn_listen_task(id, new_config, handle)) {
                old.abort();
            }
        }
        Ok(())
    }
}
fn spawn_listen_task(id: String, config: Config, state: ComponentHandle) -> tokio::task::JoinHandle<()> {
    let mut ticker = tokio::time::interval(config.interval.to_std().expect("couldn't convert interval to std interval"));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    tokio::task::spawn(async move {
        let client = reqwest::Client::new();
        loop {
            ticker.tick().await;
            let new_status = matches!(request_website(&client, &config).await, Ok(true));
            let old_state = state.get_online_state(&id);
            trace!("old state: {old_state:?}; new_state: {new_status}");
            if Some(new_status) != old_state {
                info!("webserver {} has changed", id);
                state.change_online_state(&id, new_status);
            }
            if new_status {
                trace!("successfully requested {}", config.url);
                state.change_attribute(&id, LAST_SEEN_ID, AttributeValue::Timestamp(chrono::Utc::now()));
            } else {
                trace!("failed to request {}", config.url);
            }
        }
    })
}
async fn request_website(client: &reqwest::Client, config: &Config) -> Result<bool, ()> {
    let status = client.get(&config.url)
        .send().await.map_err(|e| {
        error!("couldn't request {:?}: {e}", config.url);
    })?
        .status();
    Ok(config.status.whitelisted(&status.as_u16()) || status.is_success() && !config.status.blacklisted(&status.as_u16()))
}
#[cfg(test)]
mod test {
    use server::Component;
    use crate::filters::FilterPriority;
    use crate::parse_test;
    use super::*;
    parse_test!(empty(<WebsiteStatuse as Component>::Config): toml::Table::new() => error);
    parse_test!(parse(<WebsiteStatuse as Component>::Config): toml!{
        [status.foo]
        url = "foo.example.com"

        [status.bar]
        url = "bar.example.com"
        interval = "5h10m"
        status.accept = [401]
        status.default = "deny"
    } => crate::Status::new(HashMap::from([
        ("foo".to_string(), Config {
            url: "foo.example.com".to_string(),
            interval: hourly(),
            status: SingleFilter::default(),
        }),
        ("bar".to_string(), Config {
            url: "bar.example.com".to_string(),
            interval: chrono::Duration::hours(5) + chrono::Duration::minutes(10),
            status: SingleFilter {
                whitelist: vec![401],
                blacklist: vec![],
                priority: FilterPriority::Blacklist,
            }
        })
    ])));
}