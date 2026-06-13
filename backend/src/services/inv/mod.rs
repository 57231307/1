//! 库存服务模块（inv = inventory）
//!
//! 由原 `services/inventory_transfer_service.rs`（1202 行）按业务子领域拆分而来。
//! 子模块：
//! - `move`   调拨单主流程（CRUD / 审核 / 状态机 / 单据号生成）
//! - `batch`  调拨单明细行管理 + 发出/接收批次处理（库存扣减/增加）
//! - `stock`  库存检查辅助逻辑
//! - `adjust` 库存调整（占位模块，详见 `services/inventory_adjustment_service.rs`）
//! - `count`  库存盘点（占位模块，详见 `services/inventory_count_service.rs`）
//! - `hold`   库存预留（占位模块，详见 `services/inventory_reservation_service.rs`）
//!
//! 兼容说明：原 `crate::services::inv::*` 路径需要由上层
//! `services/mod.rs` 通过 `pub use super::inv::*;` 重新导出以保持向后兼容。
//!
//! 注意：`move` 与 `return` 同为 Rust 关键字，不能直接作为模块名。
//! 实际文件名为 `inventory_move.rs`（参考 `return_rs.rs` 的命名约定），通过 `as` 别名对外暴露。

use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod adjust;
pub mod batch;
pub mod count;
pub mod hold;
pub mod inventory_move;
pub mod stock;

// =====================================================
// 共享 DTO（与原 inventory_transfer_service.rs 保持一致）
// =====================================================

/// 库存调拨详情响应
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InventoryTransferDetail {
    pub id: i32,
    pub transfer_no: String,
    pub from_warehouse_id: i32,
    pub to_warehouse_id: i32,
    pub transfer_date: chrono::DateTime<chrono::Utc>,
    pub status: String,
    pub total_quantity: rust_decimal::Decimal,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub shipped_at: Option<chrono::DateTime<chrono::Utc>>,
    pub received_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub items: Vec<InventoryTransferItemDetail>,
}

/// 库存调拨明细项详情
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InventoryTransferItemDetail {
    pub id: i32,
    pub transfer_id: i32,
    pub product_id: i32,
    pub quantity: rust_decimal::Decimal,
    pub shipped_quantity: rust_decimal::Decimal,
    pub received_quantity: rust_decimal::Decimal,
    pub unit_cost: Option<rust_decimal::Decimal>,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 创建库存调拨请求
#[derive(Debug, Deserialize)]
pub struct CreateInventoryTransferRequest {
    pub from_warehouse_id: Option<i32>,
    pub to_warehouse_id: Option<i32>,
    pub transfer_date: Option<chrono::DateTime<chrono::Utc>>,
    pub status: Option<String>,
    pub notes: Option<String>,
    pub items: Option<Vec<InventoryTransferItemRequest>>,
}

#[derive(Debug, Deserialize)]
pub struct InventoryTransferItemRequest {
    pub product_id: Option<i32>,
    pub quantity: Option<rust_decimal::Decimal>,
    pub notes: Option<String>,
}

/// 更新库存调拨请求
#[derive(Debug, Deserialize)]
pub struct UpdateInventoryTransferRequest {
    pub status: Option<String>,
    pub notes: Option<String>,
    pub items: Option<Vec<InventoryTransferItemRequest>>,
}

// =====================================================
// 共享 Service 结构体（子模块均通过 impl InventoryTransferService 扩展）
// =====================================================

/// 库存调拨服务
pub struct InventoryTransferService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl InventoryTransferService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}
