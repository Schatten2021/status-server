use web_sys::SubmitEvent;
use yew::{Callback, Context, Html, NodeRef};

#[derive(PartialEq, yew::Properties)]
pub struct Props {
    pub on_login: Callback<String>
}
pub enum Message {
    RequestLogin,
    LoginResultReceived(api_types::ApiResponse<api_types::auth::LoginResponse, String>),
}
pub struct LoginPage {
    username_ref: NodeRef,
    password_ref: NodeRef,
}
impl yew::Component for LoginPage {
    type Message = Message;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            username_ref: NodeRef::default(),
            password_ref: NodeRef::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Message::RequestLogin => {
                macro_rules! input_value {
                    ($input:expr) => {$input.cast::<web_sys::HtmlInputElement>().map(|v| v.value()).unwrap_or_default()};
                }
                let username = input_value!(self.username_ref);
                let password = input_value!(self.password_ref);
                let link = ctx.link().clone();
                wasm_bindgen_futures::spawn_local(async move {
                    link.send_message(Message::LoginResultReceived(gloo_net::http::Request::post("/api/login")
                        .json(&api_types::auth::LoginRequest { username, password }).expect("unable to serialize Authentication JSON")
                        .send().await.expect("unable to send request to /api/current")
                        .json().await.expect("unable to read api response from /api/current")));
                });
            }
            Message::LoginResultReceived(response) => match response {
                api_types::ApiResponse::Ok(api_types::auth::LoginResponse { session_id }) => ctx.props().on_login.emit(session_id),
                api_types::ApiResponse::ServerError(e) => error!("Server error: {e:?}"),
                api_types::ApiResponse::ClientError(e) => error!("invalid request: \"{e}\""),
            }
        }
        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let callback = ctx.link().callback(move |e: SubmitEvent| {
            e.prevent_default();
            Message::RequestLogin
        });
        html!{
            <form id="login-form" onsubmit={callback}>
                <label>{"username: "}<input type="text" name="username" ref={self.username_ref.clone()}/></label><br/>
                <label>{"password: "}<input type="password" name="password" ref={self.password_ref.clone()}/></label><br/>
                <input type="submit" value="submit"/>
            </form>
        }
    }
}