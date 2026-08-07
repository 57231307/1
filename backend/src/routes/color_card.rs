//! 色卡仓储管理路由
//!
//! V15 P0-F03~F05 重构：删除借出/归还(borrow)路由，新增发放(issue)路由
//! 16 端点：色卡 CRUD + 色号 CRUD + 发放/归还/遗失/损坏/取消/列表/详情 + 扫码/导入/导出

use axum::{
    routing::{get, post, put},
    Router,
};

use crate::container::AppState;
use crate::handlers::color_card;
use crate::handlers::print_handler;

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
        // V15 P1-08-7：色卡发放记录导出 xlsx
        .route("/issues/export", get(color_card::export_issue_records))
        // V15 P2 类九 10.3-3：色卡发放报表（发放明细/汇总/客户台账/过期未使用/订单关联）
        .route(
            "/reports/issue-detail",
            get(color_card::issue_detail_report),
        )
        .route(
            "/reports/issue-detail/export",
            get(color_card::export_issue_detail_report),
        )
        .route(
            "/reports/issue-summary",
            get(color_card::issue_summary_report),
        )
        .route(
            "/reports/customer-ledger/:customer_id",
            get(color_card::customer_color_card_ledger),
        )
        .route(
            "/reports/expired-unused",
            get(color_card::expired_unused_report),
        )
        .route(
            "/reports/order-related/:sales_order_id",
            get(color_card::order_related_report),
        )
        // V15 P2 类九 10.5-2：库存预警
        .route("/warnings", get(color_card::check_all_warnings))
        .route(
            "/warnings/:color_card_id",
            get(color_card::check_single_warning),
        )
        // V15 P2 类九 10.3-4：成本核算（制作成本归集/发放成本结转/取消恢复/过期损失）
        .route(
            "/cost/production/:color_card_id",
            get(color_card::collect_production_cost),
        )
        .route(
            "/cost/issue/:record_id/transfer",
            get(color_card::transfer_issue_cost),
        )
        .route(
            "/cost/issue/:record_id/restore",
            post(color_card::restore_cost_on_cancel),
        )
        .route(
            "/cost/issue/:record_id/expiry-loss",
            get(color_card::calculate_expiry_loss),
        )
        // V15 P2 类九 10.5-3：发放统计（每日日报）
        .route(
            "/statistics/daily",
            get(color_card::generate_daily_stats),
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
        // 打印路由
        .route(
            "/:id/bulk-approval/print",
            get(print_handler::bulk_color_approval_print_docx),
        )
        .route(
            "/:id/issue/print",
            get(print_handler::color_card_issue_print_docx),
        )
        .route(
            "/:id/lab-dip/print",
            get(print_handler::lab_dip_request_print_docx),
        )
}
