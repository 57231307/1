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
        // P3 7-17 修复：删除 get_csrf_token 死代码接口
        // 原实现生成 token 不存缓存，前端拿到后无法通过 CSRF 中间件校验。
        // CSRF token 已通过 login/refresh 的 Set-Cookie 头下发，前端从 cookie 读取。
        .route("/totp/setup", get(auth_handler_misc::setup_totp))
        .route("/totp/enable", post(auth_handler_misc::enable_totp))
        // v11 批次 141：2FA 恢复码生成端点（前端 generateRecoveryCodes API 真实接入）
        .route(
            "/totp/recovery-codes",
            post(auth_handler_misc::generate_recovery_codes),
        )
        .route("/me", get(auth_handler_misc::get_current_user))
        .layer(middleware::from_fn(rate_limit::anti_brute_force))
}
