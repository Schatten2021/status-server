use bytecode::ByteCode;
use server::{AttributeValue, Component, ComponentHandle};
use std::collections::HashMap;
use utils::Never;

const NAME_ATTRIBUTE_ID: &str = "name";

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Config {
    #[serde(flatten)]
    names: HashMap<String, String>,
}

/// Very simple component allowing the configuration of names for elements.
///
/// This is literally just a component so that you can do:
/// ```toml
/// [names]
/// foo = "Foo"
/// bar = "Bar"
/// ```
/// and then have the attribute `name` of `foo` be "Foo".
/// This is intended for use by other components to have a nicer display the elements, since the ids
/// can sometimes be not very nice to read.
pub struct Names(Config, ComponentHandle);
impl Component for Names {
    const ID: &'static str = "names";
    type Config = Config;
    type ConfigError = Never;

    fn init(server: ComponentHandle, config: Self::Config) -> Result<Self, Self::ConfigError> {
        for (id, name) in &config.names {
            server.change_attribute(id, NAME_ATTRIBUTE_ID, AttributeValue::Custom(ByteCode::String(name.clone())));
        }
        Ok(Self(config, server))
    }

    fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::ConfigError> {
        for (id, name) in &config.names {
            if self.0.names.get(id) == Some(name) { continue; }
            self.1.change_attribute(id, NAME_ATTRIBUTE_ID, AttributeValue::Custom(ByteCode::String(name.clone())));
        }
        self.0 = config;
        Ok(())
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use crate::parse_test;
    parse_test!(empty(Config): toml::Table::new() => Config::default());
    parse_test!(with_names(Config): toml!{
        foo = "Foo"
        bar = "Bar"
    } => Config { names: HashMap::from([
        ("foo".to_string(), "Foo".to_string()),
        ("bar".to_string(), "Bar".to_string()),
    ])});
    parse_test!(invalid_nested(Config): toml!{
        foo = "Foo"
        bar = "Bar"
        smth.invalid = "Invalid"
    } => error);
}