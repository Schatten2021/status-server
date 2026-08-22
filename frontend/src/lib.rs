//! Frontend for my Status server.

#![cfg_attr(not(debug_assertions), deny(missing_docs))]
#![cfg_attr(debug_assertions, warn(missing_docs))]
#![warn(clippy::pedantic)]
#![warn(clippy::complexity, clippy::suspicious, clippy::perf, clippy::style, clippy::allow_attributes_without_reason)]
#![allow(
    clippy::needless_continue,
    reason = "adding a `continue` often makes the code easier to read."
)]
#![allow(
    clippy::missing_errors_doc,
    clippy::doc_markdown,
    reason = "don't want these lints."
)]
#![cfg_attr(not(debug_assertions), deny(clippy::undocumented_unsafe_blocks))]
#![cfg_attr(debug_assertions, warn(clippy::undocumented_unsafe_blocks))]

mod status;
mod element_display;

#[macro_use]
extern crate tracing;

#[macro_use]
extern crate yew;

use gloo_net::websocket::Message;
use yew::{Context, Html};
use crate::status::AppState;
use api_types::{ApiResponse, States};


#[cfg(debug_assertions)]
const LEVEL: tracing::Level = tracing::Level::TRACE;
#[cfg(not(debug_assertions))]
const LEVEL: tracing::Level = tracing::Level::INFO;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
/// runs the frontend.
///
/// # Panics
/// When something goes irrecoverably wrong.
pub fn run() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    tracing_wasm::set_as_global_default_with_config(tracing_wasm::WASMLayerConfigBuilder::new()
        .set_max_level(LEVEL)
        .build());
    // tracing_subscriber::fmt()
    //     .with_max_level(LevelFilter::TRACE)
    //     .init();
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let app_div = document.get_element_by_id("app").expect("should have a app element");
    yew::Renderer::<App>::with_root(app_div)
        .render();
    web_sys::console::log_1(&"running_app".into());
}

struct App {
    state: Option<AppState>,
}
enum AppMessage {
    LoadedInitial(AppState),
    ReceivedMessage(api_types::websocket::Message),
}
impl yew::Component for App {
    type Message = AppMessage;
    type Properties = ();
    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link().clone();
        wasm_bindgen_futures::spawn_local(async move {
            let initial: ApiResponse<States, ()> = gloo_net::http::Request::get("/api/current")
                .send().await.expect("unable to send request to /api/current")
                .json().await.expect("unable to read api response from /api/current");
            let data = match initial {
                ApiResponse::Ok(v) => AppState::from(v),
                ApiResponse::ServerError(e) => {
                    error!("server error `{}`: {}", e.id, e.message);
                    return;
                }
                ApiResponse::ClientError(()) => panic!("something went wrong...")
            };
            link.send_message(Self::Message::LoadedInitial(data));
        });
        Self {
            state: None,
        }
    }
    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            AppMessage::LoadedInitial(data) => {
                self.state = Some(data);
                let link = ctx.link().clone();
                wasm_bindgen_futures::spawn_local(async move {
                    use futures_util::StreamExt;
                    let (host, secure) = {
                        if let Some(window) = web_sys::window() {
                            let location = window.location();
                            (
                                location.host().unwrap_or_default(),
                                location.protocol().unwrap_or_default().starts_with("https")
                            )
                        } else {
                            (String::new(), false)
                        }
                    };
                    let url = format!("{}://{host}/api/ws", if secure { "wss" } else { "ws" });
                    let mut socket = gloo_net::websocket::futures::WebSocket::open(&url)
                        .expect("unable to open websocket");
                    while let Some(Ok(message)) = socket.next().await {
                        let message = match message {
                            Message::Text(msg) => {
                                trace!("received message: {msg}");
                                serde_json::from_str::<api_types::websocket::Message>(&msg)
                                    .expect("received an invalid message from the server!")
                            },
                            Message::Bytes(_) => {
                                error!("received bytes message...");
                                continue;
                            }
                        };
                        link.send_message(AppMessage::ReceivedMessage(message));
                    }
                });
            },
            AppMessage::ReceivedMessage(msg) => self.state.as_mut().expect("received websocket message before state was set")
                .handle(msg)
        }
        true
    }
    fn view(&self, _: &Context<Self>) -> Html {
        let Some(state) = &self.state else {
            return html!(<p>{"loading state from server..."}</p>)
        };
        state.0.iter()
            .map(|(a, b)| (a.clone(), b.clone()))
            .map(|(id, val)| html!(<element_display::ElementDisplay id={id} element={val}/>))
            .collect::<Html>()
    }
}