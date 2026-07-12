//! 定制订单全流程跟踪路由
//!
//! 16 端点：CRUD + 流程推进 + 质检 + 售后 + 销售订单转
//! 创建时间: 2026-06-17
//! 关联 spec: docs/superpowers/specs/2026-06-16-custom-order-design.md §3.2

use axum::{
    // 批次 357 v13 复审 baseline 清零：移除 unused import delete
    routing::{get, post, put},
    Router,
};

use crate::handlers::custom_order_handler;
use crate::utils::app_state::AppState;

/// 定制订单路由（nest 到 /api/v1/erp/custom-orders）
pub fn routes() -> Router<AppState> {
    Router::new()
        // 列表 + 创建
        .route(
            "/",
            get(custom_order_handler::list_custom_orders)
                .post(custom_order_handler::create_custom_order),
        )
        // 详情 + 更新
        .route(
            "/:id",
            get(custom_order_handler::get_custom_order)
                .put(custom_order_handler::update_custom_order)
                .delete(custom_order_handler::cancel_custom_order),
        )
        // 流程推进
        .route(
            "/:id/advance",
            post(custom_order_handler::advance_custom_order),
        )
        // 工艺节点
        .route(
            "/:id/nodes",
            get(custom_order_handler::get_timeline)
                .post(custom_order_handler::add_process_node),
        )
        .route(
            "/:id/nodes/:nid",
            put(custom_order_handler::update_process_node),
        )
        .route(
            "/:id/nodes/:nid/advance",
            post(custom_order_handler::advance_process_node),
        )
        .route(
            "/:id/nodes/:nid/logs",
            post(custom_order_handler::add_node_log),
        )
        // 工艺时间线
        .route(
            "/:id/timeline",
            get(custom_order_handler::get_timeline),
        )
        // 质检
        .route(
            "/:id/issues",
            get(custom_order_handler::list_quality_issues)
                .post(custom_order_handler::report_quality_issue),
        )
        .route(
            "/issues/:id/resolve",
            put(custom_order_handler::resolve_quality_issue),
        )
        // 售后
        .route(
            "/:id/after-sales",
            get(custom_order_handler::list_after_sales)
                .post(custom_order_handler::create_after_sales),
        )
        .route(
            "/after-sales/:id",
            put(custom_order_handler::update_after_sales),
        )
}
