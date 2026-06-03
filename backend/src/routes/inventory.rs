//! 库存域路由
//!
//! 处理库存、调拨、调整、盘点、预留、批次、物流等库存相关接口。

use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::{
    inventory_adjustment_handler, inventory_batch_handler, inventory_count_handler,
    inventory_reservation_handler, inventory_stock_handler, inventory_transfer_handler,
    logistics_handler, print_handler,
};

/// 库存主路由（nest 到 /api/v1/erp/inventory）
pub fn inventory() -> Router {
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
            get(inventory_stock_handler::list_stock_fabric),
        )
        .route(
            "/stock/fabric",
            post(inventory_stock_handler::create_stock_fabric),
        )
        .route(
            "/stock/transactions",
            get(inventory_stock_handler::list_transactions),
        )
        .route(
            "/stock/summary",
            get(inventory_stock_handler::get_inventory_summary),
        )
        .route(
            "/stock/low-stock",
            get(inventory_stock_handler::check_low_stock),
        )
        .route(
            "/stock/product/:productId",
            get(inventory_stock_handler::get_stock_by_product),
        )
        .route(
            "/stock/alerts",
            get(inventory_stock_handler::get_stock_alerts),
        )
        .route(
            "/transfers",
            get(inventory_transfer_handler::list_transfers),
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
        .route("/counts", get(inventory_count_handler::list_counts))
        .route("/counts", post(inventory_count_handler::create_count))
        .route(
            "/counts/:id",
            get(inventory_count_handler::get_count)
                .put(inventory_count_handler::update_count)
                .delete(inventory_count_handler::delete_count),
        )
        .route(
            "/counts/:id/approve",
            post(inventory_count_handler::approve_count),
        )
        .route(
            "/counts/:id/complete",
            post(inventory_count_handler::complete_count),
        )
        .route(
            "/counts/:id/items",
            get(inventory_count_handler::list_items).post(inventory_count_handler::add_item),
        )
        .route(
            "/counts/items/:item_id",
            put(inventory_count_handler::update_item).delete(inventory_count_handler::delete_item),
        )
        .route(
            "/counts/:id/print",
            get(print_handler::inventory_count_print_html),
        )
        .route(
            "/adjustments",
            get(inventory_adjustment_handler::list_adjustments),
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
}

/// 批次管理路由（nest 到 /api/v1/erp/batches）
pub fn batches() -> Router {
    Router::new()
        .route("/", get(inventory_batch_handler::list_batches))
        .route("/", post(inventory_batch_handler::create_batch))
        .route("/:id", get(inventory_batch_handler::get_batch))
        .route("/:id", put(inventory_batch_handler::update_batch))
        .route("/:id", delete(inventory_batch_handler::delete_batch))
        .route(
            "/:id/transfer",
            post(inventory_batch_handler::transfer_batch),
        )
}

/// 物流管理路由（nest 到 /api/v1/erp/logistics）
pub fn logistics() -> Router {
    Router::new()
        .route("/", get(logistics_handler::list_waybills))
        .route("/", post(logistics_handler::create_waybill))
        .route("/:id", get(logistics_handler::get_waybill))
        .route("/:id", put(logistics_handler::update_waybill_status))
        .route("/:id", delete(logistics_handler::delete_waybill))
}

/// 库存域统一入口
pub fn routes() -> Router {
    Router::new()
        .merge(inventory())
        .merge(batches())
        .merge(logistics())
}
