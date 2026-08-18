use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
/// Configuration for the server.
pub struct Config {
    #[serde(flatten)]
    /// Map of all the configs for the different [`crate::Component`]s.
    pub configs: HashMap<String, toml::Value>,
    
    /// The global configuration for the server.
    /// 
    /// This its own subsection to make sure it doesn't interfere with other components.
    pub global: Global,
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct Global {
    #[serde(alias="ignore", alias="disabled", alias="disable")]
    #[serde(default)]
    /// The things that the server ignores completely.
    pub ignored: Ignored
}
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
/// Ignored values 
pub struct Ignored {
    #[serde(default)]
    /// ignores all components with the given id.
    pub components: HashSet<String>,

}