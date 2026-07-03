//! V1 API 路由总入口
//!
//! 所有 14 个业务域子模块由主 `mod.rs` 在不同 nest 路径下独立挂载。
//! 本文件保留 `pub fn routes() -> Router` 接口签名以满足统一规范，
//! 实际内容由主 `mod.rs` 通过 `nest()` 组合各业务域。
//!
//! 批次 95 P3-16 修复：移除原占位 404 路由（v1_placeholder），
//! routes() 返回空 Router。/api/v1 根路径请求由 axum 默认 404 处理，
//! 各业务域路由由主 mod.rs 在 /api/v1/erp/* 下独立 nest。

use crate::utils::app_state::AppState;
use axum::Router;

/// 统一入口（空 Router，业务域路由由主 mod.rs nest 组合）
pub fn routes() -> Router<AppState> {
    Router::new()
}
