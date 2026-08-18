use std::path::PathBuf;
use toml::macros::Deserialize;
use server::{Component, Config};

pub fn check(path_: &PathBuf) -> Result<(), ()> {
    let path = path_.to_string_lossy();
    let content = std::fs::read_to_string(path_)
        .map_err(|e| error!("couldn't read file `{path}` due to: {e}"))?;
    let toml: Config = toml::from_str(&content)
        .map_err(|e| error!("`{path}` contains invalid TOML: {e}"))?;

    let mut ok = true;
    macro_rules! component {
        (if $feature:literal: $component:ident) => {
            #[cfg(feature = $feature)]
            if let Some(config) = toml.configs.get(::default_components::$component::ID) {
                match <::default_components::$component as ::server::Component>::Config::deserialize(config.clone()) {
                    Ok(v) => {
                        info!("successfully parsed config for `{}`", ::default_components::$component::ID);
                        debug!("config for component `{}` is: `{v:?}`", ::default_components::$component::ID);
                    }
                    Err(e) => {
                        error!("Invalid config for `{}`: {e}", ::default_components::$component::ID);
                        ok &= !toml.global.ignored.components.contains(::default_components::$component::ID);
                    }
                }
            } else {
                info!("no config found for `{}`", ::default_components::$component::ID);
            }
        };
    }
    component!(if "api": Api);
    component!(if "websockets": Websockets);
    component!(if "frontend": Frontend);

    component!(if "ntfy-notifications": NtfyNotificationProvider);
    component!(if "email-notifications": EmailNotificationProvider);

    component!(if "dataminer-status": DataminerStatus);
    component!(if "minecraft-status": MinecraftStatus);
    component!(if "website-status": WebsiteStatuse);
    if ok {
        info!("all config is OK");
        Ok(())
    } else { Err(()) }
}