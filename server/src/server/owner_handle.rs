use super::Server;
use crate::notification_provider::NotificationProvider;
use crate::{Component, ComponentHandle};
use parking_lot::RwLock;
use std::any::TypeId;
use std::collections::HashMap;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use axum::extract::Request;
use crate::state::State;

#[derive(Clone)]
/// A Handle to a Server, used to configure said server or interact with it outside of [`Component`]s.
///
/// Cloning this handle does not clone the server.
///
/// # Note
/// This is also an [`axum::handler::Handle`], so that you can use it in a [`axum::Router::route`]
/// call.
pub struct ServerHandle(Arc<RwLock<Server>>);
impl ServerHandle {
    /// creates a new Server loading the config from the given path.
    #[must_use]
    pub fn new(config_path: PathBuf) -> Self {
        Self(Arc::new(RwLock::new(Server::new(config_path))))
    }
    /// Adds a new [`Component`] (& dependencies) to the server.
    ///
    /// # Note
    /// Notification providers need to be registered via [`Self::add_notification_provider`],
    /// so that the server knows that they can send notifications.
    // #[expect(clippy::must_use_candidate, reason="returning something here is more just for chaining.")]
    pub fn add_component<C: Component>(&self) -> &Self {
        if self.0.read().ignored::<C>() {
            info!("ignored component {}", C::ID);
            return self;
        }
        let config = self.0.read().get_config::<C>();
        let component = match C::init(self.provider_handle::<C>(), config) {
            Ok(v) => v,
            Err(e) => {
                error!("couldn't initialize component {}: {e}", C::ID);
                return self
            }
        };
        self.0.write().add_component::<C>(component);
        self
    }
    /// Removes a component (& dependant components) from the server.
    #[expect(clippy::must_use_candidate, reason="returning something here is more just for chaining.")]
    pub fn remove_component<C: Component>(&self) -> &Self {
        self.0.write().remove_component(TypeId::of::<C>());
        self
    }
    /// Adds a new [`NotificationProvider`] to the server.
    // #[expect(clippy::must_use_candidate, reason="returning something here is more just for chaining.")]
    pub fn add_notification_provider<P: NotificationProvider>(&self) -> &Self {
        if self.0.read().ignored::<P>() {
            info!("ignored component {}", P::ID);
            return self;
        }
        let config = self.0.read().get_config::<P>();
        let provider = match P::init(self.provider_handle::<P>(), config) {
            Ok(v) => v,
            Err(e) => {
                error!("couldn't initialize component {}: {e}", P::ID);
                return self
            }
        };
        self.0.write().add_notification_provider::<P>(provider);
        self
    }
    /// checks the config, returning the deserialized config if it exists or the error if it doesn't.
    #[must_use]
    pub fn check_config<C: Component>(&self) -> Option<Result<C::Config, toml::de::Error>> {
        self.0.read().check_config::<C>()
    }
    /// reload the config from the config file.
    #[expect(clippy::must_use_candidate, reason="returning something here is more just for chaining.")]
    pub fn reload_config(&self) -> &Self {
        self.0.write().reload_config();
        self
    }
    /// retrieves a reference to a component from the server and applies the map function to it.
    ///
    /// # Note
    /// This is this way because the actual server is behind an [`Arc`] reference and if we didn't
    /// do it this way there would be lifetime issues.
    // NOTE: I could build a custom struct that houses the reference to the component & the lock.
    //       Might do that in the future.
    pub fn component_map<C: Component, F: FnOnce(Option<&C>) -> V, V>(&self, func: F) -> V {
        func(self.0.read().get_component())
    }
    /// retrieves a mutable reference to a component from the server and applies the map function to it.
    ///
    /// # Note
    /// This is this way because the actual server is behind an [`Arc`] reference and if we didn't
    /// do it this way there would be lifetime issues.
    // NOTE: I could build a custom struct that houses the reference to the component & the lock.
    //       Might do that in the future.
    pub fn component_map_mut<C: Component, F: FnOnce(Option<&mut C>) -> V, V>(&self, func: F) -> V {
        func(self.0.write().get_component_mut())
    }
    fn provider_handle<P: Component>(&self) -> ComponentHandle {
        ComponentHandle::new::<P>(self.0.clone())
    }
    /// Returns a copy of all elements and their states.
    #[must_use]
    pub fn get_states(&self) -> HashMap<String, State> {
        self.0.read().get_states()
    }

    /// returns the current config path.
    #[must_use]
    pub fn get_config_path(&self) -> PathBuf {
        self.0.read().get_config_path()
    }
    /// sets the config path
    pub fn set_config_path(&self, path: PathBuf) {
        self.0.write().set_config_path(path);
    }
}
impl axum::handler::Handler<(), ()> for ServerHandle {
    type Future = Pin<Box<dyn Future<Output=axum::response::Response> + Send + 'static>>;

    fn call(self, req: Request, (): ()) -> Self::Future {
        match self.0.read().try_handle_request(req) {
            Ok(v) => v,
            Err(r) => Box::pin(std::future::ready({
                info!("unable to handle request to {}", r.uri());
                axum::response::Response::builder()
                    .status(404)
                    .body(axum::body::Body::new(r#"<script> window.socket = new WebSocket("ws://127.0.0.1:8000/api/ws");</script>"#.to_string()))
                    .unwrap()
            }))
        }
    }
}