use std::collections::HashMap;
use argon2::PasswordVerifier;
use bytecode::ByteCode;
use parking_lot::RwLock;
use utils::Never;
use server::ComponentHandle;
use crate::auth::User;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
    roles: Vec<String>,
    #[serde(default)]
    attributes: HashMap<String, ByteCode>,
}
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Config {
    #[serde(alias="user")]
    users: Vec<ConfigUser>,
    #[serde(with="utils::duration_parsing")]
    session_duration: chrono::Duration,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
struct LoadedUser {
    password: Password,
    roles: Vec<String>,
    attributes: HashMap<String, ByteCode>,
}

#[derive(Debug)]
pub struct ConfigAuthBackend {
    users: HashMap<String, LoadedUser>,
    sessions: RwLock<HashMap<String, (chrono::DateTime<chrono::Utc>, String)>>,
    max_session_age: chrono::Duration,
}
#[async_trait::async_trait]
impl super::AuthBackend for ConfigAuthBackend {
    type Config = Config;
    type ConfigError = Never;

    fn init(config: Self::Config, _server: &ComponentHandle) -> Result<Self, Self::ConfigError> {
        Ok(Self {
            users: config.users.into_iter()
                .map(|u| (u.username, LoadedUser {
                    password: u.password,
                    roles: u.roles,
                    attributes: u.attributes,
                }))
                .collect(),
            sessions: RwLock::new(HashMap::new()),
            max_session_age: config.session_duration
        })
    }

    fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::ConfigError> {
        self.users = config.users.into_iter()
            .map(|u| (u.username, LoadedUser {
                password: u.password,
                roles: u.roles,
                attributes: u.attributes,
            }))
            .collect();
        self.max_session_age = config.session_duration;
        Ok(())
    }

    type AccessError = argon2::password_hash::Error;

    async fn login(&self, username: &str, password: &str) -> Result<Option<String>, Self::AccessError> {
        let Some(user) = self.users.get(username) else { return Ok(None) };
        if !user.password.verify(password)? {
            return Ok(None)
        }
        let session_id = super::generate_session_id();
        self.sessions.write().insert(session_id.clone(), (chrono::Utc::now(), username.to_string()));
        Ok(Some(session_id))
    }

    async fn get_user(&self, session_id: &str) -> Result<Option<User>, Self::AccessError> {
        let lock = self.sessions.read();
        let Some((from, username)) = lock.get(session_id) else {
            return Ok(None);
        };
        if (*from - chrono::Utc::now()) > self.max_session_age {
            drop(lock);
            self.sessions.write().remove(session_id);
            return Ok(None);
        }
        let Some(user) = self.users.get(username) else {
            error!("invalid user for session!");
            return Ok(None);
        };
        Ok(Some(User {
            username: username.clone(),
            roles: user.roles.clone(),
            attributes: user.attributes.clone(),
        }))
    }

    async fn get_roles(&self, session_id: &str) -> Result<Option<Vec<String>>, Self::AccessError> {
        let lock = self.sessions.read();
        let Some((from, username)) = lock.get(session_id) else {
            return Ok(None);
        };
        if (*from - chrono::Utc::now()) > self.max_session_age {
            drop(lock);
            self.sessions.write().remove(session_id);
            return Ok(None);
        }
        let Some(user) = self.users.get(username) else {
            error!("invalid user for session!");
            return Ok(None);
        };
        Ok(Some(user.roles.clone()))
    }

    async fn get_attribute(&self, session_id: &str, attribute: &str) -> Result<Option<ByteCode>, Self::AccessError> {
        let lock = self.sessions.read();
        let Some((from, username)) = lock.get(session_id) else {
            return Ok(None);
        };
        if (*from - chrono::Utc::now()) > self.max_session_age {
            drop(lock);
            self.sessions.write().remove(session_id);
            return Ok(None);
        }
        let Some(user) = self.users.get(username) else {
            error!("invalid user for session!");
            return Ok(None);
        };
        Ok(user.attributes.get(attribute).cloned())
    }
}