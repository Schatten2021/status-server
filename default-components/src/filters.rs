//! utilities to enable easier filtering of messages for [`server::StatusProvider`].

use server::{AttributeValueChange, Notification, NotificationReason};
use std::hash::Hash;

const fn always() -> bool { true }

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
/// A fully configurable filter.
///
/// Allows filtering for different components, entities & messages, down to the id of a changed attribute.
pub struct Filter {
    #[serde(alias="components", alias="component-id", alias="component_id", alias="source", alias="source-id", alias="source_id")]
    #[serde(default)]
    /// [`SingleFilter`] filtering the id of the component that caused the [`Notification`].
    pub component: SingleFilter<String>,

    #[serde(default)]
    #[serde(alias="entities", alias="entity-id", alias="entity_id",
        alias="element", alias="elements", alias="element-id", alias="element_id")]
    /// [`SingleFilter`] filtering the id of the target entity of the [`Notification`].
    pub entity: SingleFilter<String>,

    #[serde(alias="state-change",
        alias="state", alias="states",
        alias="status", alias="statuses", alias="stati",
        alias="change", alias="changes")]
    #[serde(default)]
    /// [`SingleFilter`] filtering the state changes.
    pub state_changes: SingleFilter<StateChange>,
}
impl Filter {
    /// whether the filter allows the given message.
    #[must_use]
    pub fn allows(&self, message: &Notification) -> bool {
        self.component.allows(&message.component_id) &&
            self.entity.allows(&message.element_id) &&
            self.state_changes.allows(&message.reason)
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
/// Filter filtering a single type of item.
pub struct SingleFilter<ItemType> {
    #[serde(alias="allow", alias="allowed", alias="enable", alias="enabled", alias="whitelisted",
        alias="accept", alias="accepts", alias="accepted")]
    #[serde(default="Vec::new")]
    /// Whitelist of things to specifically allow.
    pub whitelist: Vec<ItemType>,
    #[serde(alias="deny", alias="denied", alias="denies", alias="disable", alias="disabled", alias="blacklisted",
        alias="disallow", alias="disallowed", alias="disallows")]
    #[serde(default="Vec::new")]
    /// Blacklist of things to block.
    pub blacklist: Vec<ItemType>,

    /// Whether to accept values per default or to reject them.
    ///
    /// Changes the behavior of the filter.
    #[serde(default)]
    #[serde(alias="default", alias="mode")]
    pub priority: FilterPriority,
}
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
/// What of the filter to prioritize.
pub enum FilterPriority {
    /// Prioritizes the whitelist.
    ///
    /// This makes it so that the [`SingleFilter`] accepts values by default and only rejects values
    /// if they are in the blacklist.
    #[serde(alias="allow", alias="accept", alias="explicit-blacklist", alias="explicit_blacklist")]
    #[default]
    Whitelist,
    /// Prioritizes the blacklist
    ///
    /// This makes it so that the [`SingleFilter`] accepts values by default and only accepts values
    /// if they are in the whitelist.
    #[serde(alias="disallow", alias="deny", alias="explicit-whitelist", alias="explicit_whitelist")]
    Blacklist
}
impl<T> Default for SingleFilter<T> {
    fn default() -> Self {
        Self {
            whitelist: Vec::new(),
            blacklist: Vec::new(),
            priority: FilterPriority::Whitelist,
        }
    }
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all="snake_case")]
/// Identifies a change in state for filtering.
pub enum StateChange {
    #[serde(alias="create-entity", alias="create")]
    /// Matches the creation of an entity (no matter the new state. Use [`Self::OnlineStateChange`] for that.)
    CreateEntity,

    #[serde(alias="attribute")]
    /// Matches changes to the attributes of an element. See [`AttributeChange`] for more infos.
    AttributeChange(AttributeChange),

    #[serde(alias="online", alias="online-state", alias="online_state")]
    /// matches changes to the online state.
    OnlineStateChange(OnlineStateChange)
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all="snake_case")]
/// changes to the online state.
pub enum OnlineStateChange {
    /// matches any online state change
    Any,

    /// when the server went online
    #[serde(alias="up")]
    Online,

    /// when the server went offline
    #[serde(alias="down")]
    Offline,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
/// Matches when attributes of an element change.
pub struct AttributeChange {
    /// The ID of the attribute to match.
    #[serde(flatten)]
    pub id: Option<AttributeIdMatcher>,

    #[serde(default)]
    /// The actual element being matched.
    pub event: AttributeEvent,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, Default)]
/// Matches an AttributeId.
pub struct AttributeIdMatcher {
    /// The id of the attribute.
    pub id: String,

    #[serde(default="always")]
    /// whether to match the id exactly (no children)
    pub exact: bool,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all="snake_case")]
/// Events that can happen to an attribute.
pub enum AttributeEvent {
    #[default]
    /// Match any change to the attribute.
    Any,
    /// Match the creation of a new attribute.
    Create,
    /// Match the change of an attribute.
    Change,
    /// Match the deletion of an attribute.
    Delete,
}
impl<Item> SingleFilter<Item> {
    /// checks whether the filter allows the given input.
    pub fn allows<V>(&self, input: &V) -> bool
    where Item: Filtering<V> {
        match self.priority {
            FilterPriority::Whitelist => self.whitelisted(input) || !self.blacklisted(input),
            FilterPriority::Blacklist => self.whitelisted(input) && !self.blacklisted(input)
        }
    }
    /// checks whether the input is whitelisted
    pub fn whitelisted<V>(&self, input: &V) -> bool
    where Item: Filtering<V> {
        self.whitelist.iter().any(|f| f.matches(input))
    }
    /// checks whether the input is blacklisted
    pub fn blacklisted<V>(&self, input: &V) -> bool
    where Item: Filtering<V> {
        self.blacklist.iter().any(|f| f.matches(input))
    }
    
}
/// Helper trait for usage with [`SingleFilter`].
///
/// Implemented by default for types that implement [`Eq`].
pub trait Filtering<Input> {
    /// Check whether the value matches given the configuration of `self`.
    fn matches(&self, value: &Input) -> bool;
}
impl<T: Eq> Filtering<T> for T {
    fn matches(&self, value: &T) -> bool {
        self == value
    }
}
impl Filtering<bool> for OnlineStateChange {
    fn matches(&self, value: &bool) -> bool {
        matches!((self, value),
            (Self::Any, _) |
            (Self::Online, true) |
            (Self::Offline, false)
        )
    }
}
impl Filtering<NotificationReason> for StateChange {
    fn matches(&self, reason: &NotificationReason) -> bool {
        match self {
            Self::OnlineStateChange(filter)=> matches!(reason,
                NotificationReason::OnlineStatusChanged(status) |
                NotificationReason::NewElement(status)
                if filter.matches(status)),
            Self::CreateEntity => matches!(reason, NotificationReason::NewElement(_)),
            Self::AttributeChange(change) => {
                let NotificationReason::AttributeEdit(edit) = reason else { return false; };
                change.id.as_ref().is_none_or(|v| v.matches(&edit.id)) &&
                    change.event.matches(&edit.change)
            }
        }
    }
}
impl Filtering<String> for AttributeIdMatcher {
    fn matches(&self, value: &String) -> bool {
        if self.exact {
            &self.id == value
        } else {
            value.starts_with(&self.id) && (value.len() == self.id.len() || value[self.id.len()..].starts_with('.'))
        }
    }
}
impl Filtering<AttributeValueChange> for AttributeEvent {
    fn matches(&self, value: &AttributeValueChange) -> bool {
        matches!((self, value),
            (Self::Any, _) |
            (Self::Create, AttributeValueChange::Create(_)) |
            (Self::Change, AttributeValueChange::Edit(_, _)) |
            (Self::Delete, AttributeValueChange::Delete(_))
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::parse_test;
    use toml::toml;
    macro_rules! behavior_test {
        ($(#[$meta:meta])* $test_name:ident($test_type:ty: $conf:expr)::$func:ident: $($target_result:ident $check:expr),* $(,)?) => {
            $(#[$meta])*
            #[test]
            fn $test_name() {
                let item = <$test_type as ::serde::Deserialize>::deserialize($conf).expect(concat!("unable to load config `", stringify!($conf), "`"));
                let mut success = true;
                $(success &= {
                    const SHOULD_MATCH: bool = behavior_test!(@internal:target_result $target_result);
                    if item.$func(&$check) == SHOULD_MATCH {
                        true
                    } else {
                        if !SHOULD_MATCH {
                            eprintln!("\x1b[1;31m[UNWANTED BEHAVIOR]\x1b[0m matched `{}` to config `{}` despite expecting it not to.\x1b[0m", stringify!($check), stringify!($conf));
                        } else {
                            eprintln!("\x1b[1;31m[UNWANTED BEHAVIOR]\x1b[0m didn't match `{}` for config `{}` despite expecting it to.\x1b[0m", stringify!($check), stringify!($conf));
                        }
                        false
                    }
                };)*
                if !success {
                    panic!("behavior test failed.");
                }
            }
        };
        (@internal:target_result success)  => {true};
        (@internal:target_result allow)  => {true};
        (@internal:target_result allows)  => {true};
        (@internal:target_result accept)  => {true};
        (@internal:target_result accepts)  => {true};
        (@internal:target_result matches)  => {true};

        (@internal:target_result failure)  => {false};
        (@internal:target_result deny)  => {false};
        (@internal:target_result denies)  => {false};

    }
    mod filter {
        use super::*;
        mod parse {
            use super::*;
            parse_test!(empty(Filter): toml::Table::new() => Filter {
                component: SingleFilter::default(),
                entity: SingleFilter::default(),
                state_changes: SingleFilter::default(),
            });
            parse_test!(component(Filter): toml!{
                component.allow = ["foo"]
                component.deny = ["bar"]
                component.default = "deny"
            } => Filter {
                component: SingleFilter {
                    whitelist: vec!["foo".to_string()],
                    blacklist: vec!["bar".to_string()],
                    priority: FilterPriority::Blacklist
                },
                entity: SingleFilter::default(),
                state_changes: SingleFilter::default(),
            });
            parse_test!(entity(Filter): toml!{
                entity.allow = ["foo"]
                entity.deny = ["bar"]
                entity.default = "deny"
            } => Filter {
                component: SingleFilter::default(),
                entity: SingleFilter {
                    whitelist: vec!["foo".to_string()],
                    blacklist: vec!["bar".to_string()],
                    priority: FilterPriority::Blacklist
                },
                state_changes: SingleFilter::default(),
            });
            parse_test!(state_change(Filter): toml!{
                state.allow = ["create", { attribute.event = "any" }]
            } => Filter {
                component: SingleFilter::default(),
                entity: SingleFilter::default(),
                state_changes: SingleFilter {
                    whitelist: vec![StateChange::CreateEntity, StateChange::AttributeChange(AttributeChange {
                        id: None,
                        event: AttributeEvent::Any
                    })],
                    blacklist: vec![],
                    priority: FilterPriority::Whitelist,
                }
            });
            parse_test!(mixed(Filter): toml!{
                component.allow = ["foo"]
                entity.deny = ["bar"]
                state.default = "deny"
            } => Filter {
                component: SingleFilter {
                    whitelist: vec!["foo".to_string()],
                    blacklist: vec![],
                    priority: FilterPriority::Whitelist,
                },
                entity: SingleFilter {
                    whitelist: vec![],
                    blacklist: vec!["bar".to_string()],
                    priority: FilterPriority::Whitelist,
                },
                state_changes: SingleFilter {
                    whitelist: vec![],
                    blacklist: vec![],
                    priority: FilterPriority::Blacklist,
                }
            });
        }
        mod behavior {
            use super::*;
            use server::{AttributeEdit, AttributeValue, AttributeValueChange, Notification, NotificationReason};
            behavior_test!(test1(Filter: toml!{
                component.allow = ["foo"]
                component.deny = ["bar", "foo"]
                entity.allow = ["foo", "bar"]
                entity.deny = ["bar"]
                entity.default = "deny"
                state.allow = [
                    "create",
                    { attribute.id = "foo", attribute.exact = true },
                    { attribute.id = "bar", attribute.exact = false },
                ]
                state.deny = [ { online = "up" }]
                state.mode = "explicit-whitelist"
            })::allows:
                denies Notification {
                    component_id: "foo".to_string(),
                    element_id: "foo".to_string(),
                    reason: NotificationReason::NewElement(true), // state.deny.online also denies new creations.
                },
                allows Notification {
                    component_id: "foo".to_string(),
                    element_id: "foo".to_string(),
                    reason: NotificationReason::NewElement(false),
                },
                denies Notification {
                    component_id: "bar".to_string(),
                    element_id: "foo".to_string(),
                    reason: NotificationReason::NewElement(true),
                },
                denies Notification {
                    component_id: "foo".to_string(),
                    element_id: "bar".to_string(),
                    reason: NotificationReason::NewElement(true),
                },
                denies Notification {
                    component_id: "smth".to_string(),
                    element_id: "foo".to_string(),
                    reason: NotificationReason::NewElement(true),
                },
                denies Notification {
                    component_id: "foo".to_string(),
                    element_id: "foo".to_string(),
                    reason: NotificationReason::OnlineStatusChanged(true),
                },
                denies Notification {
                    component_id: "foo".to_string(),
                    element_id: "foo".to_string(),
                    reason: NotificationReason::OnlineStatusChanged(false),
                },
                allows Notification {
                    component_id: "foo".to_string(),
                    element_id: "foo".to_string(),
                    reason: NotificationReason::AttributeEdit(AttributeEdit {
                        id: "foo".to_string(),
                        change: AttributeValueChange::Create(AttributeValue::Marker)
                    }),
                },
                allows Notification {
                    component_id: "foo".to_string(),
                    element_id: "foo".to_string(),
                    reason: NotificationReason::AttributeEdit(AttributeEdit {
                        id: "foo".to_string(),
                        change: AttributeValueChange::Edit(AttributeValue::Marker, AttributeValue::Marker)
                    }),
                },
                allows Notification {
                    component_id: "foo".to_string(),
                    element_id: "foo".to_string(),
                    reason: NotificationReason::AttributeEdit(AttributeEdit {
                        id: "foo".to_string(),
                        change: AttributeValueChange::Delete(AttributeValue::Marker)
                    }),
                },
                denies Notification {
                    component_id: "foo".to_string(),
                    element_id: "foo".to_string(),
                    reason: NotificationReason::AttributeEdit(AttributeEdit {
                        id: "foo.bar".to_string(),
                        change: AttributeValueChange::Create(AttributeValue::Marker)
                    }),
                },
                allows Notification {
                    component_id: "foo".to_string(),
                    element_id: "foo".to_string(),
                    reason: NotificationReason::AttributeEdit(AttributeEdit {
                        id: "bar".to_string(),
                        change: AttributeValueChange::Create(AttributeValue::Marker)
                    }),
                },
                allows Notification {
                    component_id: "foo".to_string(),
                    element_id: "foo".to_string(),
                    reason: NotificationReason::AttributeEdit(AttributeEdit {
                        id: "bar.foo".to_string(),
                        change: AttributeValueChange::Create(AttributeValue::Marker)
                    }),
                },
            );
        }
    }
    mod single_filter {
        use super::*;
        mod parse {
            use super::*;
            type Filter = SingleFilter<String>;
            parse_test!(empty(Filter): toml::Table::new() => Filter {
                whitelist: vec![],
                blacklist: vec![],
                priority: FilterPriority::Whitelist
            });
            parse_test!(whitelist(Filter): toml!(allow = ["foo", "bar"]) => Filter {
                whitelist: vec!["foo".to_string(), "bar".to_string()],
                blacklist: vec![],
                priority: FilterPriority::Whitelist,
            });
            parse_test!(blacklist(Filter): toml!(deny = ["foo", "bar"]) => Filter {
                whitelist: vec![],
                blacklist: vec!["foo".to_string(), "bar".to_string()],
                priority: FilterPriority::Whitelist,
            });
            parse_test!(default_deny(Filter): toml!(default = "deny") => Filter {
                whitelist: vec![],
                blacklist: vec![],
                priority: FilterPriority::Blacklist,
            });
            parse_test!(mode_explicit_whitelist(Filter): toml!(mode = "explicit-whitelist") => Filter {
                whitelist: vec![],
                blacklist: vec![],
                priority: FilterPriority::Blacklist,
            });
            parse_test!(combined(Filter): toml!{
                enable = ["foo"]
                disable = ["bar"]
                default = "deny"
            } => Filter {
                whitelist: vec!["foo".to_string()],
                blacklist: vec!["bar".to_string()],
                priority: FilterPriority::Blacklist,
            });
        }
        mod behavior {
            use super::*;
            type Filter = SingleFilter<String>;
            behavior_test!(default(Filter: toml!(deny = []))::allows:
                matches String::new(),
                matches "foo".to_string(),
            );
            behavior_test!(whitelist(Filter: toml!{
                allow = ["foo.bar"]
                deny = ["foo", "foo.bar"]
                default = "allow"
            })::allows:
                matches "bar".to_string(), // check default
                denies "foo".to_string(), // check overwrites
                matches "foo.bar".to_string(), // check prioritizing of whitelist
            );
            behavior_test!(blacklist(Filter: toml!{
                allow = ["foo", "foo.bar"]
                deny = ["foo.bar"]
                default = "deny"
            })::allows:
                denies "bar".to_string(), // check default
                allows "foo".to_string(), // check overwrites
                denies "foo.bar".to_string(), // check prioritizing of blacklist
            );
        }
    }
    mod state_change {
        use super::*;
        use server::{AttributeValue, NotificationReason};
        mod parse {
            use super::*;
            parse_test!(empty(StateChange): toml::Table::new() => error);
            parse_test!(create(StateChange): toml::Value::String("create".to_string()) => StateChange::CreateEntity);
            parse_test!(attribute_change(StateChange): toml!{
                attribute.id = "foo.bar"
                attribute.event = "create"
            } => StateChange::AttributeChange(AttributeChange {
                id: Some(AttributeIdMatcher {
                    id: "foo.bar".to_string(),
                    exact: true,
                }),
                event: AttributeEvent::Create
            }));
            parse_test!(online_state_change(StateChange): toml!{online = "any"} => StateChange::OnlineStateChange(OnlineStateChange::Any));
            parse_test!(online_state_change_up(StateChange): toml!{online = "up"} => StateChange::OnlineStateChange(OnlineStateChange::Online));
            parse_test!(online_state_change_down(StateChange): toml!{online = "down"} => StateChange::OnlineStateChange(OnlineStateChange::Offline));
            parse_test!(multiple(StateChange): toml!{
                online = "any"
                attribute.event = "any"
            } => error);
        }
        mod behavior {
            use super::*;
            use server::{AttributeEdit, AttributeValueChange};
            behavior_test! {create(StateChange: toml::Value::String("create".to_string()))::matches:
                allows NotificationReason::NewElement(true),
                allows NotificationReason::NewElement(false),
                denies NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Create(AttributeValue::Marker),
                }),
                denies NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Edit(AttributeValue::Marker, AttributeValue::Marker),
                }),
                denies NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Delete(AttributeValue::Marker)
                }),
                denies NotificationReason::OnlineStatusChanged(true),
                denies NotificationReason::OnlineStatusChanged(false),
            }
            behavior_test!{attribute_create(StateChange: toml!{
                attribute.event = "create"
            })::matches:
                denies NotificationReason::NewElement(true),
                denies NotificationReason::NewElement(false),
                allows NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Create(AttributeValue::Marker),
                }),
                denies NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Edit(AttributeValue::Marker, AttributeValue::Marker),
                }),
                denies NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Delete(AttributeValue::Marker)
                }),
                denies NotificationReason::OnlineStatusChanged(true),
                denies NotificationReason::OnlineStatusChanged(false),
            }
            behavior_test!{attribute_change(StateChange: toml!{
                attribute.event = "change"
            })::matches:
                denies NotificationReason::NewElement(true),
                denies NotificationReason::NewElement(false),
                denies NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Create(AttributeValue::Marker),
                }),
                allows NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Edit(AttributeValue::Marker, AttributeValue::Marker),
                }),
                denies NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Delete(AttributeValue::Marker)
                }),
                denies NotificationReason::OnlineStatusChanged(true),
                denies NotificationReason::OnlineStatusChanged(false),
            }
            behavior_test!{attribute_delete(StateChange: toml!{
                attribute.event = "delete"
            })::matches:
                denies NotificationReason::NewElement(true),
                denies NotificationReason::NewElement(false),
                denies NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Create(AttributeValue::Marker),
                }),
                denies NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Edit(AttributeValue::Marker, AttributeValue::Marker),
                }),
                allows NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Delete(AttributeValue::Marker)
                }),
                denies NotificationReason::OnlineStatusChanged(true),
                denies NotificationReason::OnlineStatusChanged(false),
            }
            behavior_test!{attribute_edit(StateChange: toml!{
                attribute.event = "any"
            })::matches:
                denies NotificationReason::NewElement(true),
                denies NotificationReason::NewElement(false),
                allows NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Create(AttributeValue::Marker),
                }),
                allows NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Edit(AttributeValue::Marker, AttributeValue::Marker),
                }),
                allows NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Delete(AttributeValue::Marker)
                }),
                denies NotificationReason::OnlineStatusChanged(true),
                denies NotificationReason::OnlineStatusChanged(false),
            }
            behavior_test!{online_status_change(StateChange: toml!{
                online = "any"
            })::matches:
                allows NotificationReason::NewElement(true),
                allows NotificationReason::NewElement(false),
                denies NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Create(AttributeValue::Marker),
                }),
                denies NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Edit(AttributeValue::Marker, AttributeValue::Marker),
                }),
                denies NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Delete(AttributeValue::Marker)
                }),
                allows NotificationReason::OnlineStatusChanged(true),
                allows NotificationReason::OnlineStatusChanged(false),
            }
            behavior_test!{online_status_change_up(StateChange: toml!{
                online = "up"
            })::matches:
                allows NotificationReason::NewElement(true),
                denies NotificationReason::NewElement(false),
                denies NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Create(AttributeValue::Marker),
                }),
                denies NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Edit(AttributeValue::Marker, AttributeValue::Marker),
                }),
                denies NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Delete(AttributeValue::Marker)
                }),
                allows NotificationReason::OnlineStatusChanged(true),
                denies NotificationReason::OnlineStatusChanged(false),
            }
            behavior_test!{online_status_change_down(StateChange: toml!{
                online = "down"
            })::matches:
                denies NotificationReason::NewElement(true),
                allows NotificationReason::NewElement(false),
                denies NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Create(AttributeValue::Marker),
                }),
                denies NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Edit(AttributeValue::Marker, AttributeValue::Marker),
                }),
                denies NotificationReason::AttributeEdit(AttributeEdit {
                    id: "foo".to_string(),
                    change: AttributeValueChange::Delete(AttributeValue::Marker)
                }),
                denies NotificationReason::OnlineStatusChanged(true),
                allows NotificationReason::OnlineStatusChanged(false),
            }
        }
    }
    mod online_state_change {
        use super::*;
        // no parsing, because that's trivial.
        mod behavior {
            use super::*;
            behavior_test!(online(OnlineStateChange: toml::Value::String("up".to_string()))::matches:
                allows true,
                denies false,
            );
            behavior_test!(offline(OnlineStateChange: toml::Value::String("down".to_string()))::matches:
                denies true,
                allows false,
            );
            behavior_test!(any(OnlineStateChange: toml::Value::String("any".to_string()))::matches:
                allows true,
                allows false,
            );
        }
    }
    mod attribute_change {
        use super::*;
        mod parse {
            use super::*;
            parse_test!(all_present(AttributeChange): toml!{
                id = "foo"
                exact = false
                event = "create"
            } => AttributeChange {
                id: Some(AttributeIdMatcher {
                    id: "foo".to_string(),
                    exact: false,
                }),
                event: AttributeEvent::Create,
            });
            parse_test!(empty(AttributeChange): toml::Table::new() => AttributeChange {
                id: None,
                event: AttributeEvent::Any,
            });
            parse_test!(id_only(AttributeChange): toml!(id="foo") => AttributeChange {
                id: Some(AttributeIdMatcher {
                    id: "foo".to_string(),
                    exact: true,
                }),
                event: AttributeEvent::Any,
            });
            parse_test!(id_exact(AttributeChange): toml!{
                id="foo"
                exact=false
            } => AttributeChange {
                id: Some(AttributeIdMatcher {
                    id: "foo".to_string(),
                    exact: false,
                }),
                event: AttributeEvent::Any,
            });
            parse_test!(event_only(AttributeChange): toml!(event="create") => AttributeChange {
                id: None,
                event: AttributeEvent::Create,
            });

        }
    }
    mod attribute_id_matcher {
        use super::*;
        mod parse {
            use super::*;

            parse_test!(empty(AttributeIdMatcher): toml::Table::new() => error);
            parse_test!(id(AttributeIdMatcher): toml!(id="foo") => AttributeIdMatcher {
                id: "foo".to_string(),
                exact: true,
            });
            parse_test!(exact_only(AttributeIdMatcher): toml!(exact=false) => error);
            parse_test!(exact(AttributeIdMatcher): toml!{
                id="foo"
                exact=false
            } => AttributeIdMatcher {
                id: "foo".to_string(),
                exact: false,
            });
            }
        mod behavior {
            use super::*;

            behavior_test!(exact(AttributeIdMatcher: toml!{
                id="foo"
                exact=true
            })::matches:
                allows "foo".to_string(),
                denies "foo.bar".to_string(),
                denies "bar".to_string(),
            );
            behavior_test!(fuzzy(AttributeIdMatcher: toml!{
                id="foo"
                exact=false
            })::matches:
                allows "foo".to_string(),
                allows "foo.bar".to_string(),
                denies "bar".to_string(),
            );
        }
    }
    mod attribute_event {
        use super::*;
        use server::AttributeValue;
        // No parsing, because that's trivial.
        mod behavior {
            use super::*;
            behavior_test!(create(AttributeEvent: toml::Value::String("create".to_string()))::matches:
                allows AttributeValueChange::Create(AttributeValue::Marker),
                denies AttributeValueChange::Edit(AttributeValue::Marker, AttributeValue::Marker),
                denies AttributeValueChange::Delete(AttributeValue::Marker),
            );
            behavior_test!(change(AttributeEvent: toml::Value::String("change".to_string()))::matches:
                denies AttributeValueChange::Create(AttributeValue::Marker),
                allows AttributeValueChange::Edit(AttributeValue::Marker, AttributeValue::Marker),
                denies AttributeValueChange::Delete(AttributeValue::Marker),
            );
            behavior_test!(delete(AttributeEvent: toml::Value::String("delete".to_string()))::matches:
                denies AttributeValueChange::Create(AttributeValue::Marker),
                denies AttributeValueChange::Edit(AttributeValue::Marker, AttributeValue::Marker),
                allows AttributeValueChange::Delete(AttributeValue::Marker),
            );
            behavior_test!(any(AttributeEvent: toml::Value::String("any".to_string()))::matches:
                allows AttributeValueChange::Create(AttributeValue::Marker),
                allows AttributeValueChange::Edit(AttributeValue::Marker, AttributeValue::Marker),
                allows AttributeValueChange::Delete(AttributeValue::Marker),
            );
        }
    }
}
