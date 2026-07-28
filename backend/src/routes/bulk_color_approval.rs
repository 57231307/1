//! 大货批色审批路由（V15 P0-F15/F16/F17，P1-10 扩展）
//!
//! 16 端点：
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
//!   GET    /:id/history                      批色状态变更历史（P1-10）
//!   GET    /reminders/pending                pending 超时提醒列表（P1-10）
//!   GET    /reminders/followups              客户跟进提醒列表（P1-10）
//!   POST   /reminders/send-pending           发送 pending 超时提醒（P1-10）
//!   POST   /reminders/send-followups         发送客户跟进提醒（P1-10）
//!   GET    /report                           批色报表（P1-10）
//!   GET    /statistics                       批色统计 KPI（P1-10）
//!
//! 路由注册顺序：静态路径（/report、/statistics、/reminders/*）必须在 /:id 之前，
//! 避免 axum matchit 把 "report"/"statistics" 当 :id 匹配。

use axum::{
    routing::{get, post},
    Router,
};

use crate::handlers::bulk_color_approval_handler;
use crate::container::AppState;

/// 大货批色审批路由（nest 到 /api/v1/erp/bulk-color-approvals）
pub fn routes() -> Router<AppState> {
    Router::new()
        // 静态路径必须在 /:id 之前注册，避免 axum matchit 把 "report"/"statistics" 当 :id 匹配
        .route(
            "/",
            get(bulk_color_approval_handler::list_bulk_color_approvals)
                .post(bulk_color_approval_handler::create_bulk_color_approval),
        )
        .route(
            "/report",
            get(bulk_color_approval_handler::report),
        )
        .route(
            "/statistics",
            get(bulk_color_approval_handler::statistics),
        )
        .route(
            "/reminders/pending",
            get(bulk_color_approval_handler::list_pending_reminders),
        )
        .route(
            "/reminders/followups",
            get(bulk_color_approval_handler::list_customer_followups),
        )
        .route(
            "/reminders/send-pending",
            post(bulk_color_approval_handler::send_pending_reminders),
        )
        .route(
            "/reminders/send-followups",
            post(bulk_color_approval_handler::send_customer_followup_reminders),
        )
        // 动态路径：/:id 及其子路径
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
        .route(
            "/:id/history",
            get(bulk_color_approval_handler::list_history),
        )
}
