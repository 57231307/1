//! 库存域路由
//!
//! 处理库存、调拨、调整、盘点、预留、批次、物流等库存相关接口。
//!
//! 路由设计说明：所有子 router 内部 path 都已加上各自独立前缀
//!（`/stock`、`/transfers`、`/batches`、`/logistics` 等），
//! 这样 `routes()` 入口用 `merge` 组合时不会出现 path+method 重叠，
//! 避免 axum 0.7 `Overlapping method route` panic。

use crate::utils::app_state::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

// v11 批次 143 P1-1：inventory_count_handler 已真实实现（盘点单 CRUD + 差异计算 + 审批流）
use crate::handlers::{
    inventory_adjustment_handler, inventory_batch_handler, inventory_count_handler,
    inventory_reservation_handler, inventory_stock_handler, inventory_stock_handler_fabric,
    inventory_stock_handler_query, inventory_transfer_handler, logistics_handler, print_handler,
};

/// 库存主路由（nest 到 /api/v1/erp/inventory）
pub fn inventory() -> Router<AppState> {
    Router::new()
        .route(
            "/piece-split",
            post(crate::handlers::piece_split_handler::split_fabric_piece),
        )
        .route("/stock", get(inventory_stock_handler::list_stock))
        .route("/stock", post(inventory_stock_handler::create_stock))
        .route("/stock/:id", get(inventory_stock_handler::get_stock))
        .route("/stock/:id", put(inventory_stock_handler::update_stock))
        .route("/stock/:id", delete(inventory_stock_handler::delete_stock))
        .route(
            "/stock/fabric",
            get(inventory_stock_handler_fabric::list_stock_fabric),
        )
        .route(
            "/stock/fabric",
            post(inventory_stock_handler_fabric::create_stock_fabric),
        )
        .route(
            "/stock/transactions",
            get(inventory_stock_handler_query::list_transactions),
        )
        .route(
            "/stock/summary",
            get(inventory_stock_handler_query::get_inventory_summary),
        )
        .route(
            "/stock/low-stock",
            get(inventory_stock_handler::check_low_stock),
        )
        .route(
            "/stock/product/:productId",
            get(inventory_stock_handler_query::get_stock_by_product),
        )
        .route(
            "/stock/alerts",
            get(inventory_stock_handler_query::get_stock_alerts),
        )
        .route(
            "/transfers",
            get(inventory_transfer_handler::list_transfers),
        )
        .route(
            "/transfers/generate-no",
            get(inventory_transfer_handler::generate_no),
        )
        .route(
            "/transfers",
            post(inventory_transfer_handler::create_transfer),
        )
        .route(
            "/transfers/:id",
            get(inventory_transfer_handler::get_transfer)
                .put(inventory_transfer_handler::update_transfer)
                .delete(inventory_transfer_handler::delete_transfer),
        )
        .route(
            "/transfers/:id/approve",
            post(inventory_transfer_handler::approve_transfer),
        )
        .route(
            "/transfers/:id/ship",
            post(inventory_transfer_handler::ship_transfer),
        )
        .route(
            "/transfers/:id/receive",
            post(inventory_transfer_handler::receive_transfer),
        )
        .route(
            "/transfers/:id/items",
            get(inventory_transfer_handler::list_items).post(inventory_transfer_handler::add_item),
        )
        .route(
            "/transfers/items/:item_id",
            put(inventory_transfer_handler::update_item)
                .delete(inventory_transfer_handler::delete_item),
        )
        .route(
            "/transfers/:id/print",
            get(print_handler::inventory_transfer_print_html),
        )
        .route(
            "/adjustments",
            get(inventory_adjustment_handler::list_adjustments),
        )
        .route(
            "/adjustments/generate-no",
            get(inventory_adjustment_handler::generate_no),
        )
        .route(
            "/adjustments",
            post(inventory_adjustment_handler::create_adjustment),
        )
        .route(
            "/adjustments/:id",
            get(inventory_adjustment_handler::get_adjustment)
                .put(inventory_adjustment_handler::update_adjustment)
                .delete(inventory_adjustment_handler::delete_adjustment),
        )
        .route(
            "/adjustments/:id/approve",
            post(inventory_adjustment_handler::approve_adjustment),
        )
        .route(
            "/adjustments/:id/reject",
            post(inventory_adjustment_handler::reject_adjustment),
        )
        .route(
            "/adjustments/:id/items",
            get(inventory_adjustment_handler::list_items)
                .post(inventory_adjustment_handler::add_item),
        )
        .route(
            "/adjustments/items/:item_id",
            put(inventory_adjustment_handler::update_item)
                .delete(inventory_adjustment_handler::delete_item),
        )
        .route(
            "/reservations",
            get(inventory_reservation_handler::list_reservations)
                .post(inventory_reservation_handler::create_reservation),
        )
        .route(
            "/reservations/:id",
            delete(inventory_reservation_handler::delete_reservation),
        )
        .route(
            "/reservations/:id/lock",
            post(inventory_reservation_handler::lock_reservation),
        )
        .route(
            "/reservations/:id/release",
            post(inventory_reservation_handler::release_reservation),
        )
        // v11 批次 143 P1-1：库存盘点路由
        .route(
            "/counts",
            get(inventory_count_handler::list_counts)
                .post(inventory_count_handler::create_count),
        )
        .route(
            "/counts/:id",
            get(inventory_count_handler::get_count)
                .put(inventory_count_handler::update_count)
                .delete(inventory_count_handler::delete_count),
        )
        .route(
            "/counts/:id/record",
            post(inventory_count_handler::record_count_items),
        )
        .route(
            "/counts/:id/submit",
            post(inventory_count_handler::submit_for_approval),
        )
        .route(
            "/counts/:id/approve",
            post(inventory_count_handler::approve_count),
        )
        .route(
            "/counts/:id/reject",
            post(inventory_count_handler::reject_count),
        )
}

/// 批次管理路由（path 前缀 /batches）
pub fn batches() -> Router<AppState> {
    Router::new()
        .route("/batches", get(inventory_batch_handler::list_batches))
        .route("/batches", post(inventory_batch_handler::create_batch))
        .route("/batches/:id", get(inventory_batch_handler::get_batch))
        .route("/batches/:id", put(inventory_batch_handler::update_batch))
        .route(
            "/batches/:id",
            delete(inventory_batch_handler::delete_batch),
        )
        .route(
            "/batches/:id/transfer",
            post(inventory_batch_handler::transfer_batch),
        )
}

/// 物流管理路由（path 前缀 /logistics）
pub fn logistics() -> Router<AppState> {
    Router::new()
        .route("/logistics", get(logistics_handler::list_waybills))
        .route("/logistics", post(logistics_handler::create_waybill))
        .route("/logistics/:id", get(logistics_handler::get_waybill))
        .route(
            "/logistics/:id",
            put(logistics_handler::update_waybill_status),
        )
        .route("/logistics/:id", delete(logistics_handler::delete_waybill))
}

/// 库存域统一入口
///
/// 子 router path 已加独立前缀，merge 时 path+method 互不重叠。
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(inventory())
        .merge(batches())
        .merge(logistics())
}
