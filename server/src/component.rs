use std::pin::Pin;
// use std::fmt::{Display, Formatter};
use crate::ComponentHandle;

/// A pinned future that handles the given request.
pub type RequestHandle = Pin<Box<dyn Future<Output=axum::response::Response> + Send >>;

#[async_trait::async_trait]
/// A single Component in a server.
pub trait Component: Sized + Send + Sync + 'static {
    /// The id of the component.
    /// 
    /// This is used to identify the config key for this component.
    const ID: &'static str;
    
    /// The type of the config.
    /// 
    /// The original toml config will be further parsed down into this.
    type Config: serde::Serialize + for<'de> serde::Deserialize<'de> + Default;
    /// Errors that can occur while configuring the server.
    type ConfigError: core::error::Error;
    /// Initialize the [`Component`] with the given server & config.
    fn init(server: ComponentHandle, config: Self::Config) -> Result<Self, Self::ConfigError>;
    /// trigger a reconfiguration of the component.
    ///
    /// # Note
    /// When reconfiguring, a component must not call a function on the [`ComponentHandle`] that
    /// requires other components (such as [`ComponentHandle::add_component_dependency`] or similar),
    /// as that could result in a deadlock.
    fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::ConfigError>;
    #[expect(clippy::result_large_err, reason="The error here isn't actually an error, but just the request if we fail to parse it.")]
    /// Try to handle the request sent to the server.
    /// 
    /// # Returns
    /// This function returns `Ok(Box::pin(async {...; response }))` if the request can be handled 
    /// by the component and `Err(request)` in the case that it can't. The request in the `Err` case
    /// is expected to be the same that was passed in so that it can be passed to the next component.
    fn try_handle(&self, request: axum::extract::Request) -> Result<RequestHandle, axum::extract::Request> {
        Err(request)
    }
}