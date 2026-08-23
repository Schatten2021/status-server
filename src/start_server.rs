use std::path::PathBuf;
use axum::routing::any;
use tokio::signal::unix::{signal, SignalKind};
use tracing::info;

pub fn start(config_file: PathBuf, host: &str, port: u16) {
    tokio::runtime::Runtime::new().unwrap()
        .block_on(async {
            let server = server::Server::new(config_file);
            // wait for USR1 signal to reload config
            let mut signal = signal(SignalKind::user_defined1()).expect("unable to register SIGUSR1 signal handler");
            let server_ = server.clone();
            tokio::spawn(async move {
                while signal.recv().await.is_some() {
                    server.reload_config();
                }
            });
            let server = server_;

            macro_rules! component {
                (if $feature:literal: $component:ident) => {
                    #[cfg(feature = $feature)]
                    server.add_component::<::default_components::$component>();
                };
                (if $feature:literal: notify $component:ident) => {
                    #[cfg(feature = $feature)]
                    server.add_notification_provider::<::default_components::$component>();
                };
            }
            // do this at the start, so that notifications already have access to the names.
            component!(if "names": Names);
            component!(if "api": Api);
            component!(if "websockets": notify Websockets);
            component!(if "frontend": Frontend);

            component!(if "ntfy-notifications": notify NtfyNotificationProvider);
            component!(if "email-notifications": notify EmailNotificationProvider);

            component!(if "dataminer-status": DataminerStatus);
            component!(if "minecraft-status": MinecraftStatus);
            component!(if "website-status": WebsiteStatuse);
            

            let router = axum::Router::new()
                .route("/", any(server.clone()))
                .route("/{*any}", any(server.clone()));
            let listener = tokio::net::TcpListener::bind((host, port)).await.unwrap();
            info!("listening on http://{}:{}", host, port);
            axum::serve(listener, router).await.unwrap();
        });
}