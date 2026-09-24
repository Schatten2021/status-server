use argon2::PasswordVerifier;
use bytecode::ByteCode;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use utils::Never;
use server::ComponentHandle;
use crate::auth::{RoleId, SessionId, UserId};
use crate::auth::type_defs::AttributeMap;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all="lowercase")]
pub enum Password {
    Plaintext(String),
    #[serde(alias="hash")]
    Hashed(String),
}
impl Default for Password {
    fn default() -> Self {
        Self::Plaintext(String::new())
    }
}
impl Password {
    pub fn verify(&self, password: &str) -> Result<bool, argon2::password_hash::Error> {
        match self {
            Password::Plaintext(inner) => Ok(inner == password),
            Password::Hashed(hash) => {
                let hash = argon2::password_hash::phc::PasswordHash::new(hash)?;
                let verifier = argon2::Argon2::default();
                match verifier.verify_password(password.as_bytes(), &hash) {
                    Ok(()) => Ok(true),
                    Err(argon2::password_hash::Error::PasswordInvalid) => Ok(false),
                    Err(e) => Err(e),
                }
            }
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ConfigUser {
    username: String,
    password: Password,
    #[serde(default)]
    roles: HashSet<String>,
    #[serde(default)]
    attributes: HashMap<String, ByteCode>,
}
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Role {
    #[serde(default)]
    attributes: HashMap<String, ByteCode>,
}
fn day() -> chrono::Duration { chrono::Duration::days(1) }
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Config {
    #[serde(alias="user", default)]
    pub users: Vec<ConfigUser>,
    #[serde(alias="role", default)]
    pub roles: HashMap<String, Role>,
    #[serde(with="utils::duration_parsing", default="day")]
    pub session_duration: chrono::Duration,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
struct LoadedUser {
    password: Password,
    roles: HashSet<String>,
    attributes: HashMap<String, ByteCode>,
}
#[derive(Debug)]
pub struct ConfigAuthBackend {
    users: HashMap<String, LoadedUser>,
    roles: HashMap<String, Role>,
    sessions: RwLock<HashMap<String, (chrono::DateTime<chrono::Utc>, String)>>,
    max_session_age: chrono::Duration,
}
macro_rules! tri {
    ($val:expr) => {match $val {
        Some(v) => v,
        None => return Ok(None),
    }};
}
impl super::Backend for ConfigAuthBackend {
    type Config = Config;
    type ConfigError = Never;

    fn init(config: Self::Config, _server: &ComponentHandle) -> Result<Self, Self::ConfigError> {
        Ok(Self {
            users: config.users.into_iter()
                .map(|user| (user.username, LoadedUser {
                    password: user.password,
                    roles: user.roles,
                    attributes: user.attributes,
                }))
                .collect(),
            roles: config.roles,
            sessions: RwLock::new(HashMap::new()),
            max_session_age: config.session_duration,
        })
    }

    fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::ConfigError> {
        self.users = config.users.into_iter()
            .map(|user| (user.username, LoadedUser {
                password: user.password,
                roles: user.roles,
                attributes: user.attributes,
            }))
            .collect();
        self.roles = config.roles;
        self.max_session_age = config.session_duration;
        Ok(())
    }

    type AccessError = Never;
    type LoginError = argon2::password_hash::Error;
    fn try_login(&self, username: &str, password: &str) -> Result<Option<SessionId>, Self::LoginError> {
        let user = tri!(self.users.get(username));
        if !user.password.verify(password)? { return Ok(None) }
        let session_id = super::generate_session_id();
        self.sessions.write().insert(session_id.clone(), (chrono::Utc::now(), username.to_string()));
        Ok(Some(session_id))
    }

    fn user_id_from_session(&self, session_id: &SessionId) -> Result<Option<UserId>, Self::AccessError> {
        let lock = self.sessions.read();
        let (creation, user) = tri!(lock.get(session_id));
        if chrono::Utc::now() - creation > self.max_session_age {
            drop(lock);
            self.sessions.write().remove(session_id);
            return Ok(None);
        }
        Ok(Some(user.clone()))
    }

    fn user_roles(&self, user_id: &UserId) -> Result<Option<Vec<RoleId>>, Self::AccessError> {
        let user = tri!(self.users.get(user_id));
        Ok(Some(user.roles.iter().cloned().collect()))
    }

    fn has_role(&self, user_id: &UserId, role: &RoleId) -> Result<Option<bool>, Self::AccessError> {
        let user = tri!(self.users.get(user_id));
        Ok(Some(user.roles.contains(role)))
    }

    fn user_attributes(&self, user_id: &UserId) -> Result<Option<AttributeMap>, Self::AccessError> {
        Ok(self.users.get(user_id).map(|u| u.attributes.clone()))
    }

    fn user_get_attribute(&self, user_id: &UserId, attribute_id: &String) -> Result<Option<ByteCode>, Self::AccessError> {
        Ok(match self.users.get(user_id) {
            Some(user) => user.attributes.get(attribute_id).cloned(),
            None => None,
        })
    }

    fn role_users(&self, role_id: &RoleId) -> Result<Vec<UserId>, Self::AccessError> {
        Ok(self.users.iter()
            .filter(|(_id, user)| user.roles.contains(role_id))
            .map(|(id, _)| id.clone())
            .collect()
        )
    }

    fn role_attributes(&self, role_id: &RoleId) -> Result<Option<AttributeMap>, Self::AccessError> {
        Ok(self.roles.get(role_id).map(|r| r.attributes.clone()))
    }

    fn role_get_attribute(&self, role_id: &RoleId, attribute_id: &String) -> Result<Option<ByteCode>, Self::AccessError> {
        Ok(tri!(self.roles.get(role_id))
            .attributes.get(attribute_id).cloned()
        )
    }
}

#[cfg(test)]
mod test {
    mod parsing {
        use super::super::*;
        use crate::parse_test;
        parse_test!(empty(Config): toml::Table::new() => Config {
            users: vec![],
            roles: HashMap::new(),
            session_duration: day(),
        });
        parse_test!(user_empty(ConfigUser): toml::Table::new() => error);
        parse_test!(user_minimal_plain(ConfigUser): toml!{
            username = "foo"
            password.plaintext = "bar"
        } => ConfigUser {
            username: "foo".to_string(),
            password: Password::Plaintext("bar".to_string()),
            roles: HashSet::new(),
            attributes: HashMap::new(),
        });
        parse_test!(user_minimal_hashed(ConfigUser): toml!{
            username = "foo"
            password.hash = "bar"
        } => ConfigUser {
            username: "foo".to_string(),
            password: Password::Hashed("bar".to_string()),
            roles: HashSet::new(),
            attributes: HashMap::new(),
        });
        parse_test!(user_maximum(ConfigUser): toml!{
            username = "foo"
            password.plaintext = "bar"
            roles = ["admin"]
            attributes.email = "foo@example.com"
        } => ConfigUser {
            username: "foo".to_string(),
            password: Password::Plaintext("bar".to_string()),
            roles: HashSet::from(["admin".to_string()]),
            attributes: HashMap::from([("email".to_string(), ByteCode::String("foo@example.com".to_string()))]),
        });
        parse_test!(role_emtpy(Role): toml::Table::new() => Role {
            attributes: HashMap::new(),
        });
        parse_test!(role_filled(Role): toml![attributes.ignores_api_rules = true] => Role {
            attributes: HashMap::from([("ignores_api_rules".to_string(), ByteCode::Bool(true))]),
        });
        parse_test!(with_user(Config): toml!{
            [[user]]
            username = "foo"
            password.plaintext = "bar"
        } => Config {
            users: vec![ConfigUser {
                username: "foo".to_string(),
                password: Password::Plaintext("bar".to_string()),
                roles: HashSet::new(),
                attributes: HashMap::new(),
            }],
            roles: HashMap::new(),
            session_duration: day(),
        });
        parse_test!(with_role(Config): toml!{
            [role.example]
        } => Config {
            users: vec![],
            roles: HashMap::from([("example".to_string(), Role::default())]),
            session_duration: day(),
        });
        parse_test!(with_session_duration(Config): toml!{session_duration = "1h"} => Config {
            users: vec![],
            roles: HashMap::new(),
            session_duration: chrono::Duration::hours(1),
        });
        parse_test!(full(Config): toml!{
            session_duration = "1h"
            [[user]]
            username = "foo"
            password.hash = "bar"
            roles = ["example"]
            [role.example]
            attributes.ignore_api_rules = true

        } => Config {
            users: vec![ConfigUser {
                username: "foo".to_string(),
                password: Password::Hashed("bar".to_string()),
                roles: HashSet::from(["example".to_string()]),
                attributes: HashMap::new(),
            }],
            roles: HashMap::from([("example".to_string(), Role {
                attributes: HashMap::from([("ignore_api_rules".to_string(), ByteCode::Bool(true))])
            })]),
            session_duration: chrono::Duration::hours(1),
        });
    }
}
