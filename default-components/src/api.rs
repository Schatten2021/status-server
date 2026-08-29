use std::collections::HashMap;
use axum::body::Body;
use axum::response::IntoResponse;
use utils::Never;
use api_types::{ApiResponse, ServerError};
use server::ComponentHandle;
use crate::filters::{AttributeIdMatcher, SingleFilter};

fn default_path() -> String { "/api/".to_string() }


#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Config {
    #[serde(default="default_path")]
    path: String,

    #[serde(default)]
    #[serde(alias="attribute-filter",
        alias="filter_attribute", alias="filter_attributes", alias="filter-attribute", alias="filter-attributes",
        alias="attribute", alias="attributes")]
    attribute_filter: SingleFilter<AttributeIdMatcher>,

    #[serde(default)]
    #[serde(alias="element-filter",
        alias="filter_element", alias="filter_elements", alias="filter-element", alias="filter-elements",
        alias="element", alias="elements")]
    element_filter: SingleFilter<String>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            path: default_path(),
            attribute_filter: SingleFilter::default(),
            element_filter: SingleFilter::default(),
        }
    }
}

/// Provides an API for interacting with the status server.
/// 
/// Currently implemented:
/// - [x] current state of all elements
/// - [ ] current state of specific element
/// - [ ] attribute of specific element
pub struct Api {
    state: ComponentHandle,
    config: Config,
}

fn should_handle_path(mut path: &str, mut prefix: &str) -> bool {
    path = path.trim_start_matches('/');
    prefix = prefix.trim_matches('/');
    if !path.starts_with(prefix) {
        trace!("not a request to the API");
        return false;
    }
    path = &path[prefix.len()..];
    trace!("got api request to endpoint \"{path}\"");
    matches!(
        path,
        "" | "/" |
        "/current"
    )
        || (cfg!(feature = "history") && matches!(path, "/history/attribute" | "/history/online"))
}
#[derive(Clone, Default, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConfigWrapper {
    pub frontend: Config,
}

impl server::Component for Api {
    const ID: &'static str = "api";
    type Config = ConfigWrapper;
    type ConfigError = Never;

    fn init(server: ComponentHandle, config: Self::Config) -> Result<Self, Self::ConfigError> {
        let config = config.frontend;
        trace!("loaded API with config {config:?}");
        Ok(Self {
            state: server,
            config,
        })
    }

    fn reconfigure(&mut self, config: Self::Config) -> Result<(), Self::ConfigError> {
        let config = config.frontend;
        self.config = config;
        Ok(())
    }

    #[allow(clippy::too_many_lines, reason="don't know how to simplify this decently. Also, a decent chunk is just macro definitions that are being reused multiple times.")]
    fn try_handle(&self, request: axum::extract::Request) -> Result<server::RequestHandle, axum::extract::Request> {
        macro_rules! json {
            ($code:expr, $val:expr) => {
                match ::serde_json::to_string(&$val) {
                    Ok(v) => ($code, v),
                    Err(e) => {
                        error!("couldn't JSON-serialize response: {e}");
                        match ::serde_json::to_string(&ApiResponse::<(), ()>::ServerError(ServerError {
                            id: "json.serialize".to_string(),
                            message: e.to_string(),
                        })) {
                            Ok(v) => (500, v),
                            Err(e) => {
                                error!("couldn't JSON serialize JSON serialization error?!?!? ({e})");
                                (200, "{}".to_string())
                            }
                        }
                    }
                }
            };
        }
        /// When the request was ok
        macro_rules! ok {
            ($val:expr) => {
                json!(200, ::api_types::ApiResponse::<_, ()>::Ok($val))
            };
        }
        /// User error
        macro_rules! err {
            ($code:literal, $val:expr) => {
                json!($code, ::api_types::ApiResponse::<(), _>::ClientError($val))
            };
        }
        /// Server-side exception
        macro_rules! exception {
            ($id:literal, $msg:expr) => {
                json!(500, ::api_types::ApiResponse::<(), ()>::ServerError(::api_types::ServerError {
                    id: $id.to_string(),
                    message: $msg.to_string()
                }))
            };
        }
        if !should_handle_path(request.uri().path(), &self.config.path) {
            return Err(request)
        }
        let mut path_prefix_len = self.config.path.len();
        if self.config.path.ends_with('/') {
            path_prefix_len -= 1;
        }
        let state = self.state.clone();
        let attribute_filter = self.config.attribute_filter.clone();
        let element_filter = self.config.element_filter.clone();
        Ok(Box::pin(async move {
            let path = &request.uri().path()[path_prefix_len..];
            let _ = request;
            let (code, json) = match path {
                "/" => ok!("Welcome to the API!"),
                "/current" => {
                    ok!(api_types::States::from(state.get_states()
                        .into_iter()
                        .filter(|(id, _)| element_filter.allows(id))
                        .map(|(id, mut state)| {
                            state.attributes.retain(|id, _| attribute_filter.allows(id));
                            (id, state)
                        })
                        .collect::<HashMap<_, _>>()
                    ))
                },
                // TODO: add routes for requesting selected elements/stati/etc.
                #[cfg(feature = "history")]
                "/history/attribute" => {
                    use axum::extract::{FromRequest, Json};
                    let Json(args): Json<api_types::history::AttributeHistoryRequest> = match Json::from_request(request, &()).await {
                        Ok(v) => v,
                        Err(e) => return e.into_response(),
                    };
                    let history = state.component_map::<crate::History, _, _>(|hist| {
                        hist.map(|hist| hist.get_attribute_history(&args.element_id, &args.attribute_id))
                    });
                    match history {
                        None => err!(404, "invalid element/attribute id"),
                        Some(Err(e)) => exception!("history.internal", e.to_string()),
                        Some(Ok(v)) => ok!(api_types::history::AttributeHistory(v.into_iter()
                            .map(|(timestamp, new_val)| api_types::history::AttributeHistoryElement {
                                timestamp,
                                new_value: new_val.map(Into::into),
                            })
                            .collect()
                        )),
                    }
                },
                #[cfg(feature = "history")]
                "/history/online" => {
                    use axum::extract::{FromRequest, Json};
                    let Json(args): Json<api_types::history::OnlineStateHistoryRequest> = match Json::from_request(request, &()).await {
                        Ok(v) => v,
                        Err(e) => return e.into_response(),
                    };
                    let history = state.component_map::<crate::History, _, _>(|hist| {
                        hist.map(|hist| hist.get_online_state_history(&args.element_id))
                    });
                    match history {
                        None => err!(404, "invalid element/attribute id"),
                        Some(Err(e)) => exception!("history.internal", e.to_string()),
                        Some(Ok(v)) => ok!(api_types::history::OnlineStateHistory(v.into_iter()
                            .map(|(timestamp, new_val)| api_types::history::OnlineStateHistoryElement {
                                timestamp,
                                new_state: new_val,
                            })
                            .collect()
                        )),
                    }
                },
                _ => {
                    error!("route `{path}` set to handle but no handle registered!");
                    exception!("unhandled.route", "Route marked as handled without a handle registered!")
                }
            };
            axum::response::Response::builder()
                .header("Content-Type", "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Allow-Methods", "GET")
                .header("Access-Control-Allow-Headers", "*")
                .status(code)
                .body(Body::new(json))
                .expect("some argument failed to parse?")
        }))
    }
}

#[cfg(test)]
mod test {
    use crate::filters::FilterPriority;
    use super::*;
    use crate::parse_test;

    parse_test!(parse_empty(Config): toml::Table::new() => Config {
        path: "/api/".to_string(),
        attribute_filter: SingleFilter::default(),
        element_filter: SingleFilter::default(),
    });
    parse_test!(with_path(Config): toml!{path = "/legacy/api"} => Config {
        path: "/legacy/api".to_string(),
        attribute_filter: SingleFilter::default(),
        element_filter: SingleFilter::default(),
    });
    parse_test!(attribute_filter(Config): toml!{attributes.allow = [{ id="test" }, { id="foo.bar", exact=false }]} => Config {
        path: "/api/".to_string(),
        attribute_filter: SingleFilter {
            whitelist: vec![
                AttributeIdMatcher { id: "test".to_string(), exact: true },
                AttributeIdMatcher { id: "foo.bar".to_string(), exact: false }
            ],
            blacklist: vec![],
            priority: FilterPriority::default(),
        },
        element_filter: SingleFilter::default(),
    });
    parse_test!(element_filter(Config): toml!{elements.deny = ["foo", "bar"]} => Config {
        path: "/api/".to_string(),
        attribute_filter: SingleFilter::default(),
        element_filter: SingleFilter {
            whitelist: vec![],
            blacklist: vec![
                "foo".to_string(),
                "bar".to_string(),
            ],
            priority: FilterPriority::default(),
        }
    });
}