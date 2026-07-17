//! 色卡仓储管理路由
//!
//! V15 P0-F03~F05 重构：删除借出/归还(borrow)路由，新增发放(issue)路由
//! 16 端点：色卡 CRUD + 色号 CRUD + 发放/归还/遗失/损坏/取消/列表/详情 + 扫码/导入/导出

use axum::{
    routing::{get, post, put},
    Router,
};

use crate::handlers::color_card;
use crate::utils::app_state::AppState;

/// 色卡仓储管理路由（nest 到 /api/v1/erp/color-cards）
pub fn routes() -> Router<AppState> {
    Router::new()
        // 色卡 CRUD
        .route(
            "/",
            get(color_card::list_color_cards)
                .post(color_card::create_color_card),
        )
        .route(
            "/:id",
            get(color_card::get_color_card)
                .put(color_card::update_color_card)
                .delete(color_card::archive_color_card),
        )
        // 直接标记色卡为遗失（不同于发放记录遗失 /issues/:record_id/lost）
        .route(
            "/:id/mark-lost",
            post(color_card::mark_card_lost),
        )
        // 色号 CRUD
        .route(
            "/:id/items",
            get(color_card::list_color_items)
                .post(color_card::create_color_item),
        )
        .route(
            "/:id/items/batch",
            post(color_card::batch_import_items),
        )
        .route(
            "/:id/items/:item_id",
            put(color_card::update_color_item)
                .delete(color_card::delete_color_item),
        )
        // V15 P0-F05：发放 / 归还 / 遗失 / 损坏 / 取消（替代旧 borrow 路由）
        .route("/issues", post(color_card::issue_color_card))
        .route(
            "/issues",
            get(color_card::list_issues),
        )
        .route(
            "/issues/:record_id",
            get(color_card::get_issue),
        )
        .route(
            "/issues/:record_id/return",
            post(color_card::return_issue),
        )
        .route(
            "/issues/:record_id/lost",
            post(color_card::mark_issue_lost),
        )
        .route(
            "/issues/:record_id/damaged",
            post(color_card::mark_issue_damaged),
        )
        .route(
            "/issues/:record_id/cancel",
            post(color_card::cancel_issue),
        )
        // 扫码查询
        .route("/scan/:code", get(color_card::scan_color_code))
        // 按 ID 扫码查询
        .route("/scan-by-id/:id", get(color_card::scan_color_by_id))
        // 导出 CSV
        .route(
            "/export/:id",
            get(color_card::export_color_card),
        )
}
