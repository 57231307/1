//! 面料多色号定价扩展路由
//!
//! 16 端点：色号价格 CRUD + 批量调价 + 审批 + 历史 + 计算 + 阶梯价 + 客户专属价 + 季节规则
//! 创建时间: 2026-06-18
//! 关联 spec: docs/superpowers/specs/2026-06-16-color-price-extension-design.md §4.1

use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::handlers::color_price_handler;
use crate::utils::app_state::AppState;

/// 面料多色号定价扩展路由（nest 到 /api/v1/erp/color-prices）
pub fn routes() -> Router<AppState> {
    Router::new()
        // 色号价格 CRUD
        .route(
            "/",
            get(color_price_handler::list_color_prices)
                .post(color_price_handler::create_color_price),
        )
        .route(
            "/:id",
            get(color_price_handler::get_color_price)
                .put(color_price_handler::update_color_price)
                .delete(color_price_handler::delete_color_price),
        )
        // 批量调价 / 审批
        .route(
            "/batch-adjust",
            post(color_price_handler::batch_adjust_color_prices),
        )
        .route(
            "/:id/approve",
            post(color_price_handler::approve_color_price),
        )
        // 价格历史
        .route(
            "/:id/history",
            get(color_price_handler::get_color_price_history),
        )
        // 价格计算
        .route(
            "/calculate",
            get(color_price_handler::calculate_color_price),
        )
        // 阶梯价
        .route(
            "/tiers/:price_id",
            get(color_price_handler::list_tiers)
                .post(color_price_handler::create_tier),
        )
        .route(
            "/tiers/item/:tier_id",
            delete(color_price_handler::delete_tier),
        )
        // 客户专属价
        .route(
            "/customer-special",
            get(color_price_handler::list_customer_special_prices)
                .post(color_price_handler::create_customer_special_price),
        )
        // 季节调价规则
        .route(
            "/seasonal-rules",
            get(color_price_handler::list_seasonal_rules)
                .post(color_price_handler::create_seasonal_rule),
        )
        .route(
            "/seasonal-rules/:id",
            delete(color_price_handler::delete_seasonal_rule),
        )
}
