use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
/// Configuration for the server.
pub struct Config {
    #[serde(flatten)]
    /// Map of all the configs for the different [`crate::Component`]s.
    pub configs: HashMap<String, toml::Value>,

    /// The global configuration for the server.
    ///
    /// This its own subsection to make sure it doesn't interfere with other components.
    #[serde(default)]
    pub global: Global,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Global {
    #[serde(alias="ignore", alias="disabled", alias="disable")]
    #[serde(default)]
    /// The things that the server ignores completely.
    pub ignored: Ignored
}
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
/// Ignored values 
pub struct Ignored {
    #[serde(default)]
    /// ignores all components with the given id.
    pub components: HashSet<String>,
}

#[cfg(test)]
mod test {
    use super::*;
    use serde::Deserialize;
    use toml::toml;
    macro_rules! test {
        ($(#[$meta:meta])* $name:ident: {$src:expr} => Config {
            configs: {
                $($conf_name:ident: $conf_value:expr),* $(,)?
            } $(,)?
            global: {
                ignored: {
                    components: [$($items:expr),* $(,)?]
                }
            }
        }) => {
            $(#[$meta])*
            #[test]
            fn $name() {
                assert_eq!(
                    Config::deserialize($src).expect("failed to deserialize config"),
                    Config {
                        configs: HashMap::from([$((stringify!($conf_name).to_string(), toml::Value::Table($conf_value))),*]),
                        global: Global {
                            ignored: Ignored {
                                components: HashSet::from([$($items.to_string()),*])
                            }
                        }
                    }
                )
            }
        };
    }
    test!(empty_config: {toml::Table::new()} =>
        Config {
            configs: {},
            global: {
                ignored: {
                    components: []
                }
            }
        }
    );
    test!(ignored_parsing: {toml!{
        [global]
        ignored.components = ["mail", "miner"]
    }} => Config {
        configs: {},
        global: {
            ignored: {
                components: ["mail", "miner"]
            }
        }
    });
    test!(configs_parsing: {toml!{
       [mail]
        test = 123
    }} => Config {
        configs: {
            mail: toml!{ test = 123 },
        },
        global: {
            ignored: {
                components: []
            }
        }
    });
    test!(combined_parsing: {toml!{
        [mail.status]
        test = 123
        [global]
        ignored.components = ["mail", "miner", "ntfy"]
    }} => Config {
        configs: {
            mail: toml!(status.test = 123)
        },
        global: {
            ignored: {
                components: ["mail", "miner", "ntfy"]
            }
        }
    });
}
