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
    Hashed(String),
}
impl Default for Password {
    fn default() -> Self {
        Self::Plaintext("".to_string())
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
    users: Vec<ConfigUser>,
    #[serde(alias="role", default)]
    roles: HashMap<String, Role>,
    #[serde(with="utils::duration_parsing", default="day")]
    session_duration: chrono::Duration,
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
