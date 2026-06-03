//! V1 API 路由总入口占位
//!
//! 所有 14 个业务域子模块由主 `mod.rs` 在不同 nest 路径下独立挂载。
//! 本文件保留 `pub fn routes() -> Router` 接口签名以满足统一规范，
//! 实际内容由主 `mod.rs` 通过 `nest()` 组合各业务域。
//!
//! 占位路由：返回 404 + 提示信息。生产环境由主 `mod.rs` 接管，
//! 不应在最终部署版本中直接 nest 此模块。

use axum::{http::StatusCode, response::IntoResponse, routing::get, Router};

async fn v1_placeholder() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        "V1 router is a placeholder. Routes are mounted by the main mod.rs.",
    )
}

/// 统一入口（占位）
pub fn routes() -> Router {
    Router::new().route("/", get(v1_placeholder))
}
