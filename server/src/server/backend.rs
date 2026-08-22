use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use untyped::{TypeMap, Untyped};
use crate::config::Config;
use crate::Component;
use crate::notification::{AttributeEdit, AttributeValueChange, Notification, NotificationReason};
use crate::notification_provider::NotificationProvider;
use crate::state::{AttributeValue, State};

pub(super) struct Server {
    config_path: PathBuf,
    loaded_config: Config,
    components: TypeMap<ComponentInfo>,
    states: HashMap<String, State>,
}
#[derive(Clone, Debug)]
struct ComponentInfo {
    reconfigure: unsafe fn(&mut Untyped, Option<toml::Value>),
    try_handle_request: unsafe fn(&Untyped, request: axum::extract::Request) -> Result<crate::component::RequestHandle, axum::extract::Request>,
    required_by: HashSet<TypeId>,
    type_id: TypeId,
    id: &'static str,
    notification_provider_info: Option<NotificationProviderInfo>,
}
#[derive(Clone, Debug)]
struct NotificationProviderInfo {
    notify: unsafe fn(&Untyped, Notification),
}

/// # SAFETY
/// The type `P` must be the same as the [`Untyped`].
unsafe fn reconfigure_component<P: Component + 'static>(this: &mut Untyped, value: Option<toml::Value>) {
    use serde::Deserialize;
    let config = match value {
        None => Default::default(),
        Some(serialized) => match P::Config::deserialize(serialized) {
            Ok(v) => v,
            Err(e) => {
                error!("couldn't deserialize config of status provider \"{}\": {}", P::ID, e);
                return;
            }
        },
    };
    // SAFETY: The correctness of the type is guaranteed by the caller.
    let this = unsafe { this.read_mut::<P>() };
    match this.reconfigure(config) {
        Ok(()) => {},
        Err(e) => {
            error!("couldn't reconfigure status provider \"{}\": {}", P::ID, e);
        }
    }
}
/// # SAFETY
/// The type `C` MUST be the same as the [`Untyped`]
#[expect(clippy::result_large_err, reason="The error here isn't actually an error, but just the request if we fail to parse it.")]
unsafe fn try_handle_request<C: Component>(this: &Untyped, request: axum::extract::Request) -> Result<crate::component::RequestHandle, axum::extract::Request> {
    // SAFETY: The correctness of the type is guaranteed by the caller.
    unsafe {
        this.read::<C>().try_handle(request)
    }
}
/// # SAFETY
/// The type `P` MUST be the same as the [`Untyped`]
unsafe fn notify_provider<P: NotificationProvider>(this: &Untyped, notification: Notification) {
    // SAFETY: The correctness of the type is guaranteed by the caller.
    unsafe {
        this.read::<P>().notify(notification);
    }
}
fn read_config(path: impl AsRef<Path>) -> Config {
    let path = path.as_ref();
    let config_str = match std::fs::read_to_string(path) {
        Ok(v) => {
            trace!("read config file `{}`: {v:?}", path.to_string_lossy());
            v
        },
        Err(e) => {
            error!("couldn't read config file `{}`: {e}", path.to_string_lossy());
            return Config::default();
        }
    };
    match toml::from_str(&config_str) {
        Ok(v) => {
            debug!("read config: {v:?}");
            v
        },
        Err(e) => {
            error!("invalid config file: `{}`: {e}", path.to_string_lossy());
            Config::default()
        }
    }
}
// component management
impl Server {
    pub(crate) fn add_notification_provider_dependency<P: NotificationProvider>(&mut self, dependant: TypeId) {
        if self.loaded_config.global.ignored.components.contains(P::ID) {
            error!("dependency {} cannot be satisfied as it is set to be ignored.", P::ID);
            return;
        }
        if !self.components.contains_key::<P>() {
            error!("component {} not yet loaded!", P::ID);
        }
        self.add_component_dependency::<P>(dependant);
        let info = NotificationProviderInfo {
            notify: notify_provider::<P>
        };
        self.components.additional_data_mut::<P>()
            .expect("just inserted it").notification_provider_info = Some(info);
    }
    pub(crate) fn add_component_dependency<C: Component>(&mut self, dependant: TypeId) {
        if self.loaded_config.global.ignored.components.contains(C::ID) {
            error!("dependency {} cannot be satisfied as it is set to be ignored.", C::ID);
            return;
        }
        if !self.components.contains_key::<C>() {
            error!("component {} not yet loaded!", C::ID);
            return;
        }
        self.components.additional_data_mut::<C>().expect("just checked its existance")
            .required_by.insert(dependant);
    }
    pub(crate) fn add_notification_provider<P: NotificationProvider>(&mut self, provider: P) {
        if self.loaded_config.global.ignored.components.contains(P::ID) { return; }
        self.add_component::<P>(provider);
        let info = NotificationProviderInfo {
            notify: notify_provider::<P>
        };
        self.components.additional_data_mut::<P>()
            .expect("just inserted it").notification_provider_info = Some(info);
    }
    pub(crate) fn add_component<C: Component>(&mut self, component: C) {
        if self.loaded_config.global.ignored.components.contains(C::ID) { return; }
        
        if self.components.contains_key::<C>() {
            debug!("called `add_component` with already registered component! Ignoring.");
            return;
        }
        
        // management info
        let data = ComponentInfo {
            reconfigure: reconfigure_component::<C>,
            try_handle_request: try_handle_request::<C>,
            required_by: HashSet::new(),
            type_id: TypeId::of::<C>(),
            id: C::ID,
            notification_provider_info: None,
        };
        
        assert!(self.components.insert(component, data).is_none(), "checked that the component was not present already, but now it somehow is?");
    }
    pub(crate) fn remove_component(&mut self, type_id: TypeId) {
        let Some((info, _)) = self.components.remove_by_type_id(&type_id) else {
            error!("tried to remove component that wasn't even present");
            return;
        };
        for dependant in info.required_by {
            self.remove_component(dependant);
        }
    }
    pub(crate) fn get_component<C: Component>(&self) -> Option<&C> {
        self.components.get::<C>()
    }
    pub(crate) fn get_component_mut<C: Component>(&mut self) -> Option<&mut C> {
        self.components.get_mut::<C>()
    }
    pub(crate) fn has_component<C: Component>(&self) -> bool {
        self.components.contains_key::<C>()
    }
    pub(crate) fn get_config<C: Component>(&self) -> C::Config {
        use serde::Deserialize;
        match self.loaded_config.configs.get(C::ID) {
            None => C::Config::default(),
            Some(serialized) => C::Config::deserialize(serialized.clone())
                .unwrap_or_else(|e| {
                    error!("couldn't deserialize config for `{}` due to: {e}", C::ID);
                    C::Config::default()
                })
        }
    }
    pub(crate) fn check_config<C: Component>(&self) -> Option<Result<C::Config, toml::de::Error>> {
        use serde::Deserialize;
        let conf = self.loaded_config.configs.get(C::ID)?;
        Some(C::Config::deserialize(conf.clone()))
    }
    pub(crate) fn ignored<C: Component>(&self) -> bool {
        self.loaded_config.global.ignored.components.contains(C::ID)
    }
}
impl Server {
    pub(crate) fn new(config_path: PathBuf) -> Self {
        Self {
            components: TypeMap::new(),
            states: HashMap::new(),
            loaded_config: read_config(&config_path),
            config_path,
        }
    }
    pub(crate) fn reload_config(&mut self) {
        info!("reloading config");
        self.loaded_config = read_config(&self.config_path);
        for to_remove in self.components.entries_mut()
            .filter_map(|(data, component)| {
                if self.loaded_config.global.ignored.components.contains(data.id) {
                    return Some(data.type_id);
                }
                let config = self.loaded_config.configs.get(data.id).cloned();
                // SAFETY: That the type is the same is guaranteed by the creation of 
                //         `data.configure` and `TypeMap::entries_mut`.
                unsafe {
                    (data.reconfigure)(component, config);
                }
                None
            }).collect::<Vec<_>>() {
            self.remove_component(to_remove);
        }
    }
}
// State changes
impl Server {
    pub(crate) fn attribute_change(&mut self, component_id: &'static str, element_id: &str, attribute_id: &str, value: AttributeValue) {
        #[expect(clippy::single_match_else, reason="using a match here actually makes it more readable.")]
        let state = match self.states.get_mut(element_id) {
            Some(state) => state,
            None => {
                self.states.insert(element_id.to_string(), State::new());
                self.notify(Notification::new(
                    component_id.to_string(),
                    element_id.to_string(),
                    NotificationReason::NewElement(false)
                ));
                self.states.get_mut(element_id)
                    .expect("just inserted, but not present?")
            }
        };
        let old_val = state.attributes.insert(attribute_id.to_string(), value.clone());
        let notification = Notification::new(
            component_id.to_string(),
            element_id.to_string(),
            NotificationReason::AttributeEdit(AttributeEdit {
                id: attribute_id.to_string(),
                change: match old_val {
                    Some(old) => AttributeValueChange::Edit(old, value),
                    None => AttributeValueChange::Create(value)
                }
            })
        );
        self.notify(notification);
    }
    pub(crate) fn get_attribute(&self, element_id: &str, attribute_id: &str) -> Option<AttributeValue> {
        self.states.get(element_id)?
            .attributes.get(attribute_id).cloned()
    }
    pub(crate) fn delete_attribute(&mut self, element_id: &str, attribute_id: &str, component_id: &str, exact: bool) {
        let Some(element) = self.states.get_mut(element_id) else { return; };
        if exact {
            if let Some(old) = element.attributes.remove(attribute_id) {
                self.notify(Notification::new(
                    component_id.to_string(),
                    element_id.to_string(),
                    NotificationReason::AttributeEdit(AttributeEdit {
                        id: attribute_id.to_string(),
                        change: AttributeValueChange::Delete(old),
                    }))
                );
            }
            return;
        }
        element.attributes.keys()
            .filter(|key| key.starts_with(attribute_id) && key.get(attribute_id.len()..)
                .is_none_or(|v| v.starts_with('.')))
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .map(|id| {
                let old = element.attributes.remove(&id).unwrap();
                (id, old)
            })
            .collect::<Vec<_>>().into_iter()
            .for_each(|(id, old)| self.notify(Notification::new(
                component_id.to_string(),
                element_id.to_string(),
                NotificationReason::AttributeEdit(AttributeEdit {
                    id,
                    change: AttributeValueChange::Delete(old),
                }))
            ));
    }
    pub(crate) fn online_status_changed(&mut self, component_id: &'static str, element_id: &str, new_status: bool) {
        if let Some(state) = self.states.get_mut(element_id) {
            if state.online == new_status {
                warn!("called online_status_changed without changing the online status; ignoring");
                return;
            }
            state.online = new_status;
            self.notify(Notification::new(
                component_id.to_string(),
                element_id.to_string(),
                NotificationReason::OnlineStatusChanged(new_status)
            ));
        } else {
            self.states.insert(element_id.to_string(), State::with_online(new_status));
            self.notify(Notification::new(
                component_id.to_string(),
                element_id.to_string(),
                NotificationReason::NewElement(new_status),
            ));
        }
    }
    pub(crate) fn get_status(&self, element_id: &str) -> Option<bool> {
        self.states.get(element_id)
            .map(|state| state.online)
    }
    #[expect(clippy::needless_pass_by_value, reason="_semantically_ we are passing ownership to the function.")]
    pub(crate) fn notify(&self, notification: Notification) {
        trace!("sending out notification: {notification:?}");
        self.components.entries()
            .filter_map(|(data, component)| {
                data.notification_provider_info.as_ref().zip(Some(component))
            })
            .for_each(|(data, component)| {
                // SAFETY: The correctness of the type is guaranteed by the creation of
                //         `NotificationProviderInfo::notify` and `TypeMap::entries`.
                unsafe {
                    (data.notify)(component, notification.clone());
                }
            });
    }
    pub(crate) fn get_states(&self) -> HashMap<String, State> {
        self.states.clone()
    }
}
impl Server {
    #[expect(clippy::result_large_err, reason="The error here isn't actually an error, but just the request if we fail to parse it.")]
    pub(crate) fn try_handle_request(&self, mut request: axum::extract::Request) -> Result<crate::component::RequestHandle, axum::extract::Request> {
        for (info, component) in self.components.entries() {
            // SAFETY: The correctness of the types is ensured by the creation of `try_handle_request` and `components.entries`
            request = match unsafe { (info.try_handle_request)(component, request) } {
                Ok(handle) => return Ok(handle),
                Err(r) => r,
            }
        }
        Err(request)
    }
}