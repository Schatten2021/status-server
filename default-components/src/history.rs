mod backends;

use backends::Backend;
use std::error::Error;



macro_rules! history_def {
    ($(#[$struct_meta:meta])*
    struct History {
        $(
        $(#[$field_meta:meta])*
        if $feature:literal($conf_ident:ident): $backend_name:ident: $backend_ty:ident
        ),* $(,)?
    }) => {
        #[derive(Debug, thiserror::Error)]
        pub enum ConfigError {
            $(
            #[cfg(feature=$feature)]
            #[error("{0}")]
            $backend_ty(#[from] <backends::$backend_ty as backends::Backend>::ConfigError),
            )*
        }

        #[derive(Clone, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
        pub struct Config {
            #[serde(alias="elements", alias="element", alias="element_filter", alias="filter_elements", alias="filtered_elements")]
            #[serde(default)]
            elements_filter: $crate::filters::SingleFilter<String>,

            #[serde(alias="attributes", alias="attribute", alias="attribute_filter", alias="filter_attributes", alias="filtered_attributes")]
            #[serde(default)]
            attributes_filter: $crate::filters::SingleFilter<$crate::filters::AttributeIdMatcher>,

            $(
            #[cfg(feature = $feature)]
            #[serde(default)]
            $conf_ident: <backends::$backend_ty as Backend>::Config,
            )*
        }
        $(#[$struct_meta])*
        pub struct History {
            elements_filter: $crate::filters::SingleFilter<String>,
            attributes_filter: $crate::filters::SingleFilter<$crate::filters::AttributeIdMatcher>,
            $(
            #[cfg(feature = $feature)]
            $(#[$field_meta])*
            $backend_name: backends::$backend_ty,
            )*
        }
        impl ::server::Component for History {
            const ID: &'static str = "history";
            type Config = Config;
            type ConfigError = ConfigError;
            fn init(server: ::server::ComponentHandle, config: Self::Config) -> Result<Self, Self::ConfigError> {
                trace!("initializing history with config: {config:?}");
                Ok(Self {
                    elements_filter: config.elements_filter,
                    attributes_filter: config.attributes_filter,
                $(
                   #[cfg(feature=$feature)]
                    $backend_name: <backends::$backend_ty as Backend>::init(server.clone(), config.$conf_ident)?,
                )*})
            }
            fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::ConfigError> {
                trace!("reconfiguring history with config: {config:?}");
                self.elements_filter = config.elements_filter;
                self.attributes_filter = config.attributes_filter;
                $(
                #[cfg(feature=$feature)]
                <backends::$backend_ty as Backend>::reconfigure(&mut self.$backend_name, config.$conf_ident)?;
                )*
                Ok(())
            }
        }
        impl ::server::NotificationProvider for History {
            fn notify(&self, notification: ::server::Notification) {
                let now = chrono::Utc::now();
                if !self.elements_filter.allows(&notification.element_id) { return; }
                match notification.reason {
                    ::server::NotificationReason::AttributeEdit(edit) => match edit.change {
                        ::server::AttributeValueChange::Create(new) |
                        ::server::AttributeValueChange::Edit(_, new) => {
                            if !self.attributes_filter.allows(&edit.id) { return; }
                            ::tracing::trace!("received attribute value change");
                        $(
                           #[cfg(feature=$feature)]
                            if let Err(e) = <backends::$backend_ty as Backend>::add_attribute_change(&self.$backend_name, &notification.element_id, &edit.id, now, Some(&new)) {
                               ::tracing::error!("Error adding attribute change to history backend `{}`: {e}", stringify!($conf_ident))
                           }
                        )*},
                        ::server::AttributeValueChange::Delete(_) => {
                            if !self.attributes_filter.allows(&edit.id) { return; }
                            ::tracing::trace!("received attribute value deletion");
                        $(
                            #[cfg(feature=$feature)]
                            if let Err(e) = <backends::$backend_ty as Backend>::add_attribute_change(&self.$backend_name, &notification.element_id, &edit.id, now, None) {
                                ::tracing::error!("Error adding attribute deletion to history backend `{}`: {e}", stringify!($conf_ident))
                            }
                        )*},
                    },
                    ::server::NotificationReason::OnlineStatusChanged(new) |
                    ::server::NotificationReason::NewElement(new) => {
                        ::tracing::trace!("received online state change");
                    $(
                        #[cfg(feature=$feature)]
                        if let Err(e) = <backends::$backend_ty as Backend>::add_online_state_change(&self.$backend_name, &notification.element_id, now, new) {
                            ::tracing::error!("Error adding online state change to history backend `{}`: {e}", stringify!($conf_ident))
                        }
                    )*},
                }
            }
        }
        impl History {
            // TODO: merge these in the future.
            /// returns the history for a given attribute of the given element.
            #[allow(unreachable_code, reason="this is generated by a macro; unreachable code is ok, because it makes the macro easier.")]
            pub fn get_attribute_history(&self, element: &str, attribute: &str) -> Result<PropertyHistory, Box<dyn Error>> {
                $(#[cfg(feature=$feature)] return <backends::$backend_ty as backends::Backend>::get_attribute_history(&self.$backend_name, element, attribute);)*
                unreachable!("you must select at least 1 history backend");
            }
            /// returns the online state history for a given element.
            #[allow(unreachable_code, reason="this is generated by a macro; unreachable code is ok, because it makes the macro easier.")]
            pub fn get_online_state_history(&self, element: &str) -> Result<OnlineStateHistory, Box<dyn Error>> {
                $(#[cfg(feature=$feature)] return <backends::$backend_ty as backends::Backend>::get_online_state_history(&self.$backend_name, element);)*
                unreachable!("you must select at least 1 history backend");
            }
        }
        #[cfg(not(any($(feature=$feature),*)))]
        compile_error!("must select a history backend from the features!");
    };
}

/// The history of a property
pub type PropertyHistory = Vec<(chrono::DateTime<chrono::Utc>, Option<server::AttributeValue>)>;
/// The history of an online state
pub type OnlineStateHistory = Vec<(chrono::DateTime<chrono::Utc>, bool)>;


history_def!(
    /// Component providing a historization of elements.
    ///
    /// # Note
    /// The `primary_backend` config key only influences which backend is picked for retrieving any
    /// history, **NOT** which one is used to *store* the history! History is stored in all enabled
    /// backends.
    struct History {
        if "history-sqlite-backend"(sqlite): sqlite: SqliteBackend,
        if "history-fs-json-backend"(fs_json): fs_json_backend: FsJsonBackend,
    }
);

#[cfg(test)]
mod test {
    #![allow(clippy::needless_update, reason="the `..Default::default()` is used for when a backend is selected")]
    use crate::filters::{AttributeIdMatcher, FilterPriority, SingleFilter};
    use super::Config;
    use super::*;
    use crate::parse_test;
    parse_test!(empty(Config): toml::Table::new() => Config {
        elements_filter: SingleFilter::default(),
        attributes_filter: SingleFilter::default(),
        #[cfg(feature="history-sqlite-backend")]
        sqlite: backends::sqlite::Config::default(),
        ..Default::default()
    });
    parse_test!(elements(Config): toml!{
        elements.allow = ["foo"]
    } => Config {
        elements_filter: SingleFilter {
            whitelist: vec!["foo".to_string()],
            blacklist: vec![],
            priority: FilterPriority::Whitelist,
        },
        attributes_filter: SingleFilter::default(),
        ..Default::default()
    });
    parse_test!(attributes(Config): toml!{
        attributes.deny = [{ id = "foo" }]
    } => Config {
        elements_filter: SingleFilter::default(),
        attributes_filter: SingleFilter {
            whitelist: vec![],
            blacklist: vec![AttributeIdMatcher {
                id: "foo".to_string(),
                exact: true,
            }],
            priority: FilterPriority::Whitelist,
        },
        ..Default::default()
    });
    parse_test!(all(Config): toml!{
        elements.allow = ["foo"]
        attributes.deny = [{ id = "foo" }]
    } => Config {
        elements_filter: SingleFilter {
            whitelist: vec!["foo".to_string()],
            blacklist: vec![],
            priority: FilterPriority::Whitelist,
        },
        attributes_filter: SingleFilter {
            whitelist: vec![],
            blacklist: vec![AttributeIdMatcher {
                id: "foo".to_string(),
                exact: true,
            }],
            priority: FilterPriority::Whitelist,
        },
        ..Default::default()
    });
}