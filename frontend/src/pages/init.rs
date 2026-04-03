//! 系统初始化页面
//! 包含数据库配置和管理员账号创建两个步骤

use yew::prelude::*;
use yew_router::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen::JsCast;
use crate::app::Route;
use crate::services::init_service::InitService;
use web_sys::{HtmlInputElement, InputEvent};

/// 初始化步骤
#[derive(Clone, PartialEq)]
pub enum InitStep {
    Welcome,
    DatabaseConfig,
    AdminConfig,
    Initializing,
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
    init_progress: u8,
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
    InitializeProgress(u8),
    InitializeSuccess(String),
    InitializeFailure(String),
    CheckStatus,
    StatusChecked(bool),
    GoToLogin,
    StartInit,
}

impl Component for InitPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        _ctx.link().send_message(Msg::CheckStatus);
        Self {
            current_step: InitStep::Welcome,
            db_host: String::from("localhost"),
            db_port: String::from("5432"),
            db_name: String::from("bingxi_erp"),
            db_username: String::new(),
            db_password: String::new(),
            admin_username: String::from("admin"),
            admin_password: String::new(),
            confirm_password: String::new(),
            error_message: None,
            success_message: None,
            is_loading: false,
            is_initialized: false,
            db_test_passed: false,
            init_progress: 0,
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
                let db_config = crate::models::init::DatabaseConfig {
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
                    InitStep::Welcome => {
                        self.current_step = InitStep::DatabaseConfig;
                        self.error_message = None;
                    }
                    InitStep::DatabaseConfig => {
                        if self.db_test_passed {
                            self.current_step = InitStep::AdminConfig;
                            self.error_message = None;
                        } else {
                            self.error_message = Some("请先测试数据库连接".to_string());
                        }
                    }
                    InitStep::AdminConfig => {
                        if self.admin_password != self.confirm_password {
                            self.error_message = Some("两次输入的密码不一致".to_string());
                            return true;
                        }
                        if self.admin_password.len() < 6 {
                            self.error_message = Some("密码长度至少为6位".to_string());
                            return true;
                        }
                        self.current_step = InitStep::Initializing;
                        _ctx.link().send_message(Msg::InitializeStarted);
                    }
                    _ => {}
                }
                true
            }
            Msg::PrevStep => {
                match self.current_step {
                    InitStep::DatabaseConfig => {
                        self.current_step = InitStep::Welcome;
                    }
                    InitStep::AdminConfig => {
                        self.current_step = InitStep::DatabaseConfig;
                    }
                    InitStep::Initializing => {
                        self.current_step = InitStep::AdminConfig;
                    }
                    _ => {}
                }
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
                    self.current_step = InitStep::Completed;
                    self.success_message = Some("系统已经初始化，请直接登录".to_string());
                }
                true
            }
            Msg::StartInit => {
                self.current_step = InitStep::DatabaseConfig;
                true
            }
            Msg::InitializeStarted => {
                self.is_loading = true;
                self.error_message = None;
                self.success_message = None;
                self.init_progress = 10;

                let db_config = crate::models::init::DatabaseConfig {
                    host: self.db_host.clone(),
                    port: self.db_port.clone(),
                    name: self.db_name.clone(),
                    username: self.db_username.clone(),
                    password: self.db_password.clone(),
                };
                let admin_username = self.admin_username.clone();
                let admin_password = self.admin_password.clone();
                let link = _ctx.link().clone();

                let link_clone = link.clone();
                spawn_local(async move {
                    link_clone.send_message(Msg::InitializeProgress(30));
                });

                spawn_local(async move {
                    match InitService::initialize_with_db(&db_config, &admin_username, &admin_password).await {
                        Ok(result) => {
                            link.send_message(Msg::InitializeProgress(100));
                            link.send_message(Msg::InitializeSuccess(result.message));
                        }
                        Err(error) => {
                            link.send_message(Msg::InitializeFailure(error.to_string()));
                        }
                    }
                });
                true
            }
            Msg::InitializeProgress(progress) => {
                self.init_progress = progress;
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
                self.current_step = InitStep::AdminConfig;
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

        let on_start = ctx.link().callback(|e: MouseEvent| {
            e.prevent_default();
            Msg::StartInit
        });

        let onlogin = ctx.link().callback(|e: MouseEvent| {
            e.prevent_default();
            Msg::GoToLogin
        });

        html! {
            <div class="init-container">
                <div class="init-background"></div>
                <div class="init-content">
                    <div class="init-card">
                        // 头部logo和标题
                        <div class="init-header">
                            <div class="init-logo">
                                <svg width="48" height="48" viewBox="0 0 48 48" fill="none">
                                    <rect width="48" height="48" rx="12" fill="url(#gradient)"/>
                                    <path d="M14 24L20 30L34 16" stroke="white" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
                                    <defs>
                                        <linearGradient id="gradient" x1="0" y1="0" x2="48" y2="48">
                                            <stop offset="0%" stop-color="#667eea"/>
                                            <stop offset="100%" stop-color="#764ba2"/>
                                        </linearGradient>
                                    </defs>
                                </svg>
                            </div>
                            <h1>{"秉羲管理系统"}</h1>
                            <p class="init-subtitle">{"企业级面料ERP解决方案"}</p>
                        </div>

                        // 步骤指示器
                        if !self.is_initialized {
                            <div class="step-indicator">
                                <div class={format!("step-item {}", if matches!(self.current_step, InitStep::Welcome | InitStep::DatabaseConfig | InitStep::AdminConfig | InitStep::Initializing | InitStep::Completed) { "active" } else { "" })}>
                                    <div class="step-number">{"1"}</div>
                                    <span class="step-label">{"欢迎"}</span>
                                </div>
                                <div class="step-line"></div>
                                <div class={format!("step-item {}", if matches!(self.current_step, InitStep::DatabaseConfig | InitStep::AdminConfig | InitStep::Initializing | InitStep::Completed) { "active" } else { "" })}>
                                    <div class="step-number">{"2"}</div>
                                    <span class="step-label">{"数据库"}</span>
                                </div>
                                <div class="step-line"></div>
                                <div class={format!("step-item {}", if matches!(self.current_step, InitStep::AdminConfig | InitStep::Initializing | InitStep::Completed) { "active" } else { "" })}>
                                    <div class="step-number">{"3"}</div>
                                    <span class="step-label">{"管理员"}</span>
                                </div>
                                <div class="step-line"></div>
                                <div class={format!("step-item {}", if matches!(self.current_step, InitStep::Completed) { "active" } else { "" })}>
                                    <div class="step-number">{"4"}</div>
                                    <span class="step-label">{"完成"}</span>
                                </div>
                            </div>
                        }

                        // 消息提示
                        if let Some(error) = &self.error_message {
                            <div class="alert alert-error">
                                <svg class="alert-icon" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                    <circle cx="12" cy="12" r="10"/>
                                    <line x1="12" y1="8" x2="12" y2="12"/>
                                    <line x1="12" y1="16" x2="12.01" y2="16"/>
                                </svg>
                                <span>{error}</span>
                            </div>
                        }

                        if let Some(success) = &self.success_message {
                            <div class="alert alert-success">
                                <svg class="alert-icon" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/>
                                    <polyline points="22 4 12 14.01 9 11.01"/>
                                </svg>
                                <span>{success}</span>
                            </div>
                        }

                        // 欢迎页面
                        if self.current_step == InitStep::Welcome {
                            <div class="step-content">
                                <div class="welcome-section">
                                    <div class="welcome-icon">
                                        <svg width="80" height="80" viewBox="0 0 80 80" fill="none">
                                            <circle cx="40" cy="40" r="35" fill="#f0f4ff"/>
                                            <path d="M40 20C32 20 25 27 25 35C25 40 28 44 33 46C33 50 36 53 40 53C44 53 47 50 47 46C52 44 55 40 55 35C55 27 48 20 40 20Z" fill="#667eea"/>
                                            <circle cx="40" cy="60" r="4" fill="#667eea"/>
                                            <circle cx="28" cy="55" r="3" fill="#667eea" opacity="0.6"/>
                                            <circle cx="52" cy="55" r="3" fill="#667eea" opacity="0.6"/>
                                        </svg>
                                    </div>
                                    <h2 class="welcome-title">{"欢迎使用秉羲管理系统"}</h2>
                                    <p class="welcome-text">{"让我们花几分钟时间来配置您的系统，开始您的数字化转型之旅。"}</p>
                                    
                                    <div class="feature-list">
                                        <div class="feature-item">
                                            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#667eea" stroke-width="2">
                                                <path d="M12 2L2 7l10 5 10-5-10-5z"/>
                                                <path d="M2 17l10 5 10-5"/>
                                                <path d="M2 12l10 5 10-5"/>
                                            </svg>
                                            <span>{"完整的面料业务管理"}</span>
                                        </div>
                                        <div class="feature-item">
                                            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#667eea" stroke-width="2">
                                                <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
                                                <line x1="3" y1="9" x2="21" y2="9"/>
                                                <line x1="9" y1="21" x2="9" y2="9"/>
                                            </svg>
                                            <span>{"实时库存追踪"}</span>
                                        </div>
                                        <div class="feature-item">
                                            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#667eea" stroke-width="2">
                                                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                                                <polyline points="14 2 14 8 20 8"/>
                                                <line x1="16" y1="13" x2="8" y2="13"/>
                                                <line x1="16" y1="17" x2="8" y2="17"/>
                                                <polyline points="10 9 9 9 8 9"/>
                                            </svg>
                                            <span>{"智能财务核算"}</span>
                                        </div>
                                    </div>

                                    <button class="btn btn-primary btn-large" onclick={on_start}>
                                        {"开始配置"}
                                    </button>
                                </div>
                            </div>
                        }

                        // 已初始化状态
                        else if self.is_initialized && self.current_step == InitStep::Completed {
                            <div class="step-content">
                                <div class="completed-section">
                                    <div class="completed-icon success">
                                        <svg width="64" height="64" viewBox="0 0 64 64" fill="none">
                                            <circle cx="32" cy="32" r="30" stroke="#10b981" stroke-width="3"/>
                                            <path d="M20 32L28 40L44 24" stroke="#10b981" stroke-width="4" stroke-linecap="round" stroke-linejoin="round"/>
                                        </svg>
                                    </div>
                                    <h2 class="completed-title">{"系统已就绪！"}</h2>
                                    <p class="completed-text">{"秉羲管理系统已成功配置，您现在可以开始使用了。"}</p>
                                    
                                    <div class="completed-info">
                                        <div class="info-item">
                                            <span class="info-label">{"系统版本"}</span>
                                            <span class="info-value">{"v1.0.0"}</span>
                                        </div>
                                        <div class="info-item">
                                            <span class="info-label">{"数据库"}</span>
                                            <span class="info-value">{"PostgreSQL"}</span>
                                        </div>
                                    </div>

                                    <button class="btn btn-primary btn-large" onclick={onlogin}>
                                        {"前往登录"}
                                    </button>
                                </div>
                            </div>
                        }

                        // 数据库配置
                        else if self.current_step == InitStep::DatabaseConfig {
                            <div class="step-content">
                                <h2 class="step-title">{"配置数据库连接"}</h2>
                                <p class="step-description">{"请输入数据库连接信息，系统将使用此连接存储所有业务数据。"}</p>

                                <form class="form-container">
                                    <div class="form-group">
                                        <label class="form-label">{"数据库地址"}</label>
                                        <input
                                            type="text"
                                            class="form-input"
                                            value={self.db_host.clone()}
                                            oninput={ctx.link().callback(|e: InputEvent| {
                                                let target = e.target_unchecked_into::<HtmlInputElement>();
                                                Msg::DbHostChanged(target.value())
                                            })}
                                            placeholder="例如：localhost 或 192.168.1.100"
                                            disabled={self.is_loading}
                                        />
                                    </div>

                                    <div class="form-row">
                                        <div class="form-group">
                                            <label class="form-label">{"端口"}</label>
                                            <input
                                                type="text"
                                                class="form-input"
                                                value={self.db_port.clone()}
                                                oninput={ctx.link().callback(|e: InputEvent| {
                                                    let target = e.target_unchecked_into::<HtmlInputElement>();
                                                    Msg::DbPortChanged(target.value())
                                                })}
                                                placeholder="5432"
                                                disabled={self.is_loading}
                                            />
                                        </div>
                                        <div class="form-group">
                                            <label class="form-label">{"数据库名"}</label>
                                            <input
                                                type="text"
                                                class="form-input"
                                                value={self.db_name.clone()}
                                                oninput={ctx.link().callback(|e: InputEvent| {
                                                    let target = e.target_unchecked_into::<HtmlInputElement>();
                                                    Msg::DbNameChanged(target.value())
                                                })}
                                                placeholder="bingxi_erp"
                                                disabled={self.is_loading}
                                            />
                                        </div>
                                    </div>

                                    <div class="form-group">
                                        <label class="form-label">{"用户名"}</label>
                                        <input
                                            type="text"
                                            class="form-input"
                                            value={self.db_username.clone()}
                                            oninput={ctx.link().callback(|e: InputEvent| {
                                                let target = e.target_unchecked_into::<HtmlInputElement>();
                                                Msg::DbUsernameChanged(target.value())
                                            })}
                                            placeholder="请输入数据库用户名"
                                            disabled={self.is_loading}
                                        />
                                    </div>

                                    <div class="form-group">
                                        <label class="form-label">{"密码"}</label>
                                        <input
                                            type="password"
                                            class="form-input"
                                            value={self.db_password.clone()}
                                            oninput={ctx.link().callback(|e: InputEvent| {
                                                let target = e.target_unchecked_into::<HtmlInputElement>();
                                                Msg::DbPasswordChanged(target.value())
                                            })}
                                            placeholder="请输入数据库密码"
                                            disabled={self.is_loading}
                                        />
                                    </div>

                                    <div class="button-group">
                                        <button
                                            type="button"
                                            class="btn btn-secondary"
                                            onclick={on_prev}
                                            disabled={self.is_loading}
                                        >
                                            {"上一步"}
                                        </button>
                                        <button
                                            type="button"
                                            class={format!("btn {}", if self.db_test_passed { "btn-success" } else { "btn-outline" })}
                                            onclick={on_test_db}
                                            disabled={self.is_loading || self.db_host.is_empty() || self.db_port.is_empty() || self.db_name.is_empty() || self.db_username.is_empty() || self.db_password.is_empty()}
                                        >
                                            if self.is_loading {
                                                <span class="spinner"></span>
                                                {"测试中..."}
                                            } else if self.db_test_passed {
                                                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                    <polyline points="20 6 9 17 4 12"/>
                                                </svg>
                                                {"连接成功"}
                                            } else {
                                                {"测试连接"}
                                            }
                                        </button>
                                        <button
                                            type="button"
                                            class="btn btn-primary"
                                            onclick={on_next}
                                            disabled={!self.db_test_passed}
                                        >
                                            {"下一步"}
                                            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                <line x1="5" y1="12" x2="19" y2="12"/>
                                                <polyline points="12 5 19 12 12 19"/>
                                            </svg>
                                        </button>
                                    </div>
                                </form>
                            </div>
                        }

                        // 管理员配置
                        else if self.current_step == InitStep::AdminConfig {
                            <div class="step-content">
                                <h2 class="step-title">{"创建管理员账号"}</h2>
                                <p class="step-description">{"请设置系统管理员账号，此账号将拥有系统最高权限。"}</p>

                                <form class="form-container">
                                    <div class="form-group">
                                        <label class="form-label">{"管理员用户名"}</label>
                                        <div class="input-icon">
                                            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#9ca3af" stroke-width="2">
                                                <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/>
                                                <circle cx="12" cy="7" r="4"/>
                                            </svg>
                                            <input
                                                type="text"
                                                class="form-input has-icon"
                                                value={self.admin_username.clone()}
                                                oninput={ctx.link().callback(|e: InputEvent| {
                                                    let target = e.target_unchecked_into::<HtmlInputElement>();
                                                    Msg::AdminUsernameChanged(target.value())
                                                })}
                                                placeholder="请输入管理员用户名"
                                                disabled={self.is_loading}
                                            />
                                        </div>
                                    </div>

                                    <div class="form-group">
                                        <label class="form-label">{"密码"}</label>
                                        <div class="input-icon">
                                            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#9ca3af" stroke-width="2">
                                                <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
                                                <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
                                            </svg>
                                            <input
                                                type="password"
                                                class="form-input has-icon"
                                                value={self.admin_password.clone()}
                                                oninput={ctx.link().callback(|e: InputEvent| {
                                                    let target = e.target_unchecked_into::<HtmlInputElement>();
                                                    Msg::AdminPasswordChanged(target.value())
                                                })}
                                                placeholder="请输入密码（至少6位）"
                                                disabled={self.is_loading}
                                            />
                                        </div>
                                        <div class="password-hint">
                                            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#9ca3af" stroke-width="2">
                                                <circle cx="12" cy="12" r="10"/>
                                                <line x1="12" y1="16" x2="12" y2="12"/>
                                                <line x1="12" y1="8" x2="12.01" y2="8"/>
                                            </svg>
                                            {"密码长度至少6位"}
                                        </div>
                                    </div>

                                    <div class="form-group">
                                        <label class="form-label">{"确认密码"}</label>
                                        <div class="input-icon">
                                            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#9ca3af" stroke-width="2">
                                                <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
                                                <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
                                            </svg>
                                            <input
                                                type="password"
                                                class="form-input has-icon"
                                                value={self.confirm_password.clone()}
                                                oninput={ctx.link().callback(|e: InputEvent| {
                                                    let target = e.target_unchecked_into::<HtmlInputElement>();
                                                    Msg::ConfirmPasswordChanged(target.value())
                                                })}
                                                placeholder="请再次输入密码"
                                                disabled={self.is_loading}
                                            />
                                        </div>
                                        if !self.confirm_password.is_empty() && self.admin_password != self.confirm_password {
                                            <div class="password-match error">
                                                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#ef4444" stroke-width="2">
                                                    <line x1="18" y1="6" x2="6" y2="18"/>
                                                    <line x1="6" y1="6" x2="18" y2="18"/>
                                                </svg>
                                                {"两次密码不一致"}
                                            </div>
                                        } else if !self.confirm_password.is_empty() && self.admin_password == self.confirm_password {
                                            <div class="password-match success">
                                                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#10b981" stroke-width="2">
                                                    <polyline points="20 6 9 17 4 12"/>
                                                </svg>
                                                {"密码匹配"}
                                            </div>
                                        }
                                    </div>

                                    <div class="button-group">
                                        <button
                                            type="button"
                                            class="btn btn-secondary"
                                            onclick={on_prev}
                                            disabled={self.is_loading}
                                        >
                                            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                <line x1="19" y1="12" x2="5" y2="12"/>
                                                <polyline points="12 19 5 12 12 5"/>
                                            </svg>
                                            {"上一步"}
                                        </button>
                                        <button
                                            type="button"
                                            class="btn btn-primary btn-large"
                                            onclick={on_next}
                                            disabled={self.is_loading || self.admin_username.is_empty() || self.admin_password.is_empty() || self.confirm_password.is_empty() || self.admin_password != self.confirm_password || self.admin_password.len() < 6}
                                        >
                                            {"完成配置"}
                                        </button>
                                    </div>
                                </form>
                            </div>
                        }

                        // 初始化中
                        else if self.current_step == InitStep::Initializing {
                            <div class="step-content">
                                <div class="initializing-section">
                                    <div class="initializing-icon">
                                        <div class="spinner-large"></div>
                                    </div>
                                    <h2 class="step-title">{"正在初始化系统..."}</h2>
                                    <p class="step-description">{"请稍候，系统正在配置数据库和创建管理员账号。"}</p>
                                    
                                    <div class="progress-container">
                                        <div class="progress-bar">
                                            <div class="progress-fill" style={format!("width: {}%", self.init_progress)}></div>
                                        </div>
                                        <span class="progress-text">{format!("{}%", self.init_progress)}</span>
                                    </div>

                                    <div class="init-steps">
                                        <div class={format!("init-step-item {}", if self.init_progress >= 10 { "active" } else { "" })}>
                                            <div class="init-step-icon">
                                                if self.init_progress >= 30 {
                                                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                        <polyline points="20 6 9 17 4 12"/>
                                                    </svg>
                                                } else if self.init_progress >= 10 {
                                                    <div class="spinner-small"></div>
                                                } else {
                                                    <span>{"1"}</span>
                                                }
                                            </div>
                                            <span>{"连接数据库"}</span>
                                        </div>
                                        <div class={format!("init-step-item {}", if self.init_progress >= 30 { "active" } else { "" })}>
                                            <div class="init-step-icon">
                                                if self.init_progress >= 60 {
                                                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                        <polyline points="20 6 9 17 4 12"/>
                                                    </svg>
                                                } else if self.init_progress >= 30 {
                                                    <div class="spinner-small"></div>
                                                } else {
                                                    <span>{"2"}</span>
                                                }
                                            </div>
                                            <span>{"初始化表结构"}</span>
                                        </div>
                                        <div class={format!("init-step-item {}", if self.init_progress >= 60 { "active" } else { "" })}>
                                            <div class="init-step-icon">
                                                if self.init_progress >= 90 {
                                                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                        <polyline points="20 6 9 17 4 12"/>
                                                    </svg>
                                                } else if self.init_progress >= 60 {
                                                    <div class="spinner-small"></div>
                                                } else {
                                                    <span>{"3"}</span>
                                                }
                                            </div>
                                            <span>{"创建管理员账号"}</span>
                                        </div>
                                        <div class={format!("init-step-item {}", if self.init_progress >= 90 { "active" } else { "" })}>
                                            <div class="init-step-icon">
                                                if self.init_progress >= 100 {
                                                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                        <polyline points="20 6 9 17 4 12"/>
                                                    </svg>
                                                } else if self.init_progress >= 90 {
                                                    <div class="spinner-small"></div>
                                                } else {
                                                    <span>{"4"}</span>
                                                }
                                            </div>
                                            <span>{"完成设置"}</span>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        }
                    </div>

                    // 页脚
                    <div class="init-footer">
                        <p>{"秉羲管理系统 v1.0.0"}</p>
                        <p class="footer-link">{"需要帮助？查看文档"}</p>
                    </div>
                </div>
            </div>
        }
    }
}
