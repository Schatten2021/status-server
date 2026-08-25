use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::Deserialize;
use untyped::{Container, TypeMap, Untyped};
use crate::{AttributeValue, Component, ComponentHandle, Config, Notification, NotificationProvider};
/// # SAFETY
/// The type `P` must be the same as the [`Untyped`].
unsafe fn reconfigure_component<C: Component + 'static>(this: &mut Untyped, value: Option<toml::Value>) {
    use serde::Deserialize;
    let config = match value {
        None => Default::default(),
        Some(serialized) => match C::Config::deserialize(serialized) {
            Ok(v) => v,
            Err(e) => {
                error!("couldn't deserialize config of status provider \"{}\": {}", C::ID, e);
                return;
            }
        },
    };
    // SAFETY: The correctness of the type is guaranteed by the caller.
    let this = unsafe { <Untyped as Container<C>>::write(this) };
    match this.reconfigure(config) {
        Ok(()) => {},
        Err(e) => {
            error!("couldn't reconfigure component \"{}\": {}", C::ID, e);
        }
    }
}
/// # SAFETY
/// The type `C` MUST be the same as the [`Untyped`]
#[expect(clippy::result_large_err, reason="The error here isn't actually an error, but just the request if we fail to parse it.")]
unsafe fn try_handle_request<C: Component>(this: &Untyped, request: axum::extract::Request) -> Result<crate::component::RequestHandle, axum::extract::Request> {
    // SAFETY: The correctness of the type is guaranteed by the caller.
    unsafe {
        <Untyped as Container<C>>::read(this).try_handle(request)
    }
}
/// # SAFETY
/// The type `P` MUST be the same as the [`Untyped`]
unsafe fn notify_provider<P: NotificationProvider>(this: &Untyped, notification: Notification) {
    // SAFETY: The correctness of the type is guaranteed by the caller.
    unsafe {
        <Untyped as Container<P>>::read(this).notify(notification);
    }
}

pub(crate) struct GlobalState {
    pub(crate) states: RwLock<HashMap<String, crate::State>>,
    pub(crate) config: RwLock<Config>,
    pub(crate) config_path: RwLock<PathBuf>,
    pub(crate) components: RwLock<TypeMap<ComponentInfo>>,
    pub(crate) tasks: std::sync::mpsc::Sender<Task>,
}
#[derive(Clone, Debug)]
pub(crate) struct ComponentInfo {
    pub(super) reconfigure: unsafe fn(&mut Untyped, Option<toml::Value>),
    pub(super) try_handle_request: unsafe fn(&Untyped, request: axum::extract::Request) -> Result<crate::component::RequestHandle, axum::extract::Request>,
    pub(super) required_by: HashSet<TypeId>,
    pub(super) type_id: TypeId,
    pub(super) id: &'static str,
    pub(super) notification_provider_info: Option<NotificationProviderInfo>,
}
#[derive(Clone, Debug)]
pub(crate) struct NotificationProviderInfo {
    pub(super) notify: unsafe fn(&Untyped, Notification),
}
pub(crate) enum Task {
    SendNotification(Notification),
    EditAttribute(EditAttribute),
    EditOnlineState(&'static str, String, bool),
    Reconfigure,
}
pub(crate) struct EditAttribute {
    pub(crate) causing_component: &'static str,
    pub(crate) element_id: String,
    pub(crate) attribute_id: String,
    pub(crate) change: AttributeChange,
}
pub(crate) enum AttributeChange {
    Set(AttributeValue),
    Delete,
}
impl GlobalState {
    pub(crate) fn read_config(path: &Path) -> Config {
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
    pub(crate) fn create(config_path: PathBuf) -> Arc<Self> {
        let config = Self::read_config(config_path.as_path());
        let (sender, receiver) = std::sync::mpsc::channel();
        let runtime = tokio::runtime::Handle::current();
        let this = Arc::new(Self {
            states: RwLock::new(HashMap::new()),
            config: RwLock::new(config),
            config_path: RwLock::new(config_path),
            components: RwLock::new(TypeMap::new()),
            tasks: sender,
        });
        let this_ = this.clone();
        std::thread::spawn(move || super::backend::backend_thread(receiver, this_, runtime));
        this
    }
    pub(crate) fn enqueue(&self, task: Task) {
        self.tasks.send(task)
            .expect("unable to enqueue task");
    }
}
// management of states
impl GlobalState {
    pub(crate) fn set_attribute(&self, component_id: &'static str, element_id: &str, attribute_id: &str, value: AttributeValue) {
        self.enqueue(Task::EditAttribute(EditAttribute {
            causing_component: component_id,
            element_id: element_id.to_string(),
            attribute_id: attribute_id.to_string(),
            change: AttributeChange::Set(value),
        }));
    }
    pub(crate) fn get_attribute(&self, element_id: &str, attribute_id: &str) -> Option<AttributeValue> {
        self.states.read()
            .get(element_id)?
            .attributes.get(attribute_id).cloned()
    }
    pub(crate) fn delete_attribute(&self, component_id: &'static str, element_id: &str, attribute_id: &str, exact: bool) {
        let state_lock = self.states.read();
        let Some(element) = state_lock.get(element_id) else { return; };
        if exact {
            if !element.attributes.contains_key(attribute_id) { return; }
            drop(state_lock);
            self.enqueue(Task::EditAttribute(EditAttribute {
                causing_component: component_id,
                element_id: element_id.to_string(),
                attribute_id: attribute_id.to_string(),
                change: AttributeChange::Delete,
            }));
            return;
        }
        let attribute_ids = element.attributes.keys()
            .filter(|key| key.starts_with(attribute_id) && (key[attribute_id.len()..].is_empty() || key[attribute_id.len()..].starts_with('.')))
            .cloned()
            .collect::<Vec<_>>();
        let tasks = attribute_ids.into_iter()
            .map(|id| Task::EditAttribute(EditAttribute {
                causing_component: component_id,
                element_id: element_id.to_string(),
                attribute_id: id,
                change: AttributeChange::Delete,
            }))
            .collect::<Vec<_>>();
        for task in tasks {
            self.enqueue(task);
        }
    }
    pub(crate) fn set_online_state(&self, component_id: &'static str, element_id: &str, state: bool) {
        let element_lock = self.states.read();
        if let Some(element) = element_lock.get(component_id) && element.online == state {
            return;
        }
        self.enqueue(Task::EditOnlineState(component_id, element_id.to_string(), state));
    }
    pub(crate) fn get_online_state(&self, element_id: &str) -> Option<bool> {
        self.states.read()
            .get(element_id)
            .map(|elem| elem.online)
    }
    pub(crate) fn get_states(&self) -> HashMap<String, crate::State> {
        self.states.read().clone()
    }
}
// management of config
impl GlobalState {
    pub(crate) fn get_config_path(&self) -> PathBuf {
        self.config_path.read().clone()
    }
    pub(crate) fn set_config_path(&self, path: PathBuf) {
        *self.config_path.write() = path;
    }
    pub(crate) fn try_get_config<C: Component>(&self) -> Option<Result<C::Config, toml::de::Error>> {
        let raw = self.config.read()
            .configs.get(C::ID)?
            .clone();
        Some(C::Config::deserialize(raw))
    }
    pub(crate) fn get_config<C: Component>(&self) -> Option<C::Config> {
        self.try_get_config::<C>()?
            .map_err(|e| error!("error reading config for {}: {e}", C::ID))
            .ok()
    }
    pub(crate) fn is_ignored<C: Component>(&self) -> bool {
        self.config.read().global.ignored.components.contains(C::ID)
    }
    pub(crate) fn reload_config(&self) {
        info!("reloading config");
        let new_config = Self::read_config(self.config_path.read().as_path());
        let mut config_lock = self.config.write();
        *config_lock = new_config;
        self.enqueue(Task::Reconfigure);
    }
}
// management of components
impl GlobalState {
    pub(crate) fn has_component<C: Component>(&self) -> bool {
        self.components.read().contains_key::<C>()
    }
    // NOTE: No get_component, as the lock would have to live long enough.
    pub(crate) fn add_component<C: Component>(&self, handle: ComponentHandle) {
        if self.is_ignored::<C>() { debug!("ignored {}", C::ID); return; }
        if self.has_component::<C>() { debug!("already has component {}", C::ID); return; }
        let config = self.get_config::<C>()
            .unwrap_or_else(|| {
                info!("no valid component {}", C::ID);
                C::Config::default()
            });
        let component = match C::init(handle, config) {
            Ok(v) => v,
            Err(e) => {
                error!("couldn't initialize component `{}`: {e}", C::ID);
                return;
            }
        };
        let data = ComponentInfo {
            reconfigure: reconfigure_component::<C>,
            try_handle_request: try_handle_request::<C>,
            required_by: HashSet::new(),
            type_id: TypeId::of::<C>(),
            id: C::ID,
            notification_provider_info: None,
        };
        if self.components.write().insert(component, data).is_some() {
            error!("inserted component `{}` twice!", C::ID);
        }
    }
    pub(crate) fn add_notification_provider<P: NotificationProvider>(&self, handle: ComponentHandle) {
        let notification_info = NotificationProviderInfo {
            notify: notify_provider::<P>,
        };
        self.add_component::<P>(handle);
        let mut component_lock = self.components.write();
        let Some(component) = component_lock.additional_data_mut::<P>() else { return; /* error while inserting; logged already => do nothing.*/};
        component.notification_provider_info = Some(notification_info);
    }
    pub(crate) fn add_dependency<C: Component>(&self, depended_on_by: TypeId) {
        let mut component_lock = self.components.write();
        let Some(component) = component_lock.additional_data_mut::<C>() else {
            error!("unmet dependency: {} is not present yet!", C::ID);
            return;
        };
        component.required_by.insert(depended_on_by);
    }
    pub(crate) fn remove_component<C: Component>(&self) {
        let mut component_lock = self.components.write();
        let Some((_, infos)) = component_lock.remove::<C>() else { debug!("component {} wasn't even present", C::ID); return; };
        let mut dependencies = infos.required_by;
        while !dependencies.is_empty() {
            let to_remove = dependencies;
            dependencies = HashSet::new();
            for item in to_remove {
                let Some((_, infos)) = component_lock.remove_by_type_id(&item) else { debug!("dependency was already removed"); return; };
                dependencies.extend(infos.required_by);
            }
        }
    }
}
impl GlobalState {
    #[expect(clippy::result_large_err, reason="The error here isn't actually an error, but just the request if we fail to parse it.")]
    pub(crate) fn try_handle_request(&self, mut request: axum::extract::Request) -> Result<crate::component::RequestHandle, axum::extract::Request> {
        for (component, info) in self.components.read().entries() {
            // SAFETY: The correctness of the types is ensured by the creation of `try_handle_request` and `components.entries`
            request = match unsafe { (info.try_handle_request)(component, request) } {
                Ok(handle) => return Ok(handle),
                Err(r) => r,
            }
        }
        Err(request)
    }
}