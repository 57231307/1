//! 系统初始化页面
//! 包含数据库配置和管理员账号创建两个步骤

use yew::prelude::*;
use yew_router::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen::JsCast;
use crate::app::Route;
use crate::services::init_service::InitService;
use web_sys::HtmlInputElement;

/// 初始化步骤
#[derive(Clone, PartialEq)]
pub enum InitStep {
    DatabaseConfig,
    AdminConfig,
    Completed,
}

pub struct InitPage {
    current_step: InitStep,
    db_host: String,
    db_port: String,
    db_name: String,
    db_username: String,
    db_password: String,
    admin_username: String,
    admin_password: String,
    confirm_password: String,
    error_message: Option<String>,
    success_message: Option<String>,
    is_loading: bool,
    is_initialized: bool,
    db_test_passed: bool,
}

pub enum Msg {
    DbHostChanged(String),
    DbPortChanged(String),
    DbNameChanged(String),
    DbUsernameChanged(String),
    DbPasswordChanged(String),
    AdminUsernameChanged(String),
    AdminPasswordChanged(String),
    ConfirmPasswordChanged(String),
    TestDbConnection,
    DbTestSuccess,
    DbTestFailed(String),
    NextStep,
    PrevStep,
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
            current_step: InitStep::DatabaseConfig,
            db_host: String::from("localhost"),
            db_port: String::from("5432"),
            db_name: String::from("bingxi_erp"),
            db_username: String::new(),
            db_password: String::new(),
            admin_username: String::new(),
            admin_password: String::new(),
            confirm_password: String::new(),
            error_message: None,
            success_message: None,
            is_loading: false,
            is_initialized: false,
            db_test_passed: false,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::DbHostChanged(value) => {
                self.db_host = value;
                self.db_test_passed = false;
                self.error_message = None;
                true
            }
            Msg::DbPortChanged(value) => {
                self.db_port = value;
                self.db_test_passed = false;
                self.error_message = None;
                true
            }
            Msg::DbNameChanged(value) => {
                self.db_name = value;
                self.db_test_passed = false;
                self.error_message = None;
                true
            }
            Msg::DbUsernameChanged(value) => {
                self.db_username = value;
                self.db_test_passed = false;
                self.error_message = None;
                true
            }
            Msg::DbPasswordChanged(value) => {
                self.db_password = value;
                self.db_test_passed = false;
                self.error_message = None;
                true
            }
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
            Msg::TestDbConnection => {
                self.is_loading = true;
                self.error_message = None;
                let db_config = crate::services::init_service::DatabaseConfig {
                    host: self.db_host.clone(),
                    port: self.db_port.clone(),
                    name: self.db_name.clone(),
                    username: self.db_username.clone(),
                    password: self.db_password.clone(),
                };
                let link = _ctx.link().clone();
                spawn_local(async move {
                    match InitService::test_database(&db_config).await {
                        Ok(_) => link.send_message(Msg::DbTestSuccess),
                        Err(e) => link.send_message(Msg::DbTestFailed(e.to_string())),
                    }
                });
                true
            }
            Msg::DbTestSuccess => {
                self.is_loading = false;
                self.db_test_passed = true;
                self.success_message = Some("数据库连接成功".to_string());
                true
            }
            Msg::DbTestFailed(error) => {
                self.is_loading = false;
                self.db_test_passed = false;
                self.error_message = Some(error);
                true
            }
            Msg::NextStep => {
                match self.current_step {
                    InitStep::DatabaseConfig => {
                        if self.db_test_passed {
                            self.current_step = InitStep::AdminConfig;
                            self.error_message = None;
                        } else {
                            self.error_message = Some("请先测试数据库连接".to_string());
                        }
                    }
                    InitStep::AdminConfig => {
                        self.current_step = InitStep::Completed;
                    }
                    _ => {}
                }
                true
            }
            Msg::PrevStep => {
                match self.current_step {
                    InitStep::AdminConfig => {
                        self.current_step = InitStep::DatabaseConfig;
                    }
                    InitStep::Completed => {
                        self.current_step = InitStep::AdminConfig;
                    }
                    _ => {}
                }
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
                    self.current_step = InitStep::Completed;
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

                let db_config = crate::services::init_service::DatabaseConfig {
                    host: self.db_host.clone(),
                    port: self.db_port.clone(),
                    name: self.db_name.clone(),
                    username: self.db_username.clone(),
                    password: self.db_password.clone(),
                };
                let admin_username = self.admin_username.clone();
                let admin_password = self.admin_password.clone();
                let link = _ctx.link().clone();

                spawn_local(async move {
                    match InitService::initialize_with_db(&db_config, &admin_username, &admin_password).await {
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
                self.current_step = InitStep::Completed;
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
        let on_db_host = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
            Msg::DbHostChanged(target.value())
        });

        let on_db_port = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
            Msg::DbPortChanged(target.value())
        });

        let on_db_name = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
            Msg::DbNameChanged(target.value())
        });

        let on_db_username = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
            Msg::DbUsernameChanged(target.value())
        });

        let on_db_password = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
            Msg::DbPasswordChanged(target.value())
        });

        let on_admin_username = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
            Msg::AdminUsernameChanged(target.value())
        });

        let on_admin_password = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
            Msg::AdminPasswordChanged(target.value())
        });

        let on_confirm_password = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
            Msg::ConfirmPasswordChanged(target.value())
        });

        let on_test_db = ctx.link().callback(|e: MouseEvent| {
            e.prevent_default();
            Msg::TestDbConnection
        });

        let on_next = ctx.link().callback(|e: MouseEvent| {
            e.prevent_default();
            Msg::NextStep
        });

        let on_prev = ctx.link().callback(|e: MouseEvent| {
            e.prevent_default();
            Msg::PrevStep
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
                <div class="login-box" style="max-width: 600px;">
                    <h1>{"秉羲管理系统"}</h1>
                    <h2>{"系统初始化"}</h2>

                    // 步骤指示器
                    <div class="step-indicator" style="display: flex; justify-content: center; margin-bottom: 20px;">
                        <div class={format!("step {}", if self.current_step == InitStep::DatabaseConfig { "active" } else { "" })}>
                            {"1. 数据库配置"}
                        </div>
                        <div style="margin: 0 10px;">{"→"}</div>
                        <div class={format!("step {}", if self.current_step == InitStep::AdminConfig { "active" } else { "" })}>
                            {"2. 管理员账号"}
                        </div>
                        <div style="margin: 0 10px;">{"→"}</div>
                        <div class={format!("step {}", if self.current_step == InitStep::Completed { "active" } else { "" })}>
                            {"3. 完成"}
                        </div>
                    </div>

                    if self.is_initialized {
                        <div class="success-message">
                            <p>{"系统已经初始化完成！"}</p>
                            <p>{"请使用管理员账号登录系统。"}</p>
                        </div>
                        <button class="login-button" onclick={onlogin}>
                            {"前往登录"}
                        </button>
                    } else {
                        if let Some(error) = &self.error_message {
                            <div class="error-message">{error}</div>
                        }

                        if let Some(success) = &self.success_message {
                            <div class="success-message">{success}</div>
                        }

                        // 步骤1: 数据库配置
                        if self.current_step == InitStep::DatabaseConfig {
                            <p class="init-description">
                                {"请配置数据库连接信息，系统将使用此连接存储所有业务数据。"}
                            </p>

                            <form onsubmit={ctx.link().callback(|e: SubmitEvent| { e.prevent_default(); Msg::TestDbConnection })}>
                                <div class="form-group">
                                    <label for="db_host">{"数据库地址"}</label>
                                    <input
                                        type="text"
                                        id="db_host"
                                        value={self.db_host.clone()}
                                        onchange={on_db_host}
                                        placeholder="例如：localhost 或 192.168.1.100"
                                        disabled={self.is_loading}
                                    />
                                </div>

                                <div class="form-group">
                                    <label for="db_port">{"数据库端口"}</label>
                                    <input
                                        type="text"
                                        id="db_port"
                                        value={self.db_port.clone()}
                                        onchange={on_db_port}
                                        placeholder="例如：5432"
                                        disabled={self.is_loading}
                                    />
                                </div>

                                <div class="form-group">
                                    <label for="db_name">{"数据库名称"}</label>
                                    <input
                                        type="text"
                                        id="db_name"
                                        value={self.db_name.clone()}
                                        onchange={on_db_name}
                                        placeholder="例如：bingxi_erp"
                                        disabled={self.is_loading}
                                    />
                                </div>

                                <div class="form-group">
                                    <label for="db_username">{"数据库用户名"}</label>
                                    <input
                                        type="text"
                                        id="db_username"
                                        value={self.db_username.clone()}
                                        onchange={on_db_username}
                                        placeholder="请输入数据库用户名"
                                        disabled={self.is_loading}
                                    />
                                </div>

                                <div class="form-group">
                                    <label for="db_password">{"数据库密码"}</label>
                                    <input
                                        type="password"
                                        id="db_password"
                                        value={self.db_password.clone()}
                                        onchange={on_db_password}
                                        placeholder="请输入数据库密码"
                                        disabled={self.is_loading}
                                    />
                                </div>

                                <div style="display: flex; gap: 10px; margin-top: 20px;">
                                    <button
                                        type="submit"
                                        class="login-button"
                                        style="flex: 1;"
                                        disabled={self.is_loading || self.db_host.is_empty() || self.db_port.is_empty() || self.db_name.is_empty() || self.db_username.is_empty() || self.db_password.is_empty()}
                                    >
                                        if self.is_loading {
                                            {"测试连接中..."}
                                        } else if self.db_test_passed {
                                            {"✓ 连接成功"}
                                        } else {
                                            {"测试数据库连接"}
                                        }
                                    </button>

                                    <button
                                        type="button"
                                        class="login-button"
                                        style="flex: 1;"
                                        onclick={on_next}
                                        disabled={!self.db_test_passed}
                                    >
                                        {"下一步"}
                                    </button>
                                </div>
                            </form>
                        }

                        // 步骤2: 管理员账号配置
                        else if self.current_step == InitStep::AdminConfig {
                            <p class="init-description">
                                {"请创建系统管理员账号，此账号将拥有系统最高权限。"}
                            </p>

                            <form onsubmit={onsubmit}>
                                <div class="form-group">
                                    <label for="username">{"管理员用户名"}</label>
                                    <input
                                        type="text"
                                        id="username"
                                        value={self.admin_username.clone()}
                                        onchange={on_admin_username}
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
                                        onchange={on_admin_password}
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
                                        onchange={on_confirm_password}
                                        placeholder="请再次输入密码"
                                        disabled={self.is_loading}
                                    />
                                </div>

                                <div style="display: flex; gap: 10px; margin-top: 20px;">
                                    <button
                                        type="button"
                                        class="login-button"
                                        style="flex: 1; background-color: #6c757d;"
                                        onclick={on_prev}
                                        disabled={self.is_loading}
                                    >
                                        {"上一步"}
                                    </button>

                                    <button
                                        type="submit"
                                        class="login-button"
                                        style="flex: 1;"
                                        disabled={self.is_loading || self.admin_username.is_empty() || self.admin_password.is_empty() || self.confirm_password.is_empty()}
                                    >
                                        if self.is_loading {
                                            {"初始化中..."}
                                        } else {
                                            {"完成初始化"}
                                        }
                                    </button>
                                </div>
                            </form>
                        }
                    }
                </div>

                <div class="login-footer">
                    <p>{"秉羲管理系统 v1.0.0"}</p>
                </div>
            </div>
        }
    }
}
