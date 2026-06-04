//! 销售域路由
//!
//! 处理销售订单、销售合同、销售价格、销售退货、面料销售订单等销售相关接口。

use crate::utils::app_state::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::{
    print_handler, sales_contract_handler, sales_fabric_order_handler, sales_order_handler,
    sales_price_handler, sales_return_handler,
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
}

/// 销售合同路由（nest 到 /api/v1/erp/sales-contracts）
pub fn sales_contracts() -> Router<AppState> {
    Router::new()
        .route("/", get(sales_contract_handler::list_contracts))
        .route("/", post(sales_contract_handler::create_contract))
        .route("/:id", get(sales_contract_handler::get_contract))
        .route("/:id", put(sales_contract_handler::update_contract))
        .route("/:id", delete(sales_contract_handler::delete_contract))
        .route(
            "/:id/approve",
            post(sales_contract_handler::approve_contract),
        )
        .route(
            "/:id/execute",
            put(sales_contract_handler::execute_contract),
        )
        .route("/:id/cancel", put(sales_contract_handler::cancel_contract))
        .route("/:id/print", get(print_handler::sales_contract_print_html))
}

/// 销售价格路由（nest 到 /api/v1/erp/sales-prices）
pub fn sales_prices() -> Router<AppState> {
    Router::new()
        .route("/", get(sales_price_handler::list_prices))
        .route("/", post(sales_price_handler::create_price))
        .route(
            "/:id",
            get(sales_price_handler::get_price)
                .put(sales_price_handler::update_price)
                .delete(sales_price_handler::delete_price),
        )
        .route("/:id/approve", post(sales_price_handler::approve_price))
        .route(
            "/history/:product_id",
            get(sales_price_handler::get_price_history),
        )
        .route("/strategies", get(sales_price_handler::list_strategies))
}

/// 销售退货路由（nest 到 /api/v1/erp/sales-returns）
///
/// 由 sales_return_handler 模块内部定义路由（router() 函数返回）
pub fn sales_returns() -> Router<AppState> {
    sales_return_handler::router()
}

/// 销售域统一入口
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(sales())
        .merge(sales_contracts())
        .merge(sales_prices())
        .merge(sales_returns())
}
