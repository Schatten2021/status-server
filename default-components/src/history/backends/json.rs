use crate::history::{OnlineStateHistory, PropertyHistory};
use server::ComponentHandle;
use std::error::Error;
use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::path::PathBuf;
use utils::Never;
const SLASH_REPLACEMENT_TOKEN: &str = "ThisTokenWillNeverExistInTheWild";
fn attr_id_to_path(id: &str) -> String {
    id.replace("/", SLASH_REPLACEMENT_TOKEN)
        .replace(".","/")
        .replace(SLASH_REPLACEMENT_TOKEN, ".")
}
// fn path_to_attr_id(path: &str) -> String {
//     path.replace(".", SLASH_REPLACEMENT_TOKEN)
//         .replace("/", ".")
//         .replace(SLASH_REPLACEMENT_TOKEN, "/")
// }
fn default_path() -> PathBuf { "history/json".into() }
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Config {
    #[serde(default="default_path")]
    base_path: PathBuf,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            base_path: default_path(),
        }
    }
}

pub struct FsJsonBackend {
    config: Config,
}
impl FsJsonBackend {
    fn element_path(&self, element: &str) -> PathBuf {
        self.config.base_path.join(element)
    }
    fn open_file(path: PathBuf, write: bool) -> std::io::Result<File> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?
            }
        }
        let file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        if write {
            file.lock()?
        } else {
            file.lock_shared()?
        }
        Ok(file)
    }
    fn open_attribute(&self, element: &str, attribute: &str, write: bool) -> std::io::Result<File> {
        let attr_path = attr_id_to_path(attribute);
        let path = self.element_path(element)
            .join(attr_path)
            .join("attribute.json");
        Self::open_file(path, write)
    }
    fn open_online_state(&self, element: &str, write: bool) -> std::io::Result<File> {
        let path = self.element_path(element)
            .join("online.json");
        Self::open_file(path, write)
    }
}
impl super::Backend for FsJsonBackend {
    type Config = Config;
    type ConfigError = Never;

    fn init(_server: ComponentHandle, config: Self::Config) -> Result<Self, Self::ConfigError> {
        Ok(Self { config })
    }

    fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::ConfigError> {
        self.config = config;
        Ok(())
    }

    fn add_attribute_change(&self, element: &str, attribute: &str, timestamp: chrono::DateTime<chrono::Utc>, value: Option<&server::AttributeValue>) -> Result<(), Box<dyn Error>> {
        let mut file = self.open_attribute(element, attribute, true)?;
        let file_len = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(0))?;
        let mut old: PropertyHistory = if file_len == 0 { Vec::new() } else {serde_json::from_reader(&mut file)? };
        old.push((timestamp, value.cloned()));
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        serde_json::to_writer(&mut file, &old)?;
        trace!("saved attribute change to JSON filesystem history backend.");
        Ok(())
    }

    fn get_attribute_history(&self, element: &str, attribute: &str) -> Result<PropertyHistory, Box<dyn Error>> {
        let mut file = self.open_attribute(element, attribute, false)?;
        Ok(serde_json::from_reader(&mut file)?)
    }

    fn add_online_state_change(&self, element: &str, timestamp: chrono::DateTime<chrono::Utc>, new_state: bool) -> Result<(), Box<dyn Error>> {
        let mut file = self.open_online_state(element, true)?;
        let file_len = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(0))?;
        let mut current: OnlineStateHistory = if file_len == 0 { Vec::new() } else {serde_json::from_reader(&mut file)? };
        current.push((timestamp, new_state));
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        serde_json::to_writer(&mut file, &current)?;
        trace!("saved online state change to JSON filesystem history backend.");
        Ok(())
    }

    fn get_online_state_history(&self, element: &str) -> Result<OnlineStateHistory, Box<dyn Error>> {
        Ok(serde_json::from_reader(self.open_online_state(element, false)?)?)
    }
}