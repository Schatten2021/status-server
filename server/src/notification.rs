use crate::state::AttributeValue;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
/// A Notification of a changing state for an element.
pub struct Notification {
    /// The id of the component that triggered this change.
    ///
    /// Primarily meant for filtering.
    pub component_id: String,
    /// The id of the element that experienced a change.
    pub element_id: String,
    /// The reason this notification was sent out.
    pub reason: NotificationReason,
}
impl Notification {
    /// creates a new [`Notification`]
    #[must_use]
    pub const fn new(component_id: String, element_id: String, reason: NotificationReason) -> Self {
        Self { component_id, element_id, reason, }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
/// Reasons why a notification might be sent out
pub enum NotificationReason {
    /// The online status of the element changed.
    ///
    /// The inner state is the new online state.
    ///
    /// # Note
    /// When receiving such a notification it is guaranteed that the previous state was `!current`.
    OnlineStatusChanged(bool),
    /// An attribute was edited (created/changed/deleted).
    AttributeEdit(AttributeEdit),
    /// A new element was created and was designated the given online status.
    NewElement(bool),
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
/// An attribute was edited.
pub struct AttributeEdit {
    /// The id of the attribute that was edited
    pub id: String,
    /// The actual change that happened.
    pub change: AttributeValueChange,
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
/// Changes to the attribute value when an attribute was edited.
pub enum AttributeValueChange {
    /// The attribute was created
    Create(AttributeValue),
    /// The attribute was edited (format: (old, new))
    Edit(AttributeValue, AttributeValue),
    /// The attribute was deleted.
    Delete(AttributeValue)
}