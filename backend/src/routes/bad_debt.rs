//! 坏账管理路由（V15 P0-B01/B02 Batch 481）
//!
//! 12 端点：
//!   **B01 坏账准备计提**：
//!   - POST   /run-provision              期末触发计提
//!   - GET    /                            计提记录列表
//!   - POST   /                            （B02 占位用 writeoffs 子路径）
//!   - GET    /:id                         计提记录详情
//!   - POST   /:id/confirm                确认计提
//!   - POST   /:id/reverse                转回计提
//!
//!   **B02 坏账核销审批**：
//!   - POST   /writeoffs                  申请核销
//!   - GET    /writeoffs                   核销申请列表
//!   - GET    /writeoffs/:id              核销申请详情
//!   - POST   /writeoffs/:id/finance-approve    一级审批
//!   - POST   /writeoffs/:id/general-manager-approve  二级审批
//!   - POST   /writeoffs/:id/reject            拒绝
//!   - POST   /writeoffs/:id/cancel           取消
//!
//! 路由注册顺序：静态路径（/run-provision、/writeoffs、/writeoffs）必须在 /:id 之前，
//! 避免 axum matchit 把 "run-provision"/"writeoffs" 当 :id 匹配。

use axum::{
    routing::{get, post},
    Router,
};

use crate::handlers::bad_debt_handler;
use crate::utils::app_state::AppState;

/// 坏账管理路由（nest 到 /api/v1/erp/bad-debts）
pub fn routes() -> Router<AppState> {
    Router::new()
        // 静态路径必须在 /:id 之前注册，避免 axum matchit 把 "run-provision" 当 :id 匹配
        .route("/run-provision", post(bad_debt_handler::run_provision))
        .route(
            "/writeoffs",
            get(bad_debt_handler::list_writeoffs).post(bad_debt_handler::create_writeoff),
        )
        .route("/", get(bad_debt_handler::list_provisions))
        .route("/:id", get(bad_debt_handler::get_provision))
        .route("/:id/confirm", post(bad_debt_handler::confirm_provision))
        .route("/:id/reverse", post(bad_debt_handler::reverse_provision))
        .route(
            "/writeoffs/:id",
            get(bad_debt_handler::get_writeoff),
        )
        .route(
            "/writeoffs/:id/finance-approve",
            post(bad_debt_handler::finance_approve),
        )
        .route(
            "/writeoffs/:id/general-manager-approve",
            post(bad_debt_handler::general_manager_approve),
        )
        .route(
            "/writeoffs/:id/reject",
            post(bad_debt_handler::reject_writeoff),
        )
        .route(
            "/writeoffs/:id/cancel",
            post(bad_debt_handler::cancel_writeoff),
        )
}
