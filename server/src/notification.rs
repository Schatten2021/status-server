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
    /// An attribute was created.
    AttributeCreated(String, AttributeValue),
    /// An attribute was changed.
    ///
    /// # Note
    /// Element format: (id, old, new)
    AttributeChanged(String, AttributeValue, AttributeValue),
    /// An attribute was deleted.
    ///
    /// # Note
    /// contains the last attribute value.
    AttributeDeleted(String, AttributeValue),
    /// A new element was created and was designated the given online status.
    NewElement(bool),
}