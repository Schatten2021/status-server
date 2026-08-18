use lettre::transport::smtp::authentication::Credentials;
use utils::Never;
use server::{Component, ComponentHandle, Notification, NotificationProvider, NotificationReason};
use crate::filters::Filter;

fn default_name() -> String { "No Reply".to_string() }

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Config {
    address: String,
    password: String,
    server: String,
    #[serde(default="default_name")]
    name: String,
    #[serde(alias="subscribed", alias="notify", alias="alert")]
    subscribers: Vec<Subscriber>,
    #[serde(default)]
    filter: Filter,
}
impl Default for Config {
    fn default() -> Self {
        error!("No information provided for EmailNotificationProvider. Please add the required config to the config file.");
        Self {
            address: String::new(),
            password: String::new(),
            server: String::new(),
            name: default_name(),
            subscribers: Vec::new(),
            filter: Filter::default(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum Subscriber {
    #[serde(untagged)]
    Default(String),
    #[serde(untagged)]
    Custom {
        #[serde(alias="address", alias="mail", alias="to")]
        email: String,
        #[serde(default)]
        filter: Filter,
    },
}
impl Subscriber {
    const fn get_email(&self) -> &'_ String {
        match self {
            Subscriber::Default(address) => address,
            Subscriber::Custom { email, .. } => email,
        }
    }
    fn allows(&self, notification: &Notification) -> bool {
        match self {
            Subscriber::Custom { filter, .. } => filter.allows(notification),
            Subscriber::Default(_) => true,
        }
    }
}
#[derive(Clone, Debug)]
/// [`NotificationProvider`] to send notifications via E-Mail.
pub struct EmailNotificationProvider {
    config: Config,
    credentials: Credentials,
}

impl Component for EmailNotificationProvider {
    const ID: &'static str = "email";
    type Config = crate::Notification<Config>;
    type ConfigError = Never;

    fn init(_: ComponentHandle, config: Self::Config) -> Result<Self, Self::ConfigError> {
        let config = config.notification;
        Ok(Self {
            credentials: Credentials::new(config.address.clone(), config.password.clone()),
            config,
        })
    }

    fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::ConfigError> {
        self.config = config.notification;
        self.credentials = Credentials::new(self.config.address.clone(), self.config.password.clone());
        Ok(())
    }
}
impl NotificationProvider for EmailNotificationProvider {
    fn notify(&self, notification: Notification) {
        if !self.config.filter.allows(&notification) { return; }
        let (subject, body) = match &notification.reason {
            NotificationReason::OnlineStatusChanged(true) => (
                format!("{} went online", notification.element_id),
                format!(r"<h1> <code>{}</code> just went online</h1>Everything is fine", notification.element_id)
            ),
            NotificationReason::OnlineStatusChanged(false) => (
                format!("{} went offline", notification.element_id),
                format!(r"<h1><code>{}</code> just went offline!</h1> Go check up on it!", notification.element_id)
            ),
            NotificationReason::AttributeCreated(attr, val) => (
                format!("{} just got the attribute {}", notification.element_id, attr),
                format!("The new value of <code>{}</code> for {} is: {}", attr, notification.element_id, val)
            ),
            NotificationReason::AttributeChanged(id, old, new) => (
                format!("{id} of {} just changed value", notification.element_id),
                format!("{id} of {} just changed from {old} to {new}", notification.element_id)
            ),
            NotificationReason::AttributeDeleted(id, val) => (
                format!("{id} of {} just got deleted", notification.element_id),
                format!("{id} of {} just got deleted ({val})", notification.element_id)
            ),
            NotificationReason::NewElement(true) => (
                format!("{} just got created (online)", notification.element_id),
                format!("just got word that {} exists and is online.", notification.element_id)
            ),
            NotificationReason::NewElement(false) => (
                format!("{} just got created (offline)", notification.element_id),
                format!("just got word that {} exists and is offline.", notification.element_id)
            ),
        };
        let cloned = self.clone();
        tokio::task::spawn(async move {
            if let Err(e) = cloned.send_message(subject, &body, &notification) {
                error!("error sending E-Mail: {e}");
            }
        });
    }
}
impl EmailNotificationProvider {
    fn send_message(self,
                    subject: String,
                    body: &str,
                    notification: &Notification) -> Result<(), Box<dyn std::error::Error>> {
        use lettre::Transport;
        trace!("sending email: {:?}", body);
        let mailer = lettre::transport::smtp::SmtpTransport::relay(&self.config.server)?
            .credentials(self.credentials)
            .build();
        let builder_preset = lettre::Message::builder()
            .from(format!("{} <{}>", self.config.name, self.config.address).parse()?)
            .subject(subject)
            .header(lettre::message::header::ContentType::TEXT_HTML);
        for target in self.config.subscribers {
            if !target.allows(notification) { continue; }
            trace!("sending email to {}", target.get_email());
            mailer.send(&builder_preset.clone()
                .to(target.get_email().parse()?)
                .body(body.to_string())?)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Notification;
    mod parsing {
        use crate::filters::{FilterPriority, OnlineStateChange, SingleFilter, StateChange};
        use super::*;
        use crate::parse_test;
        parse_test!(empty(<EmailNotificationProvider as Component>::Config): toml::Table::new() => error);
        parse_test!(minimal(<EmailNotificationProvider as Component>::Config): toml!{
            [notify]
            address = "noreply@example.com"
            password = "Password123"
            server = "example.com"
            notify = []
        } => Notification::new(Config {
            address: "noreply@example.com".to_string(),
            password: "Password123".to_string(),
            server: "example.com".to_string(),
            name: "No Reply".to_string(),
            subscribers: vec![],
            filter: Filter::default(),
        }));
        parse_test!(maximal(<EmailNotificationProvider as Component>::Config): toml!{
            [notify]
            address = "noreply@example.com"
            password = "Password123"
            server = "example.com"
            name = "foo"
            subscribers = [
                "foo@example.com",
                { to = "bar@example.com" },
                { mail="test@example.com", filter.elements.deny = ["bar"] },
            ]
            filter.state.mode = "explicit-whitelist"
            filter.state.allow = [ { online = "down" } ]
        } => Notification::new(Config {
            address: "noreply@example.com".to_string(),
            password: "Password123".to_string(),
            server: "example.com".to_string(),
            name: "foo".to_string(),
            subscribers: vec![
                Subscriber::Default("foo@example.com".to_string()),
                Subscriber::Custom { email: "bar@example.com".to_string(), filter: Filter::default() },
                Subscriber::Custom {
                    email: "test@example.com".to_string(),
                    filter: Filter {
                        component: Default::default(),
                        entity: SingleFilter {
                            whitelist: vec![],
                            blacklist: vec!["bar".to_string()],
                            priority: FilterPriority::Whitelist,
                        },
                        state_changes: Default::default(),
                    }
                }
            ],
            filter: Filter {
                component: SingleFilter::default(),
                entity: SingleFilter::default(),
                state_changes: SingleFilter {
                    whitelist: vec![StateChange::OnlineStateChange(OnlineStateChange::Offline)],
                    blacklist: vec![],
                    priority: FilterPriority::Blacklist
                }
            }
        }));
    }
    mod behavior {
        use crate::filters::{FilterPriority, OnlineStateChange, SingleFilter, StateChange};
        use super::*;
        #[test]
        fn get_email() {
            let subscriber = Subscriber::Default("mail@example.com".to_string());
            assert_eq!(subscriber.get_email(), "mail@example.com");
            assert_eq!(Subscriber::Custom {
                email: "test@example.com".to_string(),
                filter: Filter::default(),
            }.get_email(), "test@example.com");
        }
        #[test]
        fn allows() {
            let filter = Filter {
                component: Default::default(),
                entity: Default::default(),
                state_changes: SingleFilter {
                    whitelist: vec![StateChange::OnlineStateChange(OnlineStateChange::Offline)],
                    blacklist: vec![],
                    priority: FilterPriority::Blacklist,
                },
            };
            let default_subscriber = Subscriber::Default("test@example.com".to_string());
            let custom_subscriber = Subscriber::Custom {
                email: "test@foobar.com".to_string(),
                filter,
            };
            let test_notif1 = server::Notification {
                component_id: "asdf".to_string(),
                element_id: "asdf".to_string(),
                reason: NotificationReason::NewElement(true),
            };
            let test_notif2 = server::Notification {
                component_id: "asdf".to_string(),
                element_id: "asdf".to_string(),
                reason: NotificationReason::OnlineStatusChanged(false),
            };
            assert!(default_subscriber.allows(&test_notif1));
            assert!(default_subscriber.allows(&test_notif2));
            assert!(!custom_subscriber.allows(&test_notif1));
            assert!(custom_subscriber.allows(&test_notif2));
        }
    }
}
