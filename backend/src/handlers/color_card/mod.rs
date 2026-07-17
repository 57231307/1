//! 色卡仓储管理 Handler 模块入口
//!
//! V15 P0-F03~F05 重构：删除借出/归还(borrow)模式，新增发放(issue)模式
//!
//! 拆分结构：
//! - crud.rs：色卡 CRUD 5 端点（list/create/get/update/archive）
//! - items.rs：色号 CRUD + 批量导入 5 端点（list/create/update/delete/batch_import）
//! - issue.rs：发放/归还/遗失/损坏/取消/列表/详情 7 端点（V15 P0-F04 新增）
//! - scan_export.rs：扫码 + 导出 CSV 2 端点
//! - error_map.rs：CrudError/ItemError → AppError 转换（V15 删除 BorrowError 转换）
//! - helpers.rs：ListItemsQuery + Model→DTO + CSV 转义（V15 删除 record_to_info）
//!
//! 实现 14 个 HTTP 端点：色卡 CRUD + 色号 CRUD + 发放/归还/遗失/损坏/取消/列表/详情 + 扫码/导入/导出

pub mod crud;
pub mod error_map;
pub mod helpers;
pub mod issue;
pub mod items;
pub mod scan_export;

pub use crud::{
    archive_color_card, create_color_card, get_color_card, list_color_cards, mark_card_lost,
    update_color_card,
};
pub use issue::{
    cancel_issue, get_issue, issue_color_card, list_issues, mark_issue_damaged, mark_issue_lost,
    return_issue,
};
pub use items::{
    batch_import_items, create_color_item, delete_color_item, list_color_items, update_color_item,
};
pub use scan_export::{export_color_card, scan_color_by_id, scan_color_code};
