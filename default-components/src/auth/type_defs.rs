use crate::auth::macros::combined;
use std::collections::HashMap;

pub type SessionId = String;
pub type UserId = String;
pub type RoleId = String;
pub type AttributeId = String;
pub type AttributeMap = HashMap<AttributeId, bytecode::ByteCode>;

combined!{
    #[main]
    #[derive(Debug)]
    pub struct<Config: Config,
               ConfigError: ConfigError,
               AccessError: AccessError,
               LoginError: LoginError> AuthServer {
        #[config] #[serde(flatten)]
        if "auth-config-backend"(config): config: ConfigAuthBackend,
    }
}
