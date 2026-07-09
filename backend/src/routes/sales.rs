//! 销售域路由
//!
//! 处理销售订单、销售合同、销售价格、销售退货、面料销售订单等销售相关接口。
//!
//! P2 2-11 文档标注：本模块中 `POST /resource/:id/{action}` 形式的端点为"动作端点"，
//! 语义上等价于状态变更（submit/approve/cancel 等），RESTful 规范应为 `PATCH /resource/:id` + body `{status}`。
//! 短期保留 POST 动作端点以兼容前端；长期计划重构为 PATCH 统一状态变更语义。

use crate::utils::app_state::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::{
    print_handler, sales_contract_handler, sales_fabric_order_handler,
    sales_order_handler, sales_price_handler, sales_return_handler,
};

/// 销售订单路由（nest 到 /api/v1/erp/sales）
pub fn sales() -> Router<AppState> {
    Router::new()
        .route("/orders", get(sales_order_handler::list_orders))
        .route("/orders", post(sales_order_handler::create_order))
        .route("/orders/:id", get(sales_order_handler::get_order))
        .route("/orders/:id", put(sales_order_handler::update_order))
        .route("/orders/:id", delete(sales_order_handler::delete_order))
        .route(
            "/orders/:id/submit",
            post(sales_order_handler::submit_order),
        )
        .route(
            "/orders/:id/approve",
            post(sales_order_handler::approve_order),
        )
        .route("/orders/:id/ship", post(sales_order_handler::ship_order))
        .route(
            "/orders/:id/complete",
            post(sales_order_handler::complete_order),
        )
        .route(
            "/orders/:id/reject",
            post(sales_order_handler::reject_order),
        )
        .route(
            "/orders/:id/cancel",
            post(sales_order_handler::cancel_order),
        )
        .route(
            "/orders/:id/deliveries",
            get(sales_order_handler::get_order_deliveries)
                .post(sales_order_handler::create_delivery),
        )
        .route(
            "/orders/:id/deliveries/:delivery_id/cancel",
            post(sales_order_handler::cancel_delivery),
        )
        .route(
            "/orders/statistics",
            get(sales_order_handler::get_order_statistics),
        )
        .route(
            "/orders/:id/history",
            get(sales_order_handler::get_order_history),
        )
        .route(
            "/orders/:id/print",
            get(print_handler::sales_order_print_html),
        )
        .route("/orders/export", get(sales_order_handler::export_orders))
        .route(
            "/orders/generate-no",
            get(sales_order_handler::generate_order_no),
        )
        .route(
            "/fabric-orders",
            get(sales_fabric_order_handler::list_fabric_orders),
        )
        .route(
            "/fabric-orders",
            post(sales_fabric_order_handler::create_fabric_order),
        )
        .route(
            "/fabric-orders/:id",
            get(sales_fabric_order_handler::get_fabric_order),
        )
        .route(
            "/fabric-orders/:id",
            put(sales_fabric_order_handler::update_fabric_order),
        )
        .route(
            "/fabric-orders/:id",
            delete(sales_fabric_order_handler::delete_fabric_order),
        )
        .route(
            "/fabric-orders/:id/approve",
            post(sales_fabric_order_handler::approve_fabric_order),
        )
    // P1-4 修复（2026-06-25 综合审计）：移除 quotations 双重路由注册。
    // 销售报价单已由 routes/quotations.rs::routes() 统一挂载至
    // /api/v1/erp/quotations/*（mod.rs:339），该处提供 12 个端点（超集）。
    // 原 sales.rs::quotations() 仅 8 个端点（子集），双重注册导致：
    // - 同一资源暴露在两路径，能力不同，前端调用混乱
    // - 同一报价操作可能命中不同端点，行为不可预测
}

/// 销售合同路由（path 前缀 /sales-contracts）
pub fn sales_contracts() -> Router<AppState> {
    Router::new()
        .route(
            "/sales-contracts",
            get(sales_contract_handler::list_contracts),
        )
        .route(
            "/sales-contracts",
            post(sales_contract_handler::create_contract),
        )
        .route(
            "/sales-contracts/:id",
            get(sales_contract_handler::get_contract),
        )
        .route(
            "/sales-contracts/:id",
            put(sales_contract_handler::update_contract),
        )
        .route(
            "/sales-contracts/:id",
            delete(sales_contract_handler::delete_contract),
        )
        .route(
            "/sales-contracts/:id/approve",
            post(sales_contract_handler::approve_contract),
        )
        .route(
            "/sales-contracts/:id/execute",
            put(sales_contract_handler::execute_contract),
        )
        .route(
            "/sales-contracts/:id/cancel",
            put(sales_contract_handler::cancel_contract),
        )
        .route(
            "/sales-contracts/:id/print",
            get(print_handler::sales_contract_print_html),
        )
}

/// 销售价格路由（path 前缀 /sales-prices）
pub fn sales_prices() -> Router<AppState> {
    Router::new()
        .route("/sales-prices", get(sales_price_handler::list_prices))
        .route("/sales-prices", post(sales_price_handler::create_price))
        .route(
            "/sales-prices/:id",
            get(sales_price_handler::get_price)
                .put(sales_price_handler::update_price)
                .delete(sales_price_handler::delete_price),
        )
        .route(
            "/sales-prices/:id/approve",
            post(sales_price_handler::approve_price),
        )
        .route(
            "/sales-prices/history/:product_id",
            get(sales_price_handler::get_price_history),
        )
        .route(
            "/sales-prices/strategies",
            get(sales_price_handler::list_strategies),
        )
}

/// 销售退货路由（path 前缀 /sales-returns）
///
/// 由 sales_return_handler 模块内部定义路由（router() 函数返回）。
/// 该模块已自带独立前缀，merge 时不会与其他子 router 冲突。
pub fn sales_returns() -> Router<AppState> {
    sales_return_handler::router()
}

/// 销售域统一入口
///
/// 子 router path 已加独立前缀，merge 时 path+method 互不重叠。
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(sales())
        .merge(sales_contracts())
        .merge(sales_prices())
        .merge(sales_returns())
}
