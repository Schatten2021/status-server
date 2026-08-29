use std::error::Error;
use utils::featured_use;
use server::{ComponentHandle, AttributeValue};
use super::{PropertyHistory, OnlineStateHistory};

featured_use!(if "history-fs-json-backend": json::FsJsonBackend);
featured_use!(if "history-sqlite-backend": sqlite::SqliteBackend);

pub trait Backend: Sized + Send + Sync + 'static {
    type Config: serde::Serialize + for<'de> serde::Deserialize<'de> + Default;
    /// Errors that can occur while configuring the backend.
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

    /// Adds an attribute change to the history
    fn add_attribute_change(&self, element: &str, attribute: &str, timestamp: chrono::DateTime<chrono::Utc>, new_val: Option<&AttributeValue>)-> Result<(), Box<dyn Error>> ;
    /// retrieves an attributes history
    fn get_attribute_history(&self, element: &str, attribute: &str) -> Result<PropertyHistory, Box<dyn Error>>;
    // /// returns a list of all historized attributes for the given element
    // fn all_historized_attributes(&self, element: &str) -> Vec<String>;
    // /// retrieves all attributes histories for the given element
    // fn all_component_attribute_histories(&self, element: &str) -> HashMap<String, PropertyHistory> {
    //     self.all_historized_attributes(element)
    //         .into_iter()
    //         .map(|attribute_id| {
    //             let history = self.get_attribute_history(element, &attribute_id);
    //             (attribute_id, history)
    //         })
    //         .collect()
    // }
    /// adds an online state change to the history
    fn add_online_state_change(&self, element: &str, timestamp: chrono::DateTime<chrono::Utc>, new_state: bool) -> Result<(), Box<dyn Error>>;
    /// retrieves the online state history for the given element
    fn get_online_state_history(&self, element: &str) -> Result<OnlineStateHistory, Box<dyn Error>>;
    // /// returns a list of all historized elements
    // fn all_historized_elements(&self) -> Vec<String>;
    // /// retrieves all elements' attributes' histories
    // fn all_attribute_histories(&self) -> HashMap<String, HashMap<String, PropertyHistory>> {
    //     self.all_historized_elements()
    //         .into_iter()
    //         .map(|element_id| {
    //             let attribute_histories = self.all_component_attribute_histories(&element_id);
    //             (element_id, attribute_histories)
    //         })
    //         .collect()
    // }
    // /// retrieves all elements' online-states' histories
    // fn all_online_state_histories(&self) -> HashMap<String, OnlineStateHistory> {
    //     self.all_historized_elements()
    //         .into_iter()
    //         .map(|element_id| {
    //             let online_history = self.get_online_state_history(&element_id);
    //             (element_id, online_history)
    //         })
    //         .collect()
    // }
    // /// retrieves all histories
    // fn all_histories(&self) -> HashMap<String, (OnlineStateHistory, HashMap<String, PropertyHistory>)> {
    //     self.all_historized_elements()
    //         .into_iter()
    //         .map(|element_id| {
    //             let online_history = self.get_online_state_history(&element_id);
    //             let attribute_history = self.all_component_attribute_histories(&element_id);
    //             (element_id, (online_history, attribute_history))
    //         })
    //         .collect()
    // }
}