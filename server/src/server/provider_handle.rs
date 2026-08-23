use super::Server;
use crate::notification_provider::NotificationProvider;
use crate::state::AttributeValue;
use crate::{Component, State};
use parking_lot::RwLock;
use std::any::TypeId;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
/// Handle to the server for [`Component`]s to use.
///
/// # Note
/// These are customized for each component.
/// If you share these between different components, the resulting notifications _will_ appear to be
/// sent by the wrong component.
pub struct ComponentHandle {
    backend: Arc<RwLock<Server>>,
    id: &'static str,
    type_id: TypeId,
}
impl ComponentHandle {
    pub(super) fn new<P: Component>(backend: Arc<RwLock<Server>>) -> Self {
        Self { 
            backend, 
            id: P::ID, 
            type_id: TypeId::of::<P>() 
        }
    }
    /// Add a [`NotificationProvider`] dependency.
    ///
    /// # Note
    /// There is no check for recursive dependencies. Do not use recursive dependencies.
    pub fn add_notification_provider_dependency<P: NotificationProvider>(&self) {
        if self.backend.read().ignored::<P>() {
            error!("ignored dependency {} for {}!", P::ID, self.id);
            return;
        }
        if !self.backend.read().has_component::<P>() {
            let handle = Self::new::<P>(self.backend.clone());
            let config = self.backend.read().get_config::<P>();
            let provider = match P::init(handle, config) {
                Ok(v) => v,
                Err(e) => {
                    error!("error initializing component: {e}; skipping...");
                    return;
                }
            };
            self.backend.write().add_notification_provider(provider);
        }
        self.backend.write().add_notification_provider_dependency::<P>(
            self.type_id,
        );
    }
    /// Add a [`Component`] dependency.
    ///
    /// # Note
    /// There is no check for recursive dependencies. Do not use recursive dependencies.
    pub fn add_component_dependency<C: Component>(&self) {
        if self.backend.read().ignored::<C>() {
            error!("ignored dependency {} for {}!", C::ID, self.id);
            return;
        }
        if !self.backend.read().has_component::<C>() {
            let handle = Self::new::<C>(self.backend.clone());
            let component = match C::init(handle, self.backend.read().get_config::<C>()) {
                Ok(v) => v,
                Err(e) => {
                    error!("error initializing component: {e}; skipping...");
                    return;
                }
            };
            self.backend.write().add_component(component);
        }
        self.backend.write().add_component_dependency::<C>(
            self.type_id,
        );
    }
    /// retrieves a reference to a component from the server and applies the map function to it.
    ///
    /// # Note
    /// This is this way because the actual server is behind an [`Arc`] reference and if we didn't
    /// do it this way there would be lifetime issues.
    // NOTE: I could build a custom struct that houses the reference to the component & the lock.
    //       Might do that in the future.
    pub fn component_map<C: Component, F: FnOnce(Option<&C>) -> V, V>(&self, func: F) -> V {
        func(self.backend.read().get_component())
    }
    /// retrieves a mutable reference to a component from the server and applies the map function to it.
    ///
    /// # Note
    /// This is this way because the actual server is behind an [`Arc`] reference and if we didn't
    /// do it this way there would be lifetime issues.
    // NOTE: I could build a custom struct that houses the reference to the component & the lock.
    //       Might do that in the future.
    pub fn component_map_mut<C: Component, F: FnOnce(Option<&mut C>) -> V, V>(&self, func: F) -> V {
        func(self.backend.write().get_component_mut())
    }
    /// Changes the online state of an element.
    ///
    /// # Note
    /// Please make sure that the state actually changes via [`Self::get_online_state`].
    /// Calling this function without checking the online state first is a lot slower.
    pub fn change_online_state(&self, element_id: &str, status: bool) {
        self.backend.write().online_status_changed(self.id, element_id, status);
    }
    /// Retrieves the online state of an element.
    #[must_use]
    pub fn get_online_state(&self, element_id: &str) -> Option<bool> {
        self.backend.read().get_status(element_id)
    }
    /// Returns a copy of all elements and their states.
    #[must_use]
    pub fn get_states(&self) -> HashMap<String, State> {
        self.backend.read().get_states()
    }
    /// Changes the attribute of an element.
    ///
    /// # Note
    /// Please make sure that the state actually changes via [`Self::get_online_state`].
    /// Calling this function without checking the online state first is a lot slower and sends
    /// unnecessary notifications.
    pub fn change_attribute(&self, element_id: &str, attribute_id: &str, value: AttributeValue) {
        self.backend.write().attribute_change(self.id, element_id, attribute_id, value);
    }
    /// Retrieves the given attribute of an element.
    #[must_use]
    pub fn get_attribute(&self, element_id: &str, attribute_id: &str) -> Option<AttributeValue> {
        self.backend.read().get_attribute(element_id, attribute_id)
    }
    #[expect(clippy::doc_overindented_list_items, reason="this is for easier reading while editing.")]
    /// Deletes an attribute for an element.
    ///
    /// # Arguments
    /// * `element_id`: The id of the element whose attribute is to be deleted
    /// * `attribute_id`: The id of the attribute to be deleted
    /// * `exact`: Whether to delete only this attribute or also all subattributes.
    ///            NOTE: It could be very good for performance if you know that there will _not_ be
    ///                  any subattributes to set this, as it currently has to check every single
    ///                  attribute of the element.
    pub fn delete_attribute(&self, element_id: &str, attribute_id: &str, exact: bool) {
        self.backend.write().delete_attribute(element_id, attribute_id, self.id, exact);
    }
    /// reload the config from the config file.
    #[expect(clippy::must_use_candidate, reason="returning something here is more just for chaining.")]
    pub fn reload_config(&self) -> &Self {
        self.backend.write().reload_config();
        self
    }

    /// returns the current config path.
    #[must_use]
    pub fn get_config_path(&self) -> PathBuf {
        self.backend.read().get_config_path()
    }
    /// sets the config path
    pub fn set_config_path(&self, path: PathBuf) {
        self.backend.write().set_config_path(path);
    }
}
