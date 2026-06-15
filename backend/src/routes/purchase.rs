//! 采购域路由
//!
//! 处理采购订单、采购合同、采购价格、采购收货、采购检验、采购退货、供应商、供应商评估等采购相关接口。

use crate::utils::app_state::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::{
    print_handler, purchase_contract_handler, purchase_inspection_handler, purchase_order_handler,
    purchase_price_handler, purchase_receipt_handler, purchase_return_handler,
    supplier_evaluation_handler, supplier_handler,
};

/// 采购订单路由（nest 到 /api/v1/erp/purchases）
pub fn purchases() -> Router<AppState> {
    Router::new()
        .route(
            "/orders/delivery-date",
            post(purchase_order_handler::calculate_delivery_date),
        )
        .route("/orders", get(purchase_order_handler::list_orders))
        .route("/orders", post(purchase_order_handler::create_order))
        .route("/orders/:id", get(purchase_order_handler::get_order))
        .route("/orders/:id", put(purchase_order_handler::update_order))
        .route("/orders/:id", delete(purchase_order_handler::delete_order))
        .route(
            "/orders/:id/approve",
            post(purchase_order_handler::approve_order),
        )
        .route(
            "/orders/:id/submit",
            post(purchase_order_handler::submit_order),
        )
        .route(
            "/orders/:id/reject",
            post(purchase_order_handler::reject_order),
        )
        .route(
            "/orders/:id/close",
            post(purchase_order_handler::close_order),
        )
        .route("/orders/export", get(purchase_order_handler::export_orders))
        .route(
            "/orders/:id/items",
            get(purchase_order_handler::list_order_items)
                .post(purchase_order_handler::create_order_item),
        )
        .route(
            "/orders/:id/items/:item_id",
            put(purchase_order_handler::update_order_item)
                .delete(purchase_order_handler::delete_order_item),
        )
        .route(
            "/orders/:id/print",
            get(print_handler::purchase_order_print_html),
        )
        .route("/receipts", get(purchase_receipt_handler::list_receipts))
        .route(
            "/receipts/generate-no",
            get(purchase_receipt_handler::generate_no),
        )
        .route(
            "/receipts/:id/print",
            get(print_handler::purchase_receipt_print_html),
        )
        .route("/receipts", post(purchase_receipt_handler::create_receipt))
        .route("/receipts/:id", get(purchase_receipt_handler::get_receipt))
        .route(
            "/receipts/:id",
            put(purchase_receipt_handler::update_receipt)
                .delete(purchase_receipt_handler::delete_receipt),
        )
        .route(
            "/receipts/:id/confirm",
            post(purchase_receipt_handler::confirm_receipt),
        )
        .route(
            "/receipts/:id/items",
            get(purchase_receipt_handler::list_receipt_items)
                .post(purchase_receipt_handler::create_receipt_item),
        )
        .route(
            "/receipts/:id/items/:item_id",
            put(purchase_receipt_handler::update_receipt_item)
                .delete(purchase_receipt_handler::delete_receipt_item),
        )
        .route(
            "/inspections",
            get(purchase_inspection_handler::list_inspections),
        )
        .route(
            "/inspections",
            post(purchase_inspection_handler::create_inspection),
        )
        .route(
            "/inspections/:id",
            get(purchase_inspection_handler::get_inspection),
        )
        .route(
            "/inspections/:id",
            put(purchase_inspection_handler::update_inspection),
        )
        .route(
            "/inspections/:id/complete",
            post(purchase_inspection_handler::complete_inspection),
        )
        .route(
            "/inspections/:id/items",
            get(purchase_inspection_handler::list_inspection_items)
                .post(purchase_inspection_handler::create_inspection_item),
        )
        .route(
            "/inspections/:id/items/:item_id",
            put(purchase_inspection_handler::update_inspection_item)
                .delete(purchase_inspection_handler::delete_inspection_item),
        )
        .route(
            "/returns",
            get(purchase_return_handler::list_purchase_returns),
        )
        .route(
            "/returns",
            post(purchase_return_handler::create_purchase_return),
        )
        .route(
            "/returns/:id",
            get(purchase_return_handler::get_purchase_return),
        )
        .route(
            "/returns/:id",
            put(purchase_return_handler::update_purchase_return),
        )
        .route(
            "/returns/:id",
            delete(purchase_return_handler::delete_purchase_return),
        )
        .route(
            "/returns/:id/submit",
            post(purchase_return_handler::submit_purchase_return),
        )
        .route(
            "/returns/:id/approve",
            post(purchase_return_handler::approve_purchase_return),
        )
        .route(
            "/returns/:id/reject",
            post(purchase_return_handler::reject_purchase_return),
        )
        .route(
            "/returns/:id/items",
            get(purchase_return_handler::list_purchase_return_items),
        )
        .route(
            "/returns/:id/items",
            post(purchase_return_handler::create_purchase_return_item),
        )
        .route(
            "/returns/:id/items/:item_id",
            put(purchase_return_handler::update_purchase_return_item),
        )
        .route(
            "/returns/:id/items/:item_id",
            delete(purchase_return_handler::delete_purchase_return_item),
        )
}

/// 采购合同路由（path 前缀 /purchase-contracts）
pub fn purchase_contracts() -> Router<AppState> {
    Router::new()
        .route(
            "/purchase-contracts",
            get(purchase_contract_handler::list_contracts),
        )
        .route(
            "/purchase-contracts",
            post(purchase_contract_handler::create_contract),
        )
        .route(
            "/purchase-contracts/:id",
            get(purchase_contract_handler::get_contract),
        )
        .route(
            "/purchase-contracts/:id",
            put(purchase_contract_handler::update_contract),
        )
        .route(
            "/purchase-contracts/:id",
            delete(purchase_contract_handler::delete_contract),
        )
        .route(
            "/purchase-contracts/:id/approve",
            post(purchase_contract_handler::approve_contract),
        )
        .route(
            "/purchase-contracts/:id/execute",
            put(purchase_contract_handler::execute_contract),
        )
        .route(
            "/purchase-contracts/:id/cancel",
            put(purchase_contract_handler::cancel_contract),
        )
}

/// 采购价格路由（path 前缀 /purchase-prices）
pub fn purchase_prices() -> Router<AppState> {
    Router::new()
        .route("/purchase-prices", get(purchase_price_handler::list_prices))
        .route(
            "/purchase-prices",
            post(purchase_price_handler::create_price),
        )
        .route(
            "/purchase-prices/history/:product_id",
            get(purchase_price_handler::get_price_history_by_product),
        )
        .route(
            "/purchase-prices/:id",
            get(purchase_price_handler::get_price),
        )
        .route(
            "/purchase-prices/:id",
            put(purchase_price_handler::update_price),
        )
        .route(
            "/purchase-prices/:id",
            delete(purchase_price_handler::delete_price),
        )
        .route(
            "/purchase-prices/:id/approve",
            post(purchase_price_handler::approve_price),
        )
        .route(
            "/purchase-prices/:id/history",
            get(purchase_price_handler::get_price_history),
        )
}

/// 供应商路由（path 前缀 /suppliers）
pub fn suppliers() -> Router<AppState> {
    Router::new()
        .route("/suppliers", get(supplier_handler::list_suppliers))
        .route("/suppliers", post(supplier_handler::create_supplier))
        .route("/suppliers/select", get(supplier_handler::list_suppliers))
        .route("/suppliers/:id", get(supplier_handler::get_supplier))
        .route("/suppliers/:id", put(supplier_handler::update_supplier))
        .route("/suppliers/:id", delete(supplier_handler::delete_supplier))
        .route(
            "/suppliers/:id/status",
            post(supplier_handler::toggle_supplier_status),
        )
        .route(
            "/suppliers/:id/contacts",
            get(supplier_handler::list_supplier_contacts)
                .post(supplier_handler::create_supplier_contact),
        )
        .route(
            "/suppliers/:id/contacts/:contact_id",
            put(supplier_handler::update_supplier_contact)
                .delete(supplier_handler::delete_supplier_contact),
        )
        .route(
            "/suppliers/:id/qualifications",
            get(supplier_handler::list_supplier_qualifications)
                .post(supplier_handler::create_supplier_qualification),
        )
        .route(
            "/suppliers/:id/evaluate",
            post(supplier_evaluation_handler::create_evaluation_record),
        )
        .route(
            "/suppliers/:id/evaluations",
            get(supplier_evaluation_handler::list_evaluation_records),
        )
}

/// 供应商评估路由（path 前缀 /supplier-evaluations）
pub fn supplier_evaluations() -> Router<AppState> {
    Router::new()
        .route(
            "/supplier-evaluations",
            get(supplier_evaluation_handler::list_evaluations),
        )
        .route(
            "/supplier-evaluations",
            post(supplier_evaluation_handler::create_evaluation),
        )
        .route(
            "/supplier-evaluations/suppliers/:supplier_id/score",
            get(supplier_evaluation_handler::get_supplier_score_by_path),
        )
        .route(
            "/supplier-evaluations/:id",
            get(supplier_evaluation_handler::get_evaluation),
        )
        .route(
            "/supplier-evaluations/:id",
            put(supplier_evaluation_handler::update_evaluation),
        )
        .route(
            "/supplier-evaluations/:id",
            delete(supplier_evaluation_handler::delete_evaluation),
        )
        .route(
            "/supplier-evaluations/indicators",
            get(supplier_evaluation_handler::list_indicators),
        )
        .route(
            "/supplier-evaluations/indicators",
            post(supplier_evaluation_handler::create_indicator),
        )
        .route(
            "/supplier-evaluations/rankings",
            get(supplier_evaluation_handler::get_rankings),
        )
        .route(
            "/supplier-evaluations/records",
            get(supplier_evaluation_handler::list_evaluation_records),
        )
        .route(
            "/supplier-evaluations/records",
            post(supplier_evaluation_handler::create_evaluation_record),
        )
        .route(
            "/supplier-evaluations/records/:id",
            get(supplier_evaluation_handler::get_evaluation_record),
        )
        .route(
            "/supplier-evaluations/scores/:supplier_id",
            get(supplier_evaluation_handler::get_supplier_score),
        )
        .route(
            "/supplier-evaluations/ratings",
            get(supplier_evaluation_handler::list_ratings),
        )
}

/// 采购域统一入口
///
/// 子 router path 已加独立前缀，merge 时 path+method 互不重叠。
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(purchases())
        .merge(purchase_contracts())
        .merge(purchase_prices())
        .merge(suppliers())
        .merge(supplier_evaluations())
}
