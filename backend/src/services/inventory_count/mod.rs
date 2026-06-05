//! 库存盘点（Inventory Count）业务模块
//!
//! 模块拆分（拆分自原 inventory_count_service.rs，2026-06-05）：
//! - `types`     — DTO/类型定义
//! - `query`     — 列表查询与详情查询（list_counts, get_count_detail, list_items）
//! - `commands`  — 增删改操作（create_count, update_count, delete_count）
//! - `workflow`  — 审批与完成（approve_count, complete_count）
//! - `items`     — 明细项管理（add_item, update_item, delete_item）

pub mod types;
pub use types::{
    CreateInventoryCountRequest, InventoryCountDetail, InventoryCountItemDetail,
    InventoryCountItemRequest, UpdateInventoryCountRequest,
};

pub mod commands;
pub mod items;
pub mod query;
pub mod workflow;
