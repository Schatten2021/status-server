use std::collections::HashMap;
use std::fmt::Formatter;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
/// The state that an element has.
pub struct State {
    /// whether the element is currently online.
    pub online: bool,
    /// The attributes of the element.
    pub attributes: HashMap<String, AttributeValue>
}
impl State {
    /// creates a new offline state
    #[must_use]
    pub fn new() -> Self {
        Self {
            online: false,
            attributes: HashMap::new(),
        }
    }
    /// creates a new state.
    #[must_use]
    pub fn init(online: bool, attributes: HashMap<String, AttributeValue>) -> Self {
        Self { online, attributes }
    }
    /// creates a new state with the given online status.
    #[must_use]
    pub fn with_online(online: bool) -> Self {
        Self {
            online,
            attributes: HashMap::new(),
        }
    }
}
#[derive(Default, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
/// A value an Attribute can have.
/// 
/// # Note
/// This is `#[non_exhaustive]`, so that adding a new variant in the future isn't a breaking change.
pub enum AttributeValue {
    /// Marker variant only present to mark that an attribute exists.
    #[default]
    Marker,
    /// Some custom (or primitive) format
    Custom(bytecode::ByteCode),
    /// A DateTime.
    /// 
    /// This is very useful for [`crate::Component`]s that store the last seen time in the attributes.
    Timestamp(chrono::DateTime<chrono::Utc>),
    /// A percentage.
    Percentage(f32),
    /// A history over (previous) values.
    History(Vec<(chrono::DateTime<chrono::Utc>, AttributeValue)>),
}
impl std::fmt::Display for AttributeValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AttributeValue::Marker => Ok(()),
            AttributeValue::Custom(inner) => inner.fmt(f),
            AttributeValue::Timestamp(dt) => dt.format("%d.%m.%Y %H:%M:%S%.3f").fmt(f),
            AttributeValue::Percentage(val) => write!(f, "{:.2}%", val * 100.0),
            AttributeValue::History(history) => {
                for (timestamp, value) in history {
                    timestamp.format("%d.%m.%Y %H:%M:%S%.3f").fmt(f)?;
                    writeln!(f, ": {value},")?;
                }
                Ok(())
            }
        }
    }
}