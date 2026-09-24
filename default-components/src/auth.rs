pub mod backends;
mod macros;
mod type_defs;

use std::sync::Arc;
use parking_lot::RwLock;
#[allow(unused_imports, reason="these are intended for outside use.")]
pub use type_defs::{
    RoleId,
    UserId,
    SessionId,
    AttributeId,
    AttributeMap,
    ConfigError, LoginError, AccessError,
};
use type_defs::AuthServer;
use backends::Backend;
use server::ComponentHandle;
// NOTE (plans):
// - users identified by their username.
// - roles identified by their name.
// - users can have attributes
// - roles can have attributes
//
// Attributes for a user:
// - username
// - password (backend managed; not exposed)
// - attributes (key-value map)
// - roles
// Attributes for a role:
// - Name (ID)
// - attributes (key-value map)
// - users

#[derive(Clone, Debug)]
/// [`server::Component`] providing authentication & user management for other components to use.
pub struct Auth(Arc<RwLock<AuthServer>>);
impl server::Component for Auth {
    const ID: &'static str = "auth";
    type Config = type_defs::Config;
    type ConfigError = ConfigError;

    fn init(server: ComponentHandle, config: Self::Config) -> Result<Self, Self::ConfigError> {
        Ok(Self(Arc::new(RwLock::new(AuthServer::new(server, config)?))))
    }

    fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::ConfigError> {
        self.0.write().reconfigure(config)
    }
}
macro_rules! forwarded_impls {
    ($(#[$meta:meta])*fn $func:ident(&self $(, $arg_name:ident: $arg_ty:ty)*) -> $ret_ty:ty) => {
        $(#[$meta])*
        pub fn $func(&self $(, $arg_name: $arg_ty)*) -> $ret_ty {
            self.0.read().$func($($arg_name),*)
        }
    };
    ($($(#[$meta:meta])*fn $func:ident(&self $(, $arg_name:ident: $arg_ty:ty)*) -> $ret_ty:ty);* $(;)?) => {
        $(forwarded_impls!($(#[$meta])*fn $func(&self $(, $arg_name: $arg_ty)*) -> $ret_ty);)*
    }
}
impl Auth {
    forwarded_impls!(
        /// attempts to log the user in, returning the generated session-id if successful.
        fn try_login(&self, username: &str, password: &str) -> Result<Option<SessionId>, LoginError>;
        /// Returns the UserId from the SessionId.
        fn user_id_from_session(&self, session_id: &SessionId) -> Result<Option<UserId>, AccessError>;
        /// Retrieves the users roles.
        fn user_roles(&self, user_id: &UserId) -> Result<Option<Vec<RoleId>>, AccessError>;
        /// Checks whether the user has a specific role.
        fn has_role(&self, user_id: &UserId, role: &RoleId) -> Result<Option<bool>, AccessError>;
        /// Retrieves all attributes of a user.
        fn user_attributes(&self, user_id: &UserId) -> Result<Option<AttributeMap>, AccessError>;
        /// Returns a specific attribute of a user.
        fn user_get_attribute(&self, user_id: &UserId, attribute_id: &String) -> Result<Option<bytecode::ByteCode>, AccessError>;
        /// Returns all users belonging to a specific role.
        fn role_users(&self, role_id: &RoleId) -> Result<Vec<UserId>, AccessError>;
        /// Returns all attributes of a role.
        fn role_attributes(&self, role_id: &RoleId) -> Result<Option<AttributeMap>, AccessError>;
        /// Returns a specific attribute of a role.
        fn role_get_attribute(&self, role_id: &RoleId, attribute_id: &String) -> Result<Option<bytecode::ByteCode>, AccessError>;
    );
}
