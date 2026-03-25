use yew::prelude::*;
use yew_router::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen::JsCast;
use crate::app::Route;
use crate::services::init_service::InitService;
use web_sys::HtmlInputElement;

pub struct InitPage {
    admin_username: String,
    admin_password: String,
    confirm_password: String,
    error_message: Option<String>,
    success_message: Option<String>,
    is_loading: bool,
    is_initialized: bool,
}

pub enum Msg {
    AdminUsernameChanged(String),
    AdminPasswordChanged(String),
    ConfirmPasswordChanged(String),
    InitializeStarted,
    InitializeSuccess(String),
    InitializeFailure(String),
    CheckStatus,
    StatusChecked(bool),
    GoToLogin,
}

impl Component for InitPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        _ctx.link().send_message(Msg::CheckStatus);
        Self {
            admin_username: String::new(),
            admin_password: String::new(),
            confirm_password: String::new(),
            error_message: None,
            success_message: None,
            is_loading: false,
            is_initialized: false,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::AdminUsernameChanged(value) => {
                self.admin_username = value;
                self.error_message = None;
                true
            }
            Msg::AdminPasswordChanged(value) => {
                self.admin_password = value;
                self.error_message = None;
                true
            }
            Msg::ConfirmPasswordChanged(value) => {
                self.confirm_password = value;
                self.error_message = None;
                true
            }
            Msg::CheckStatus => {
                let link = _ctx.link().clone();
                spawn_local(async move {
                    match InitService::check_status().await {
                        Ok(status) => {
                            link.send_message(Msg::StatusChecked(status.initialized));
                        }
                        Err(_) => {
                            link.send_message(Msg::StatusChecked(false));
                        }
                    }
                });
                true
            }
            Msg::StatusChecked(initialized) => {
                self.is_initialized = initialized;
                if initialized {
                    self.success_message = Some("系统已经初始化，请直接登录".to_string());
                }
                true
            }
            Msg::InitializeStarted => {
                if self.admin_password != self.confirm_password {
                    self.error_message = Some("两次输入的密码不一致".to_string());
                    return true;
                }
                if self.admin_password.len() < 6 {
                    self.error_message = Some("密码长度至少为6位".to_string());
                    return true;
                }

                self.is_loading = true;
                self.error_message = None;
                self.success_message = None;

                let admin_username = self.admin_username.clone();
                let admin_password = self.admin_password.clone();
                let link = _ctx.link().clone();

                spawn_local(async move {
                    match InitService::initialize(&admin_username, &admin_password).await {
                        Ok(result) => {
                            link.send_message(Msg::InitializeSuccess(result.message));
                        }
                        Err(error) => {
                            link.send_message(Msg::InitializeFailure(error.to_string()));
                        }
                    }
                });
                true
            }
            Msg::InitializeSuccess(message) => {
                self.is_loading = false;
                self.success_message = Some(message);
                self.is_initialized = true;
                true
            }
            Msg::InitializeFailure(error) => {
                self.is_loading = false;
                self.error_message = Some(error);
                true
            }
            Msg::GoToLogin => {
                if let Some(navigator) = _ctx.link().navigator() {
                    navigator.push(&Route::Login);
                }
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let onusername = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
            Msg::AdminUsernameChanged(target.value())
        });

        let onpassword = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
            Msg::AdminPasswordChanged(target.value())
        });

        let onconfirm = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
            Msg::ConfirmPasswordChanged(target.value())
        });

        let onsubmit = ctx.link().callback(|e: SubmitEvent| {
            e.prevent_default();
            Msg::InitializeStarted
        });

        let onlogin = ctx.link().callback(|e: MouseEvent| {
            e.prevent_default();
            Msg::GoToLogin
        });

        html! {
            <div class="login-container">
                <div class="login-box">
                    <h1>{"秉羲管理系统"}</h1>
                    <h2>{"系统初始化"}</h2>
                    
                    if self.is_initialized {
                        <div class="success-message">
                            <p>{"系统已经初始化完成！"}</p>
                            <p>{"请使用管理员账号登录系统。"}</p>
                        </div>
                        <button class="login-button" onclick={onlogin}>
                            {"前往登录"}
                        </button>
                    } else {
                        <p class="init-description">
                            {"首次使用系统，请创建管理员账号进行初始化配置。"}
                        </p>
                        
                        if let Some(error) = &self.error_message {
                            <div class="error-message">{error}</div>
                        }

                        if let Some(success) = &self.success_message {
                            <div class="success-message">{success}</div>
                        }

                        <form onsubmit={onsubmit}>
                            <div class="form-group">
                                <label for="username">{"管理员用户名"}</label>
                                <input
                                    type="text"
                                    id="username"
                                    value={self.admin_username.clone()}
                                    onchange={onusername}
                                    placeholder="请输入管理员用户名"
                                    disabled={self.is_loading}
                                />
                            </div>

                            <div class="form-group">
                                <label for="password">{"管理员密码"}</label>
                                <input
                                    type="password"
                                    id="password"
                                    value={self.admin_password.clone()}
                                    onchange={onpassword}
                                    placeholder="请输入管理员密码（至少6位）"
                                    disabled={self.is_loading}
                                />
                            </div>

                            <div class="form-group">
                                <label for="confirm">{"确认密码"}</label>
                                <input
                                    type="password"
                                    id="confirm"
                                    value={self.confirm_password.clone()}
                                    onchange={onconfirm}
                                    placeholder="请再次输入密码"
                                    disabled={self.is_loading}
                                />
                            </div>

                            <button
                                type="submit"
                                class="login-button"
                                disabled={self.is_loading || self.admin_username.is_empty() || self.admin_password.is_empty() || self.confirm_password.is_empty()}
                            >
                                if self.is_loading {
                                    {"初始化中..."}
                                } else {
                                    {"初始化系统"}
                                }
                            </button>
                        </form>
                    }

                    <div class="login-footer">
                        <p>{"秉羲管理系统 v1.0.0"}</p>
                    </div>
                </div>
            </div>
        }
    }
}
