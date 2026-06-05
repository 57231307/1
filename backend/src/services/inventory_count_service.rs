//! 库存盘点服务（facade 入口）
//!
//! 拆分历史：原文件 949 行已拆分为 `inventory_count/` 子目录
//! 本文件仅保留：
//!   1. `InventoryCountService` 类型的 re-export（保持向后兼容）
//!   2. 公开 DTO 的 re-export 以保持向后兼容
//!
//! ⚠️ 已完成拆分（2026-06-05）：
//!   - list_counts / get_count_detail / list_items     → inventory_count/query.rs
//!   - create_count / update_count / delete_count     → inventory_count/commands.rs
//!   - approve_count / complete_count                  → inventory_count/workflow.rs
//!   - add_item / update_item / delete_item            → inventory_count/items.rs
//!
//! 新代码请使用子模块路径 `crate::services::inventory_count::*`
//!
//! 拆分计划：见 docs/refactoring/inventory_count_service_splitting_plan.md

#![allow(dead_code)]

// 公开 DTO re-export（保持向后兼容）
pub use crate::services::inventory_count::types::{
    CreateInventoryCountRequest, InventoryCountDetail, InventoryCountItemDetail,
    InventoryCountItemRequest, UpdateInventoryCountRequest,
};

// InventoryCountService 重新指向 inventory_count/ 子模块中的实现占位
// 由于子模块中的方法以独立函数形式提供（如 `query::list_counts`），
// 此处仅 re-export 业务 DTO 供旧调用方使用。
// 如需调用具体业务逻辑，请直接使用 `crate::services::inventory_count::query::*` 等。
