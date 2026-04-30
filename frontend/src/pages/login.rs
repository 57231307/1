use crate::app::Route;
use crate::services::auth::AuthService;
use crate::services::init_service::InitService;
use crate::utils::storage::Storage;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

pub struct LoginPage {
    username: String,
    password: String,
    error_message: Option<String>,
    is_loading: bool,
    auth_service: AuthService,
    need_init: bool,
}

pub enum Msg {
    UsernameChanged(String),
    PasswordChanged(String),
    LoginStarted,
    LoginSuccess(String),
    LoginFailure(String),
    LoadingChanged(bool),
    CheckInitStatus,
    InitStatusChecked(bool),
}

impl Component for LoginPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        _ctx.link().send_message(Msg::CheckInitStatus);
        Self {
            username: String::new(),
            password: String::new(),
            error_message: None,
            is_loading: false,
            auth_service: AuthService::new(),
            need_init: false,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::CheckInitStatus => {
                let link = _ctx.link().clone();
                spawn_local(async move {
                    match InitService::check_status().await {
                        Ok(status) => {
                            link.send_message(Msg::InitStatusChecked(!status.initialized));
                        }
                        Err(_) => {
                            link.send_message(Msg::InitStatusChecked(true));
                        }
                    }
                });
                true
            }
            Msg::InitStatusChecked(need_init) => {
                self.need_init = need_init;
                if need_init {
                    if let Some(navigator) = _ctx.link().navigator() {
                        navigator.push(&Route::Init);
                    }
                }
                true
            }
            Msg::UsernameChanged(value) => {
                self.username = value;
                self.error_message = None;
                true
            }
            Msg::PasswordChanged(value) => {
                self.password = value;
                self.error_message = None;
                true
            }
            Msg::LoginStarted => {
                self.is_loading = true;
                self.error_message = None;

                let auth_service = self.auth_service.clone();
                let username = self.username.clone();
                let password = self.password.clone();
                let link = _ctx.link().clone();

                spawn_local(async move {
                    match auth_service.login(&username, &password).await {
                        Ok(response) => {
                            link.send_message(Msg::LoginSuccess(response.token));
                        }
                        Err(error) => {
                            link.send_message(Msg::LoginFailure(error));
                        }
                    }
                });
                true
            }
            Msg::LoginSuccess(token) => {
                self.is_loading = false;
                Storage::set_token(&token);

                // 登录成功后清除 just_initialized 标志
                if let Some(Ok(Some(storage))) = web_sys::window().map(|w| w.session_storage()) {
                    let _ = storage.remove_item("just_initialized");
                }

                // 登录成功，跳转到仪表板
                if let Some(navigator) = _ctx.link().navigator() {
                    navigator.push(&Route::Dashboard);
                }
                true
            }
            Msg::LoginFailure(error) => {
                self.is_loading = false;
                self.error_message = Some(error);
                true
            }
            Msg::LoadingChanged(loading) => {
                self.is_loading = loading;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let onusername = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
            Msg::UsernameChanged(target.value())
        });

        let onpassword = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
            Msg::PasswordChanged(target.value())
        });

        let onsubmit = ctx.link().callback(|e: SubmitEvent| {
            e.prevent_default();
            Msg::LoginStarted
        });

        html! {
            <div class="login-container">
                <div class="login-box">
                    <h1>{"秉羲面料管理"}</h1>
                    <h2>{"用户登录"}</h2>

                    if let Some(error) = &self.error_message {
                        <div class="error-message">{error}</div>
                    }

                    <form onsubmit={onsubmit}>
                        <div class="form-group">
                            <label for="username">{"用户名"}</label>
                            <input
                                type="text"
                                id="username"
                                value={self.username.clone()}
                                onchange={onusername}
                                placeholder="请输入用户名"
                                disabled={self.is_loading}
                            />
                        </div>

                        <div class="form-group">
                            <label for="password">{"密码"}</label>
                            <input
                                type="password"
                                id="password"
                                value={self.password.clone()}
                                onchange={onpassword}
                                placeholder="请输入密码"
                                disabled={self.is_loading}
                            />
                        </div>

                        <button
                            type="submit"
                            class="login-button"
                            disabled={self.is_loading || self.username.is_empty() || self.password.is_empty()}
                        >
                            if self.is_loading {
                                {"登录中..."}
                            } else {
                                {"登录"}
                            }
                        </button>
                    </form>

                    <div class="login-footer">
                        <p>{"秉羲面料管理 v1.0.0"}</p>
                    </div>
                </div>
            </div>
        }
    }
}
