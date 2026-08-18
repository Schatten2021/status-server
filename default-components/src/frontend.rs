use axum::extract::Request;
use utils::Never;
use server::{Component, ComponentHandle, RequestHandle};
use crate::Api;
use crate::websockets::Websockets;

/// provides a frontend for the status server.
pub struct Frontend;
impl Component for Frontend {
    const ID: &'static str = "frontend";
    type Config = ();
    type ConfigError = Never;

    fn init(handle: ComponentHandle, (): Self::Config) -> Result<Self, Self::ConfigError> {
        handle.add_component_dependency::<Api>();
        handle.add_notification_provider_dependency::<Websockets>();
        Ok(Self)
    }

    fn reconfigure(&mut self, (): Self::Config) -> Result<(), Self::ConfigError> {
        Ok(())
    }
    fn try_handle(&self, request: Request) -> Result<RequestHandle, Request> {
        #[cfg(debug_assertions)]
        macro_rules! include_file {
            ($dyn_path:literal | $include_path:literal) => {
                std::fs::read($dyn_path)
                    .map(|data| {
                        ::axum::body::Bytes::from_owner(data)
                    })
                    .unwrap_or_else(|e| {
                        error!(concat!("couldn't read \"", $dyn_path, "\": {}"), e);
                        ::axum::body::Bytes::from_static(include_bytes!($include_path))
                    })
            };
        }
        #[cfg(not(debug_assertions))]
        macro_rules! include_file {
            ($dyn_path:literal | $include_path:literal) => {
                ::axum::body::Bytes::from_static(include_bytes!($include_path))
            };
        }
        if !should_handle(request.uri().path()) {
            trace!("not a url for the frontend: \"{}\"", request.uri().path());
            return Err(request)
        }
        let (content, ty) = match request.uri().path() {
            "/" | "" => (include_file!("static/index.html" | "../../static/index.html"), "text/html; charset=utf-8"),
            "/static/style.css" => (include_file!("static/style.css" | "../../static/style.css"), "text/css; charset=utf-8"),
            "/static/wasm/frontend.js" => (include_file!("static/wasm/frontend.js" | "../../static/wasm/frontend.js"), "text/javascript; charset=utf-8"),
            "/static/wasm/frontend_bg.wasm" => (include_file!("static/wasm/frontend_bg.wasm" | "../../static/wasm/frontend_bg.wasm"), "application/wasm"),
            &_ => {
                error!("unhandled route");
                return Err(request);
            }
        };
        let response = axum::response::Response::builder()
            .status(200)
            .header("Content-Type", ty)
            .body(content.into()).unwrap();
        Ok(Box::pin(std::future::ready(response)))
    }
}
fn should_handle(mut url: &str) -> bool {
    url = url.trim_end_matches('/');
    matches!(url,
        "/" | "" |
        "/static/style.css" |
        "/static/wasm/frontend.js" |
        "/static/wasm/frontend_bg.wasm"
    )
}