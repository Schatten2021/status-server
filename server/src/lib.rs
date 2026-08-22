//! Infrastructure for my Data-server
//! 
//! This crate contains all the necessary definitions & wiring to allow the creation of 
//! [`Component`]s and [`NotificationProvider`]s.
//! 
//! Most importantly are:
//! - [`ServerHandle`]: for interacting with the backend (initializing the server, etc.)
//! - [`Component`]: for building components, getting access to new element-/notification-types
//! - [`NotificationProvider`]: for actually sending out notifications
//! - [`ComponentHandle`]: Handle for [`Components`] to interact with the server. 
//!     - IMPORTANT: These are customized for each [`Component`]. Do not switch these between components, or it will appear that the other component is sending the messages.

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


#[macro_use]
extern crate tracing;


mod state;
mod config;
mod notification_provider;
mod component;
mod server;
mod notification;

pub use server::{
    ComponentHandle,
    ServerHandle as Server,
};
pub use config::Config;
pub use component::{
    Component,
    RequestHandle,
};
pub use notification::{
    Notification,
    NotificationReason,
    AttributeEdit,
    AttributeValueChange,
};
pub use state::{
    State,
    AttributeValue,
};
pub use notification_provider::NotificationProvider;
