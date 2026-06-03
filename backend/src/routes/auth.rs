//! 认证域路由
//!
//! 处理登录、登出、刷新、CSRF、TOTP、当前用户等认证相关接口。

use axum::{
    middleware,
    routing::{get, post},
    Router,
};

use crate::handlers::{auth_handler};
use crate::middleware::rate_limit;

pub fn routes() -> Router {
    Router::new()
        .route("/login", post(auth_handler::login))
        .route("/logout", post(auth_handler::logout))
        .route("/refresh", post(auth_handler::refresh_token))
        .route("/csrf-token", get(auth_handler::get_csrf_token))
        .route("/totp/setup", get(auth_handler::setup_totp))
        .route("/totp/enable", post(auth_handler::enable_totp))
        .route("/me", get(auth_handler::get_current_user))
        .layer(middleware::from_fn(rate_limit::anti_brute_force))
}
