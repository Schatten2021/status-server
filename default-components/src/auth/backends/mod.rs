use rand::distr::SampleString;
use utils::featured_use;
use server::ComponentHandle;
use super::type_defs::*;


pub const SESSION_ID_LENGTH: usize = 256;
pub fn generate_session_id() -> String {
    rand::distr::Alphanumeric.sample_string(&mut rand::rng(), SESSION_ID_LENGTH)
}

pub trait Backend: Sized {
    type Config: for<'de> serde::Deserialize<'de> + Default;
    type ConfigError: core::error::Error;
    fn init(config: Self::Config, server: &ComponentHandle) -> Result<Self, Self::ConfigError>;
    fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::ConfigError>;

    type AccessError: std::error::Error;
    type LoginError: std::error::Error;
    fn try_login(&self, username: &str, password: &str) -> Result<Option<SessionId>, Self::LoginError>;
    fn user_id_from_session(&self, session_id: &SessionId) -> Result<Option<UserId>, Self::AccessError>;
    fn user_roles(&self, user_id: &UserId) -> Result<Option<Vec<RoleId>>, Self::AccessError>;
    fn has_role(&self, user_id: &UserId, role: &RoleId) -> Result<Option<bool>, Self::AccessError> {
        Ok(self.user_roles(user_id)?
            .map(|roles| roles.contains(role)))
    }
    fn user_attributes(&self, user_id: &UserId) -> Result<Option<AttributeMap>, Self::AccessError>;
    fn user_get_attribute(&self, user_id: &UserId, attribute_id: &String) -> Result<Option<bytecode::ByteCode>, Self::AccessError> {
        Ok(match self.user_attributes(user_id)? {
            // Note: using remove to move the value out of the map directly so that it doesn't need
            // to be cloned.
            Some(mut v) => v.remove(attribute_id),
            None => None,
        })
    }

    fn role_users(&self, role_id: &RoleId) -> Result<Vec<UserId>, Self::AccessError>;
    fn role_attributes(&self, role_id: &RoleId) -> Result<Option<AttributeMap>, Self::AccessError>;
    fn role_get_attribute(&self, role_id: &RoleId, attribute_id: &String) -> Result<Option<bytecode::ByteCode>, Self::AccessError>;

}

featured_use!(if "auth-config-backend": config::ConfigAuthBackend);