macro_rules! config_struct {
    ($(#[$struct_meta:meta])* pub struct $struct_name:ident {
        $($(#[$field_meta:meta])*if $feature:literal: $field_name:ident: $backend_name:ident),* $(,)?
    }) => {
        #[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
        $(#[$struct_meta])*
        pub struct Config {
            $(#[cfg(feature=$feature)] $(#[$field_meta])* pub  $field_name: <$crate::auth::backends::$backend_name as $crate::auth::Backend>::Config,)*
        }
    };
}
pub(super) use config_struct;
macro_rules! error_type {
    ($(#[$error_meta:meta])* pub enum $type_name:ident {
        $($(#[$field_meta:meta])* if $feature:literal: $source_struct_name:ident::$attr:ident),* $(,)?
    }) => {
        #[derive(Debug, thiserror::Error)]
        pub enum $type_name {$(
            #[cfg(feature=$feature)]
            $(#[$field_meta])*
            #[error("{0}")]
            $source_struct_name(#[from] <$crate::auth::backends::$source_struct_name as $crate::auth::Backend>::$attr)
        )*}
    };
}
pub(super) use error_type;
macro_rules! main_struct {
    (
        $(#[$struct_meta:meta])*
        pub struct $struct_name:ident {
            config: $config_type:ty,
            config_err: $config_error_type:ty,
            access_err: $access_error_type:ty,
            login_err: $login_error_type:ty,
            $(
                $(#[$field_attr:meta])*
                if $feature:literal($config_key:ident): $field_name:ident: $backend:ident
            ),* $(,)?
        }
    ) => {
        $(#[$struct_meta])*
        pub struct $struct_name {
            $(
            #[cfg(feature=$feature)] $(#[$field_attr])*
            $field_name: $crate::auth::backends::$backend
            )*
        }
        impl $struct_name {
            pub(crate) fn new(server: ::server::ComponentHandle, config: $config_type) -> Result<Self, $config_error_type> {
                Ok(Self {$(
                   #[cfg(feature=$feature)]
                   $field_name: match <$crate::auth::backends::$backend as $crate::auth::Backend>::init(config.$config_key, &server) {
                       Ok(v) => v,
                       Err(e) => return Err(<$config_error_type>::$backend(e)),
                   },
                )*})
            }
            pub(crate) fn reconfigure(&mut self, config: $config_type) -> Result<(), $config_error_type> {
                $(
                #[cfg(feature=$feature)]
                if let Err(e) = <$crate::auth::backends::$backend as $crate::auth::Backend>::reconfigure(&mut self.$field_name, config.$config_key) {
                    return Err(<$config_error_type>::$backend(e));
                }
                )*
                Ok(())
            }
            pub(crate) fn try_login(&self, username: &str, password: &str) -> Result<Option<String>, $login_error_type> {
                $(
                $crate::auth::macros::main_struct!(@internal:func_call if $feature: $backend::try_login(&self.$field_name, username, password) -> $login_error_type);
                )*
                Ok(None)
            }
            pub(crate) fn user_id_from_session(&self, session_id: &SessionId) -> Result<Option<UserId>, $access_error_type> {
                $(
                $crate::auth::macros::main_struct!(@internal:func_call if $feature: $backend::user_id_from_session(&self.$field_name, session_id) -> $access_error_type);
                )*
                Ok(None)
            }
            pub(crate) fn user_roles(&self, user_id: &UserId) -> Result<Option<Vec<RoleId>>, $access_error_type> {
                $(
                $crate::auth::macros::main_struct!(@internal:func_call if $feature: $backend::user_roles(&self.$field_name, user_id) -> $access_error_type);
                )*
                Ok(None)
            }
            pub(crate) fn has_role(&self, user_id: &UserId, role: &RoleId) -> Result<Option<bool>, $access_error_type> {
                $(
                $crate::auth::macros::main_struct!(@internal:func_call if $feature: $backend::has_role(&self.$field_name, user_id, role) -> $access_error_type);
                )*
                Ok(None)
            }
            pub(crate) fn user_attributes(&self, user_id: &UserId) -> Result<Option<AttributeMap>, $access_error_type> {
                $($crate::auth::macros::main_struct!(@internal:func_call if $feature: $backend::user_attributes(&self.$field_name, user_id) -> $access_error_type);)*
                Ok(None)
            }
            pub(crate) fn user_get_attribute(&self, user_id: &UserId, attribute_id: &AttributeId) -> Result<Option<::bytecode::ByteCode>, $access_error_type> {
                $($crate::auth::macros::main_struct!(@internal:func_call if $feature: $backend::user_get_attribute(&self.$field_name, user_id, attribute_id) -> $access_error_type);)*
                Ok(None)
            }

            pub(crate) fn role_users(&self, role_id: &RoleId) -> Result<Vec<UserId>, $access_error_type> {
                let mut result = Vec::new();
                $(
                #[cfg(feature=$feature)]
                result.extend(
                    <$crate::auth::backends::$backend as $crate::auth::Backend>::role_users(&self.$field_name, role_id)
                        .map_err(<$access_error_type>::$backend)?
                );
                )*
                Ok(result)
            }
            pub(crate) fn role_attributes(&self, role_id: &RoleId) -> Result<Option<AttributeMap>, $access_error_type> {
                $($crate::auth::macros::main_struct!(@internal:func_call if $feature: $backend::role_attributes(&self.$field_name, role_id) -> $access_error_type))*
                Ok(None)
            }
            pub(crate) fn role_get_attribute(&self, role_id: &RoleId, attribute_id: &AttributeId) -> Result<Option<::bytecode::ByteCode>, $access_error_type> {
                $($crate::auth::macros::main_struct!(@internal:func_call if $feature: $backend::role_get_attribute(&self.$field_name, role_id, attribute_id) -> $access_error_type))*
                Ok(None)
            }
        }
    };
    (@internal:func_call if $feature:literal: $backend:ident::$func:ident($($args:expr),*) -> $err:ty) => {
        #[cfg(feature=$feature)]
        match <$crate::auth::backends::$backend as $crate::auth::Backend>::$func($($args),*) {
            Ok(Some(v)) => return Ok(Some(v)),
            Ok(None) => {},
            Err(e) => return Err(<$err>::$backend(e)),
        }
    };
}
pub(super) use main_struct;

macro_rules! combined {
    (
        $(#[main] #[$main_meta:meta])*
        $(#[config] $(#[$config_meta:meta])*)?
        $(#[config_error] $(#[$config_error_meta:meta])*)?
        $(#[access_error] $(#[$access_error_meta:meta])*)?
        $(#[login_error] $(#[$login_error_meta:meta])*)?
        pub struct<Config: $config_name:ident, ConfigError: $config_error_name:ident, AccessError: $access_error_name:ident, LoginError: $login_error_name:ident> $main_name:ident {$(
            $(#[main] #[$main_field_meta:meta])*
            $(#[config] $(#[$config_field_meta:meta])*)?
            $(#[config_error] $(#[$config_error_field_meta:meta])*)?
            $(#[access_error] $(#[$access_error_field_meta:meta])*)?
            $(#[login_error] $(#[$login_error_field_meta:meta])*)?
            if $feature:literal($config_field:ident): $main_struct_field_name:ident: $backend_name:ident
        ),* $(,)*}
    ) => {
        $crate::auth::macros::config_struct!(
            $(#[$config_meta])*
            pub struct $config_name {
                $($(#[$config_field_meta])* if $feature: $config_field: $backend_name,)*
            }
        );
        $crate::auth::macros::error_type!(
            $(#[$config_error_meta])*
            pub enum $config_error_name {
                $($(#[$config_error_field_meta])* if $feature: $backend_name::ConfigError,)*
            }
        );
        $crate::auth::macros::error_type!(
            $(#[$access_error_meta])*
            pub enum $access_error_name {
                $($(#[$access_error_field_meta])* if $feature: $backend_name::AccessError,)*
            }
        );
        $crate::auth::macros::error_type!(
            $(#[$login_error_meta])*
            pub enum $login_error_name {
                $($(#[$login_error_field_meta])* if $feature: $backend_name::LoginError,)*
            }
        );
        $crate::auth::macros::main_struct!(
            $(#[$main_meta])*
            pub struct $main_name {
                config: $config_name,
                config_err: $config_error_name,
                access_err: $access_error_name,
                login_err: $login_error_name,
                $($(#[$main_field_meta])* if $feature($config_field): $main_struct_field_name: $backend_name)*
            }
        );
    };
}
pub(super) use combined;