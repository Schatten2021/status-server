use crate::history::{OnlineStateHistory, PropertyHistory};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, Error, OptionalExtension};
use server::{AttributeValue, ComponentHandle};
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;


fn default_path() -> PathBuf { "history.sqlite3".into() }

mod mode_defaults {
    pub mod standard {
        pub fn element_lookup_table() -> String {
            "elements".to_string()
        }
        pub fn attribute_lookup_table() -> String {
            "attributes".to_string()
        }
        pub fn online_state_changes_table() -> String {
            "online_state_changes".to_string()
        }
        pub fn attribute_changes_table() -> String {
            "attribute_changes".to_string()
        }
    }
}
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Mode {
    Standard {
        #[serde(default="mode_defaults::standard::element_lookup_table")]
        element_lookup_table: String,
        #[serde(default="mode_defaults::standard::attribute_lookup_table")]
        attribute_lookup_table: String,
        #[serde(default="mode_defaults::standard::online_state_changes_table")]
        online_state_changes_table: String,
        #[serde(default="mode_defaults::standard::attribute_changes_table")]
        attribute_changes_table: String,
    }
}
impl Default for Mode {
    fn default() -> Self {
        Self::Standard {
            element_lookup_table: mode_defaults::standard::element_lookup_table(),
            attribute_lookup_table: mode_defaults::standard::attribute_lookup_table(),
            online_state_changes_table: mode_defaults::standard::online_state_changes_table(),
            attribute_changes_table: mode_defaults::standard::attribute_changes_table(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Config {
    #[serde(default="default_path")]
    path: PathBuf,
    #[serde(default)]
    mode: Mode,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            path: default_path(),
            mode: Mode::default(),
        }
    }
}

pub struct SqliteBackend {
    connection: Arc<Mutex<Connection>>,
    mode: Mode,
}
impl SqliteBackend {
    fn conn(&self) -> impl Deref<Target=Connection> {
        self.connection.lock().unwrap()
    }
    fn init_db(&self) -> Result<(), Error> {
        match &self.mode {
            Mode::Standard {
                element_lookup_table,
                attribute_lookup_table,
                online_state_changes_table,
                attribute_changes_table
            } => {
                // TODO: change it so that the columns are created if they don't exist.
                self.conn().execute(&format!(r#"CREATE TABLE IF NOT EXISTS {element_lookup_table} (
    id INTEGER NOT NULL CONSTRAINT element_pk PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE
)"#, ), [])?;
                self.conn().execute(&format!(r#"CREATE TABLE IF NOT EXISTS {attribute_lookup_table} (
    id INTEGER NOT NULL CONSTRAINT attribute_pk PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE
)"#, ), [])?;
                self.conn().execute(&format!(r#"CREATE TABLE IF NOT EXISTS {online_state_changes_table} (
    id INTEGER NOT NULL CONSTRAINT online_state_change_pk PRIMARY KEY AUTOINCREMENT,
    element_id INTEGER NOT NULL CONSTRAINT online_state_change_element_fk REFERENCES {element_lookup_table},
    state INTEGER NOT NULL,
    timestamp_secs INTEGER,
    timestamp_subsec_nanos INTEGER
)"#), [])?;
                self.conn().execute(&format!(r#"CREATE TABLE IF NOT EXISTS {attribute_changes_table} (
    id INTEGER NOT NULL CONSTRAINT attribute_change_pk PRIMARY KEY AUTOINCREMENT,
    element_id INTEGER NOT NULL CONSTRAINT attribute_change_element_fk REFERENCES {element_lookup_table},
    attribute_id INTEGER NOT NULL CONSTRAINT attribute_change_attribute_fk REFERENCES {attribute_lookup_table},
    val TEXT,
    timestamp_secs INTEGER,
    timestamp_subsec_nanos INTEGER
)"#), [])?;
            }
        }
        debug!("initialized database");
        Ok(())
    }
    fn try_get_element_id(&self, element: &str) -> Result<Option<i64>, Error> {
        match &self.mode {
            Mode::Standard { element_lookup_table, .. } => {
                self.conn().query_row(&format!(r#"SELECT * FROM {element_lookup_table} WHERE name=$1"#),
                                          [element],
                                          |row| row.get("id")
                ).optional()
            }
        }
    }
    fn get_element_id(&self, element: &str) -> Result<i64, Error> {
        if let Some(id) = self.try_get_element_id(element)? { return Ok(id); }
        match &self.mode {
            Mode::Standard { element_lookup_table, .. } => {
                self.conn().query_row(&format!(r#"INSERT INTO {element_lookup_table}(name) VALUES ($1) RETURNING *;"#),
                                          [element],
                                          |row| row.get("id")
                )
            }
        }
    }
    fn try_get_attribute_id(&self, attribute: &str) -> Result<Option<i64>, Error> {
        match &self.mode {
            Mode::Standard { attribute_lookup_table, .. } => self.conn().query_row(
                &format!(r#"SELECT * FROM {attribute_lookup_table} WHERE name=$1"#),
                [attribute],
                |row| row.get("id")
            ).optional()
        }
    }
    fn get_attribute_id(&self, attribute: &str) -> Result<i64, Error> {
        if let Some(id) = self.try_get_attribute_id(attribute)? { return Ok(id) }
        match &self.mode {
            Mode::Standard { attribute_lookup_table, .. } => self.conn().query_row(
                &format!(r#"INSERT INTO {attribute_lookup_table}(name) VALUES ($1) RETURNING *;"#),
                [attribute],
                |row| row.get("id")
            )
        }
    }
}
impl super::Backend for SqliteBackend {
    type Config = Config;
    type ConfigError = Error;

    fn init(_server: ComponentHandle, config: Self::Config) -> Result<Self, Self::ConfigError> {
        let connection = Connection::open(config.path)?;
        let this = Self {
            connection: Arc::new(Mutex::new(connection)),
            mode: config.mode,
        };
        this.init_db()?;
        Ok(this)
    }

    fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::ConfigError> {
        *self.connection.lock().unwrap() = Connection::open(config.path)?;
        self.mode = config.mode;
        self.init_db()?;
        Ok(())
    }

    fn add_attribute_change(&self, element: &str, attribute: &str, timestamp: DateTime<Utc>, new_val: Option<&AttributeValue>) -> Result<(), Box<dyn std::error::Error>> {
        if attribute == "minecraft.last_seen" || attribute == "website.last_seen" {
            error!("saw a last_seen!")
        }
        let attribute_id = self.get_attribute_id(attribute)?;
        let element_id = self.get_element_id(element)?;
        let timestamp_secs = timestamp.timestamp();
        let timestamp_subsec_nanos = timestamp.timestamp_subsec_nanos();
        let serialized = new_val.map(|val| serde_json::to_string(val).expect("unable to serialize attribute value"));
        match &self.mode {
            Mode::Standard { attribute_changes_table, .. } => {
                self.conn().execute(
                    &format!(r#"INSERT INTO {attribute_changes_table}(element_id, attribute_id, val, timestamp_secs, timestamp_subsec_nanos) VALUES ($1, $2, $3, $4, $5)"#),
                    (element_id, attribute_id, serialized, timestamp_secs, timestamp_subsec_nanos),
                )?;
            }
        }
        debug!("inserted attribute change into sqlite db");
        Ok(())
    }

    fn get_attribute_history(&self, element: &str, attribute: &str) -> Result<PropertyHistory, Box<dyn std::error::Error>> {
        match &self.mode {
            Mode::Standard { attribute_changes_table, .. } => {
                let conn = self.conn();
                let Some(element_id) = self.try_get_element_id(element)? else { return Ok(vec![]) };
                let Some(attribute_id) = self.try_get_attribute_id(attribute)? else { return Ok(vec![]) };
                let mut statement = conn.prepare(&format!("SELECT * FROM {attribute_changes_table} WHERE element_id=$2 AND attribute_id=$3;"))?;
                Ok(statement.query_map((element_id, attribute_id),
                                    |row| {
                                        let value: Option<String> = row.get("val")?;
                                        let value: Option<AttributeValue> = value.map(|serialized| serde_json::from_str(&serialized)
                                            .expect("invalid value in database"));
                                        let timestamp_secs: i64 = row.get("timestamp_secs")?;
                                        let timestamp_subsec_millis: i64 = row.get("timestamp_subsec_millis")?;
                                        let timestamp_subsec_millis = timestamp_subsec_millis as u32;
                                        Ok((DateTime::from_timestamp(timestamp_secs, timestamp_subsec_millis).expect("invalid timestamp"), value))
                                    }
                )?.collect::<Result<Vec<_>, Error>>()?)
            }
        }
    }

    fn add_online_state_change(&self, element: &str, timestamp: DateTime<Utc>, new_state: bool) -> Result<(), Box<dyn std::error::Error>> {
        let element_id = self.get_element_id(element)?;
        let timestamp_secs = timestamp.timestamp();
        let timestamp_subsec_nanos = timestamp.timestamp_subsec_nanos();
        match &self.mode {
            Mode::Standard { online_state_changes_table, .. } => {
                self.conn().execute(
                    &format!(r#"INSERT INTO {online_state_changes_table}(element_id, state, timestamp_secs, timestamp_subsec_nanos) VALUES ($2, $3, $4, $5)"#),
                    (element_id, new_state, timestamp_secs, timestamp_subsec_nanos),
                )?;
            }
        }
        debug!("inserted online state change into sqlite db");
        Ok(())
    }

    fn get_online_state_history(&self, element: &str) -> Result<OnlineStateHistory, Box<dyn std::error::Error>> {
        match &self.mode {
            Mode::Standard { online_state_changes_table, .. } => {
                let conn = self.conn();
                let Some(element_id) = self.try_get_element_id(element)? else { return Ok(vec![]) };
                let mut statement = conn.prepare(&format!("SELECT * FROM {online_state_changes_table} WHERE element_id=$2;"))?;
                Ok(statement.query_map([element_id],
                                       |row| {
                                           let value: bool = row.get("state")?;
                                           let timestamp_secs: i64 = row.get("timestamp_secs")?;
                                           let timestamp_subsec_millis: i64 = row.get("timestamp_subsec_millis")?;
                                           let timestamp_subsec_millis = timestamp_subsec_millis as u32;
                                           Ok((DateTime::from_timestamp(timestamp_secs, timestamp_subsec_millis).expect("invalid timestamp"), value))
                                       }
                )?.collect::<Result<Vec<_>, Error>>()?)
            }
        }
    }
}
