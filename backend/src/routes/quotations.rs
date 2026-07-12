//! 销售报价单路由
//!
//! 处理销售报价单 CRUD、审批、转换、定价、色号价格等接口。
//! 创建时间: 2026-06-16
//! 关联计划: 2026-06-16-sales-quotation-plan.md Task 3

use axum::{
    // 批次 357 v13 复审 baseline 清零：移除 unused import put
    routing::{get, post},
    Router,
};

use crate::handlers::quotation_handler;
use crate::utils::app_state::AppState;

/// 销售报价单路由（nest 到 /api/v1/erp/quotations）
pub fn routes() -> Router<AppState> {
    Router::new()
        // 列表 + 创建
        .route("/", get(quotation_handler::list_quotations).post(quotation_handler::create_quotation))
        // 详情 + 更新
        .route("/:id", get(quotation_handler::get_quotation).put(quotation_handler::update_quotation))
        // 审批流
        .route("/:id/submit", post(quotation_handler::submit_quotation))
        .route("/:id/approve", post(quotation_handler::approve_quotation))
        .route("/:id/reject", post(quotation_handler::reject_quotation))
        .route("/:id/cancel", post(quotation_handler::cancel_quotation))
        // 转换
        .route("/:id/convert", post(quotation_handler::convert_to_sales_order))
        // 贸易条款
        .route(
            "/:id/terms",
            get(quotation_handler::get_quotation_terms).put(quotation_handler::set_quotation_terms),
        )
        // 状态分类
        .route("/expiring", get(quotation_handler::list_expiring))
        .route("/expired", get(quotation_handler::list_expired))
        // 定价引擎
        .route("/calculate-price", post(quotation_handler::calculate_price))
        // 色号价格
        .route(
            "/color-prices/:product_color_id",
            get(quotation_handler::list_color_prices).post(quotation_handler::set_color_price),
        )
}
