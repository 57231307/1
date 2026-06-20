//! 色卡仓储管理 Handler 模块入口
//!
//! 任务编号: P14 批 2 I-3 第 9 批
//! 拆分原 handlers/color_card_handler.rs（590 行 → 4 子模块 + helpers）
//!
//! 拆分结构：
//! - crud.rs：色卡 CRUD 5 端点（list/create/get/update/archive）
//! - items.rs：色号 CRUD + 批量导入 5 端点（list/create/update/delete/batch_import）
//! - borrow.rs：借出/归还/遗失/损坏/历史 5 端点
//! - scan_export.rs：扫码 + 导出 CSV 2 端点
//! - error_map.rs：CrudError/ItemError/BorrowError → AppError 转换
//! - helpers.rs：ListItemsQuery + Model→DTO + CSV 转义
//!
//! 实现 13 个 HTTP 端点：色卡 CRUD + 色号 CRUD + 借出/归还/遗失/扫码/导入/导出
//! 设计依据：docs/superpowers/specs/2026-06-16-color-card-design.md §4.2
//! 行为完全保持一致（仅结构重构）

pub mod borrow;
pub mod crud;
pub mod error_map;
pub mod helpers;
pub mod items;
pub mod scan_export;

pub use borrow::{
    borrow_color_card, list_borrow_records, mark_damaged_color_card, mark_lost_color_card,
    return_color_card,
};
pub use crud::{
    archive_color_card, create_color_card, get_color_card, list_color_cards, update_color_card,
};
pub use items::{
    batch_import_items, create_color_item, delete_color_item, list_color_items, update_color_item,
};
pub use scan_export::{export_color_card, scan_color_code};
