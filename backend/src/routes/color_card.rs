//! 色卡仓储管理路由
//!
//! 16 端点：色卡 CRUD + 色号 CRUD + 借出/归还/遗失/扫码/导入/导出
//! 创建时间: 2026-06-17
//! 关联 spec: docs/superpowers/specs/2026-06-16-color-card-design.md §4.2

use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::color_card_handler;
use crate::utils::app_state::AppState;

/// 色卡仓储管理路由（nest 到 /api/v1/erp/color-cards）
pub fn routes() -> Router<AppState> {
    Router::new()
        // 色卡 CRUD
        .route(
            "/",
            get(color_card_handler::list_color_cards)
                .post(color_card_handler::create_color_card),
        )
        .route(
            "/:id",
            get(color_card_handler::get_color_card)
                .put(color_card_handler::update_color_card)
                .delete(color_card_handler::archive_color_card),
        )
        // 色号 CRUD
        .route(
            "/:id/items",
            get(color_card_handler::list_color_items)
                .post(color_card_handler::create_color_item),
        )
        .route(
            "/:id/items/batch",
            post(color_card_handler::batch_import_items),
        )
        .route(
            "/:id/items/:item_id",
            put(color_card_handler::update_color_item)
                .delete(color_card_handler::delete_color_item),
        )
        // 借出 / 归还 / 遗失 / 损坏
        .route("/borrow", post(color_card_handler::borrow_color_card))
        .route(
            "/return/:record_id",
            post(color_card_handler::return_color_card),
        )
        .route(
            "/lost/:record_id",
            post(color_card_handler::mark_lost_color_card),
        )
        .route(
            "/damaged/:record_id",
            post(color_card_handler::mark_damaged_color_card),
        )
        // 借出历史
        .route(
            "/borrow-records",
            get(color_card_handler::list_borrow_records),
        )
        // 扫码查询
        .route("/scan/:code", get(color_card_handler::scan_color_code))
        // 导出 CSV
        .route(
            "/export/:id",
            get(color_card_handler::export_color_card),
        )
}
