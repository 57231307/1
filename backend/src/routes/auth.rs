//! 认证域路由
//!
//! 处理登录、登出、刷新、CSRF、TOTP、当前用户等认证相关接口。

use crate::utils::app_state::AppState;
use axum::{
    middleware,
    routing::{get, post},
    Router,
};

use crate::handlers::auth_handler;
use crate::handlers::auth_handler_misc;
use crate::handlers::auth_handler_session;
use crate::middleware::rate_limit;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/login", post(auth_handler::login))
        .route("/logout", post(auth_handler_session::logout))
        .route("/refresh", post(auth_handler_misc::refresh_token))
        .route("/csrf-token", get(auth_handler_misc::get_csrf_token))
        .route("/totp/setup", get(auth_handler_misc::setup_totp))
        .route("/totp/enable", post(auth_handler_misc::enable_totp))
        .route("/me", get(auth_handler_misc::get_current_user))
        .layer(middleware::from_fn(rate_limit::anti_brute_force))
}
