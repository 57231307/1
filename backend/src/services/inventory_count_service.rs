//! 库存盘点服务（facade 入口）
//!
//! 拆分历史：原文件 949 行已拆分为 inventory_count/ 子目录
//! 本文件仅保留：
//!   1. `InventoryCountService` 结构体（含全部方法实现）
//!   2. 公开 DTO 的 re-export 以保持向后兼容
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

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ExprTrait, IntoActiveModel,
    Order, PaginatorTrait, QueryFilter, QueryOrder, TransactionTrait,
};
use std::sync::Arc;

use crate::models::dto::PageRequest;
use crate::models::inventory_count::{self, Entity as InventoryCountEntity};
use crate::models::inventory_count_item::{self, Entity as InventoryCountItemEntity};
use crate::models::inventory_stock::{self, Entity as InventoryStockEntity};
use crate::models::inventory_transaction;
use crate::services::inventory_adjustment_service::{
    AdjustmentItemRequest, CreateAdjustmentRequest, InventoryAdjustmentService,
};
use crate::utils::error::AppError;
use crate::utils::PaginatedResponse;

/// 库存盘点服务
pub struct InventoryCountService {
    db: Arc<DatabaseConnection>,
}

// 注：InventoryCountService 的所有方法实现（list_counts、create_count、approve_count 等）
// 保留在 `inventory_count/legacy.rs` 中。该文件保留了原 inventory_count_service.rs 的完整内容。
// 后续 PR 将逐步把各方法迁移到 inventory_count/ 子模块对应文件：
//   - list_counts / get_count_detail / list_items     → inventory_count/query.rs
//   - create_count / update_count / delete_count     → inventory_count/commands.rs
//   - approve_count / complete_count                  → inventory_count/workflow.rs
//   - add_item / update_item / delete_item            → inventory_count/items.rs
include!("inventory_count/legacy.rs");
