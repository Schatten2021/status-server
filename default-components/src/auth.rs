use rand::distr::SampleString;
use server::ComponentHandle;
use std::collections::HashMap;

pub const SESSION_ID_LENGTH: usize = 256;

#[derive(Clone, Debug, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct User {
    pub username: String,
    pub roles: Vec<String>,
    pub attributes: HashMap<String, bytecode::ByteCode>,
}
#[async_trait::async_trait]
pub trait AuthBackend: Sized {
    type Config: for<'de> serde::Deserialize<'de> + Default;
    type ConfigError: std::error::Error;
    fn init(config: Self::Config, server: &ComponentHandle) -> Result<Self, Self::ConfigError>;
    fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::ConfigError>;

    type AccessError: std::error::Error;
    fn login(&self, username: &str, password: &str) -> Result<Option<String>, Self::AccessError>;
    fn get_user(&self, session_id: &str) -> Result<Option<User>, Self::AccessError>;
    fn get_roles(&self, session_id: &str) -> Result<Option<Vec<String>>, Self::AccessError> {
        self.get_user(session_id).map(|v| v.map(|u| u.roles))
    }
    fn has_role(&self, session_id: &str, role: &str) -> Result<bool, Self::AccessError> {
        Ok(self.get_roles(session_id)?
            .is_some_and(|v| v.iter().any(|r| r == role)))
    }
    fn get_attribute(&self, session_id: &str, attribute: &str) -> Result<Option<bytecode::ByteCode>, Self::AccessError> {
        match self.get_user(session_id)? {
            Some(user) => Ok(user.attributes.get(attribute).cloned()),
            None => Ok(None)
        }
    }
}
pub fn generate_session_id() -> String {
    rand::distr::Alphanumeric.sample_string(&mut rand::rng(), SESSION_ID_LENGTH)
}
macro_rules! auth_component {
    (
        $(#[$struct_meta:meta])*
        pub struct $struct_name:ident {
            $(if $feature:literal($(#[$config_meta:meta])*$config_key:ident): $backend_field_name:ident: $backend_module:ident::$backend_type_name:ident);* $(;)?
        }
    ) => {
        $(#[cfg(feature=$feature)] mod $backend_module;)*
        #[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
        pub struct Config {$(
            #[cfg(feature=$feature)]
            $(#[$config_meta])*
            $config_key: <$backend_module::$backend_type_name as AuthBackend>::Config,
        ),*}
        #[derive(Debug, thiserror::Error)]
        pub enum ConfigError {$(
            #[error("{0}")]
            #[cfg(feature=$feature)]
            $backend_type_name(#[from] <$backend_module::$backend_type_name as AuthBackend>::ConfigError),
        )*}
        $(#[$struct_meta])*
        #[derive(Debug)]
        pub struct $struct_name {$(
            #[cfg(feature = $feature)]
            $backend_field_name: $backend_module::$backend_type_name,
        )*}
        impl ::server::Component for $struct_name {
            const ID: &'static str = "auth";
            type Config = Config;
            type ConfigError = ConfigError;
            fn init(server: ComponentHandle, config: Self::Config) -> Result<Self, Self::ConfigError> {
                Ok(Self {$(
                    #[cfg(feature=$feature)]
                    $backend_field_name: <$backend_module::$backend_type_name as AuthBackend>::init(config.$config_key, &server)?,
                )*})
            }
            fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::ConfigError> {
                $(
                #[cfg(feature=$feature)]
                if let Err(e) = self.$backend_field_name.reconfigure(config.$config_key) {
                    return Err(ConfigError::$backend_type_name(e));
                }
                )*
                Ok(())
            }
        }
        #[derive(Debug, thiserror::Error)]
        pub enum AccessError {
            $(#[cfg(feature=$feature)] #[error("{0}")] $backend_type_name(#[from] <$backend_module::$backend_type_name as AuthBackend>::AccessError),)*
        }
        impl $struct_name {
            /// Tries to log a user in.
            ///
            /// Returns `Ok(Some(session_id))` when the user was successfully logged in and
            /// `Ok(None)` if the user does not exist/has a different password.
            pub fn login(&self, username: &str, password: &str) -> Result<Option<String>, AccessError> {
                trace!("attempting to log in user {username}");
                $(
                #[cfg(feature=$feature)]
                if let Some(session_id) = self.$backend_field_name.login(username, password)? {
                    return Ok(Some(session_id));
                }
                )*
                Ok(None)
            }
            /// Returns the user belonging to the session.
            pub fn get_user(&self, session_id: &str) -> Result<Option<User>, AccessError> {
                $(
                #[cfg(feature=$feature)]
                if let Some(session_id) = self.$backend_field_name.get_user(session_id)? {
                    return Ok(Some(session_id));
                }
                )*
                Ok(None)
            }
            /// returns the roles of the user belonging to the session.
            pub fn get_roles(&self, session_id: &str) -> Result<Option<Vec<String>>, AccessError> {
                $(
                #[cfg(feature=$feature)]
                if let Some(session_id) = self.$backend_field_name.get_roles(session_id)? {
                    return Ok(Some(session_id));
                }
                )*
                Ok(None)
            }
            /// checks whether the user has a given role.
            pub fn has_role(&self, session_id: &str, role: &str) -> Result<bool, AccessError> {
                $(
                #[cfg(feature=$feature)]
                if self.$backend_field_name.has_role(session_id, role)? {
                    return Ok(true)
                }
                )*
                Ok(false)
            }
            /// Returns the given attribute of the user to whom the session belongs to.
            pub fn get_attribute(&self, session_id: &str, attribute_id: &str) -> Result<Option<bytecode::ByteCode>, AccessError> {
                $(
                #[cfg(feature=$feature)]
                if let Some(session_id) = self.$backend_field_name.get_attribute(session_id, attribute_id)? {
                    return Ok(Some(session_id));
                }
                )*
                Ok(None)
            }
        }
    };
}
auth_component!(
    /// [`server::Component`] for providing authentication services to other components.
    pub struct Auth {
        if "auth-config-backend"(#[serde(flatten)] config): config: config::ConfigAuthBackend;
    }
);