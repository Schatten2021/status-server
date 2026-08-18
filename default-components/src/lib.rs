//! Library containing default components for my [Status server](server).
//!
//! Contains
//! - [`WebsiteStatuse`]: component for keeping track of the status of websites
//! - [`filters`]: utilities for configuring filters for [`server::NotificationProvider`]
//! - [`Api`]: An API for interacting with the [Status server](server)
//! - [`Frontend`]: A web frontend for easily checking the state of the server and elements.
//! - [`DataminerStatus`]: Component for keeping track of the status of dataminers.
//! - [`MinecraftStatus`]: Component for keeping track of the status of minecraft servers.
//! - [`EmailNotificationProvider`]: [`server::NotificationProvider`] for sending E-Mail notifications.
//! - [`NtfyNotificationProvider`]: [`server::NotificationProvider`] for sending Push-Notifications via [NTFY](https://ntfy.sh/)
 

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

#[allow(unused_imports, reason="is usually used when anything other than `filters` are selected.")]
#[macro_use]
extern crate tracing;

use utils::featured_use;

#[cfg(feature = "filters")]
pub mod filters;
mod config_wrappers;
pub(crate) use config_wrappers::{ Notification, Status };

featured_use!(if "websockets": websockets::Websockets);

featured_use!(if "website-status": website::WebsiteStatuse);
featured_use!(if "api": api::Api);
featured_use!(if "frontend": frontend::Frontend);
featured_use!(if "dataminer-status": dataminer::DataminerStatus);
featured_use!(if "minecraft-status": minecraft::MinecraftStatus);
featured_use!(if "email-notifications": email::EmailNotificationProvider);
featured_use!(if "ntfy-notifications": ntfy::NtfyNotificationProvider);

