use std::collections::HashMap;
use bytecode::ByteCode;
use utils::Never;
use server::{AttributeValue, AttributeValueChange, ComponentHandle, Notification, NotificationReason};
use crate::filters::Filter;

fn default_message() -> String {
    "{element} {reason_long}".to_string()
}

// TODO: support actions?
#[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq)]
pub struct Config {
    base: String,
    topic: String,
    title: Option<String>,
    #[serde(default="default_message")]
    message: String,
    #[serde(default)]
    tags: Vec<String>,
    priority: Option<u8>,
    click: Option<url::Url>,
    attach: Option<url::Url>,
    markdown: Option<bool>,
    icon: Option<url::Url>,
    filename: Option<String>,
    delay: Option<String>,
    email: Option<String>,
    call: Option<String>,
    #[serde(default)]
    filter: Filter,
    auth_token: Option<String>,
}
#[derive(serde::Serialize, Clone, Debug)]
struct NotificationBody {
    topic: String,
    #[serde(skip_serializing_if="Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if="Vec::is_empty")]
    tags: Vec<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    priority: Option<u8>,
    #[serde(skip_serializing_if="Option::is_none")]
    click: Option<url::Url>,
    #[serde(skip_serializing_if="Option::is_none")]
    attach: Option<url::Url>,
    #[serde(skip_serializing_if="Option::is_none")]
    markdown: Option<bool>,
    #[serde(skip_serializing_if="Option::is_none")]
    icon: Option<url::Url>,
    #[serde(skip_serializing_if="Option::is_none")]
    filename: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    delay: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    email: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    call: Option<String>,
}
impl From<&Config> for NotificationBody {
    fn from(value: &Config) -> Self {
        Self {
            topic: value.topic.clone(),
            message: None,
            title: value.title.clone(),
            tags: value.tags.clone(),
            priority: value.priority,
            click: value.click.clone(),
            attach: value.attach.clone(),
            markdown: value.markdown,
            icon: value.icon.clone(),
            filename: value.filename.clone(),
            delay: value.delay.clone(),
            email: value.email.clone(),
            call: value.call.clone(),
        }
    }
}
/// [`NotificationProvider`] to send notifications via [NTFY](https://ntfy.sh).
pub struct NtfyNotificationProvider {
    config: Vec<Config>,
    handle: ComponentHandle,
}
impl server::Component for NtfyNotificationProvider {
    const ID: &'static str = "ntfy";
    type Config = crate::Notification<Vec<Config>>;
    type ConfigError = Never;

    fn init(handle: ComponentHandle, config: Self::Config) -> Result<Self, Self::ConfigError> {
        let config = config.notification;
        Ok(Self {
            config,
            handle,
        })
    }

    fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::ConfigError> {
        let config = config.notification;
        self.config = config;
        Ok(())
    }
}

impl server::NotificationProvider for NtfyNotificationProvider {
    fn notify(&self, notification: Notification) {
        let mut format_values = HashMap::from([
            ("component_id".to_string(), notification.component_id.clone()),
            ("element_id".to_string(), notification.element_id.clone()),
            ("element".to_string(), match self.handle.get_attribute(&notification.element_id, "name") {
                Some(AttributeValue::Custom(ByteCode::String(name))) => name,
                _ =>  notification.element_id.clone(),
            }),
            ("reason_short".to_string(), match &notification.reason {
                NotificationReason::OnlineStatusChanged(true) => "went online".to_string(),
                NotificationReason::OnlineStatusChanged(false) => "went offline".to_string(),
                NotificationReason::AttributeEdit(edit) => match &edit.change {
                    AttributeValueChange::Create(_) => format!("got attribute {}", edit.id),
                    AttributeValueChange::Edit(_, _) => format!("attribute {} changed", edit.id),
                    AttributeValueChange::Delete(_) => format!("attribute {} got deleted", edit.id),
                }
                NotificationReason::NewElement(true) => "created (online)".to_string(),
                NotificationReason::NewElement(false) => "created (offline)".to_string(),
            }),
            ("reason_long".to_string(), match &notification.reason {
                NotificationReason::OnlineStatusChanged(true) => "went online".to_string(),
                NotificationReason::OnlineStatusChanged(false) => "went offline".to_string(),
                NotificationReason::AttributeEdit(edit) => match &edit.change {
                    AttributeValueChange::Create(val) => format!("attribute {} got created ({val})", edit.id),
                    AttributeValueChange::Edit(old, new) => format!("attribute {} got changed ({old} => {new})", edit.id),
                    AttributeValueChange::Delete(old) => format!("attribute {} got deleted ({old})", edit.id),
                }
                NotificationReason::NewElement(true) => "got created and went online".to_string(),
                NotificationReason::NewElement(false) => "got created and went offline".to_string(),
            }),
        ]);
        match &notification.reason {
            NotificationReason::NewElement(true) |
            NotificationReason::OnlineStatusChanged(true) => {
                format_values.insert("status_new".to_string(), "online".to_string());
                format_values.insert("status_old".to_string(), "offline".to_string());
            }
            NotificationReason::NewElement(false) |
            NotificationReason::OnlineStatusChanged(false) => {

                format_values.insert("status_new".to_string(), "offline".to_string());
                format_values.insert("status_old".to_string(), "online".to_string());
            }
            NotificationReason::AttributeEdit(change) => {
                format_values.insert("attr_id".to_string(), change.id.clone());
                match &change.change {
                    AttributeValueChange::Create(new) => {
                        format_values.insert("attr_new_value".to_string(), new.to_string());
                    }
                    AttributeValueChange::Edit(old, new) => {
                        format_values.insert("attr_new_value".to_string(), new.to_string());
                        format_values.insert("attr_old_values".to_string(), old.to_string());
                    }
                    AttributeValueChange::Delete(old) => {
                        format_values.insert("attr_old_values".to_string(), old.to_string());
                    }
                }
            }
        }
        debug!("sending ntfy notification with format values: {:?}", format_values);
        let client = reqwest::Client::new();
        for config in &self.config {
            use strfmt::Format;
            if !config.filter.allows(&notification) {
                trace!("message filtered out through config");
                continue;
            }
            debug!("sending ntfy notification to {}", config.base);
            let title = config.title.as_ref().map(|t| {
                t.format(&format_values).unwrap_or_else(|e| {
                    error!("couldn't format message: {e}");
                    t.clone()
                })
            });
            let message = config.message.format(&format_values).unwrap_or_else(|_| config.message.clone());

            let mut body = NotificationBody::from(config);
            body.message = Some(message.clone());
            body.title = title;
            trace!("finished ntfy notification: {:?}", body);
            let mut request = client.post(&config.base)
                .json(&body);
            if let Some(token) = &config.auth_token {
                request = request.bearer_auth(token);
            }
            tokio::spawn(request.send());
        }
    }
}

#[cfg(test)]
mod test {
    use server::Component;
    use crate::filters::{FilterPriority, SingleFilter};
    use crate::parse_test;
    use super::*;

    parse_test!(empty(<NtfyNotificationProvider as Component>::Config): toml::Table::new() => error);
    parse_test!(full(<NtfyNotificationProvider as Component>::Config): toml!{
        [[notify]]
        base = "ntfy.sh"
        topic = "test"
        title = "{element_id} {reason_short}"
        message = "{element_id} {reason_long}"
        tags = ["foo"]
        priority = 1
        click = "https://example.com"
        attach = "https://example.com/file.jpg"
        markdown = true
        icon = "https://example.com/icon.png"
        filename = "file.jpg"
        delay = "30min"
        email = "foo@example.com"
        call = "+1234556789"

        filter.changes.default = "deny"
        auth_token = "tk_asdf"
    } => crate::Notification::new(vec![
        Config {
            base: "ntfy.sh".to_string(),
            topic: "test".to_string(),
            title: Some("{element_id} {reason_short}".to_string()),
            message: "{element_id} {reason_long}".to_string(),
            tags: vec!["foo".to_string()],
            priority: Some(1),
            click: Some("https://example.com".parse().unwrap()),
            attach: Some("https://example.com/file.jpg".parse().unwrap()),
            markdown: Some(true),
            icon: Some("https://example.com/icon.png".parse().unwrap()),
            filename: Some("file.jpg".to_string()),
            delay: Some("30min".to_string()),
            email: Some("foo@example.com".to_string()),
            call: Some("+1234556789".to_string()),
            filter: Filter {
                component: SingleFilter::default(),
                entity: SingleFilter::default(),
                state_changes: SingleFilter {
                    whitelist: vec![],
                    blacklist: vec![],
                    priority: FilterPriority::Blacklist,
                },
            },
            auth_token: Some("tk_asdf".to_string()),
        },
    ]));
    // TODO: add formatting test
}
