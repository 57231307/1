//! 大货批色审批路由（V15 P0-F15/F16/F17）
//!
//! 9 端点：
//!   GET    /                                 列表
//!   POST   /                                 创建批色记录
//!   GET    /:id                              详情
//!   POST   /:id/cut-sample                   剪大货样（P0-F16）
//!   POST   /:id/send-to-customer             发送客户批色
//!   POST   /:id/approve                      客户批色确认通过（P0-F17）
//!   POST   /:id/reject                        客户批色拒绝
//!   POST   /:id/rework                       客户批色要求返工
//!   POST   /:id/downgrade                    降级处理
//!   POST   /:id/scrap                        报废

use axum::{
    routing::{get, post},
    Router,
};

use crate::handlers::bulk_color_approval_handler;
use crate::utils::app_state::AppState;

/// 大货批色审批路由（nest 到 /api/v1/erp/bulk-color-approvals）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(bulk_color_approval_handler::list_bulk_color_approvals)
                .post(bulk_color_approval_handler::create_bulk_color_approval),
        )
        .route(
            "/:id",
            get(bulk_color_approval_handler::get_bulk_color_approval),
        )
        .route(
            "/:id/cut-sample",
            post(bulk_color_approval_handler::cut_sample),
        )
        .route(
            "/:id/send-to-customer",
            post(bulk_color_approval_handler::send_to_customer),
        )
        .route(
            "/:id/approve",
            post(bulk_color_approval_handler::customer_approve),
        )
        .route(
            "/:id/reject",
            post(bulk_color_approval_handler::customer_reject),
        )
        .route(
            "/:id/rework",
            post(bulk_color_approval_handler::customer_rework),
        )
        .route(
            "/:id/downgrade",
            post(bulk_color_approval_handler::downgrade),
        )
        .route(
            "/:id/scrap",
            post(bulk_color_approval_handler::scrap),
        )
}
