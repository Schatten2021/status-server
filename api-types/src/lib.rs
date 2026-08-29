//! Types for the API to ensure that both the frontend and backend expect the same data.
#![cfg_attr(not(debug_assertions), deny(missing_docs))]
#![cfg_attr(debug_assertions, warn(missing_docs))]
#![warn(clippy::pedantic)]
#![warn(clippy::complexity, clippy::suspicious, clippy::perf, clippy::style, clippy::allow_attributes_without_reason)]
#![allow(
    clippy::needless_continue,
    reason = "adding a `continue` often makes the code easier to read."
)]
#![allow(
    clippy::missing_errors_doc,
    clippy::doc_markdown,
    reason = "don't want these lints."
)]
#![cfg_attr(not(debug_assertions), deny(clippy::undocumented_unsafe_blocks))]
#![cfg_attr(debug_assertions, warn(clippy::undocumented_unsafe_blocks))]

use std::collections::HashMap;
use std::fmt::Debug;

macro_rules! api_type {
    (
        $(#[$ty_attrs:meta])*
        struct $name:ident {$(
            $(#[$field_attr:meta])*
            $field_name:ident: $field_ty:ty
        ),* $(,)?}
    ) => {
        $(#[$ty_attrs])*
        #[derive(Clone, Debug, PartialEq, ::serde::Deserialize, ::serde::Serialize)]
        pub struct $name {$(
            $(#[$field_attr])*
            pub $field_name: $field_ty,
        )*}
        impl $crate::ApiType for $name {}
    };
    ($(#[$ty_attrs:meta])*
    struct $name:ident($(#[$inner_attrs:meta])*$inner:ty)) => {
        $(#[$ty_attrs])*
        #[derive(Clone, Debug, PartialEq, ::serde::Deserialize, ::serde::Serialize)]
        pub struct $name($(#[$inner_attrs])* pub $inner);
        impl $crate::ApiType for $name {}
    };

    (
        $(#[$ty_attrs:meta])*
        enum $name:ident {$(
            $(#[$variant_attr:meta])*
            $variant_name:ident$(($(#[$inner_attr:meta])*$inner:ty))?
        ),* $(,)?}
    ) => {
        $(#[$ty_attrs])*
        #[derive(Clone, Debug, PartialEq, ::serde::Deserialize, ::serde::Serialize)]
        pub enum $name {$(
            $(#[$variant_attr])*
            $variant_name$(($(#[$inner_attr])* $inner))?,
        )*}
        impl $crate::ApiType for $name {}
    };
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, ::serde::Serialize)]
#[expect(private_bounds, reason="this is supposed to be private to prevent accidentally sending the\
 wrong type that might not be understood by the client.")]
/// A response sent by the api
pub enum ApiResponse<V: ApiType=(), E: ApiType=()> {
    /// The request was handled successfully
    Ok(V),
    /// Some server error occurred trying to process the request.
    /// No fault of the client.
    ServerError(ServerError),
    /// The client did something wrong.
    ClientError(E),
}
trait ApiType {}
impl ApiType for () {}
impl ApiType for String {}
impl ApiType for &'static str {}

api_type!(
/// Errors that happen on the server-side
struct ServerError {
    /// The id of the error.
    id: String,
    /// The error message.
    message: String,
});
api_type!(
#[non_exhaustive]
/// A value an Attribute can have.
///
/// # Note
/// This is `#[non_exhaustive]`, so that adding a new variant in the future isn't a breaking change.
enum AttributeValue {
    /// Marks that an attribute exists with no value.
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
});
api_type!(
/// Data for an [`AttributeValue::Enum`]
struct EnumAttributeValue  {
    /// The identifier of the variant.
    variant: String,
    /// The actual value of the variant.
    value: Box<AttributeValue>,
});
#[cfg(feature = "server-support")]
impl From<server::AttributeValue> for AttributeValue {
    fn from(value: server::AttributeValue) -> Self {
        match value {
            server::AttributeValue::Marker => Self::Marker,
            server::AttributeValue::Custom(inner) => Self::Custom(inner),
            server::AttributeValue::Timestamp(dt) => Self::Timestamp(dt),
            server::AttributeValue::Percentage(v) => Self::Percentage(v),
            server::AttributeValue::History(hist) => Self::History(hist.into_iter()
                .map(|(a, b)| (a, b.into()))
                .collect()),
            _ => todo!("outdated api-types!")
        }
    }
}

api_type!(
/// The state of a single element.
struct State {
    /// Whether the element is currently online.
    online: bool,
    /// The attributes of the element.
    attributes: HashMap<String, AttributeValue>,
});
#[cfg(feature = "server-support")]
impl From<server::State> for State {
    fn from(value: server::State) -> Self {
        Self {
            online: value.online,
            attributes: value.attributes.into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect()
        }
    }
}
api_type!(
/// The current state of all elements (Hashmap maps element_id -> element_state).
struct States(HashMap<String, State>)
);
#[cfg(feature = "server-support")]
impl From<HashMap<String, server::State>> for States {
    fn from(value: HashMap<String, server::State>) -> Self {
        States(value.into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect())
    }
}
/// Types that are used when communicating via websockets.
pub mod websocket {
    use crate::AttributeValue;

    api_type!(
    /// A single websocket message.
    struct Message {
        /// The id of the element that changed.
        element_id: String,
        /// THe id of the component that caused the message.
        component_id: String,
        /// The reason why the message was sent.
        reason: MessageReason
    });
    api_type!(
    /// The reasons why a [`Message`] was sent.
    enum MessageReason {
        /// Something happened to the online status.
        OnlineStatus(OnlineStatusChange),
        /// Something happened to one of the attributes.
        Attribute(AttributeMessage),
    });
    api_type!(
    /// Changes that can happen to the online status.
    enum OnlineStatusChange {
        /// It (the element) was deleted.
        Delete,
        /// The element was created.
        Create(bool),
        /// The element was changed.
        Change(bool),
    });
    api_type!(
    /// infos for messages when an attribute was changed.
    struct AttributeMessage {
        /// The id of the attribute.
        attribute_id: String,
        /// The change that actually happened.
        change: AttributeChange,
    });
    api_type!(
    /// Changes to the attribute of an element.
    enum AttributeChange {
        /// The attribute was created (contains it's new value)
        Create(AttributeValue),
        /// The attribute was changed (contains it's new value)
        Change(AttributeValue),
        /// The attribute was deleted.
        Delete,
    });
    #[cfg(feature = "server-support")]
    impl From<server::Notification> for Message {
        fn from(value: server::Notification) -> Self {
            Self {
                element_id: value.element_id,
                component_id: value.component_id,
                reason: MessageReason::from(value.reason)
            }
        }
    }
    #[cfg(feature = "server-support")]
    impl From<server::NotificationReason> for MessageReason {
        fn from(value: server::NotificationReason) -> Self {
            match value {
                server::NotificationReason::OnlineStatusChanged(new) => Self::OnlineStatus(OnlineStatusChange::Change(new)),
                server::NotificationReason::NewElement(state) => Self::OnlineStatus(OnlineStatusChange::Create(state)),
                server::NotificationReason::AttributeEdit(edit) => Self::Attribute(AttributeMessage {
                    attribute_id: edit.id,
                    change: match edit.change {
                        server::AttributeValueChange::Create(val) => AttributeChange::Create(val.into()),
                        server::AttributeValueChange::Edit(_, val) => AttributeChange::Change(val.into()),
                        server::AttributeValueChange::Delete(_) => AttributeChange::Delete,
                    }
                }),
            }
        }
    }
}
pub mod history {
    //! Types for requesting the History of an element's online-state or attribute.
    use crate::AttributeValue;

    api_type!(
        /// Requests the history of an attribute
        struct AttributeHistoryRequest {
            /// The element of which the attribute history is requested.
            element_id: String,
            /// The attribute whose history is being requested.
            attribute_id: String,
        }
    );
    api_type!(
        /// A single element in an attribute's history.
        struct AttributeHistoryElement {
            /// The timestamp that the change was recorded
            timestamp: chrono::DateTime<chrono::Utc>,
            /// The new value (if any new value existed).
            ///
            /// A change to [`None`] indicates that the attribute was deleted.
            new_value: Option<AttributeValue>,
        }
    );
    api_type!(
        /// The entire history of a single attribute
        struct AttributeHistory(Vec<AttributeHistoryElement>)
    );

    api_type!(
        /// Requests the online-state history of an element
        struct OnlineStateHistoryRequest {
            /// The id of the element whose online-state history is being requested.
            element_id: String,
        }
    );
    api_type!(
        /// A single element in an element's online-state history
        struct OnlineStateHistoryElement {
            /// The timestamp that the change was recorded
            timestamp: chrono::DateTime<chrono::Utc>,
            /// The new online-state of hte element.
            new_state: bool,
        }
    );
    api_type!(
        /// The entire history of a single element's online-state
        struct OnlineStateHistory(Vec<OnlineStateHistoryElement>)
    );
}
