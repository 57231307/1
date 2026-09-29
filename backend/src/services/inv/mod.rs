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
//! - `fabric_class` 纺织白坯/染色判定与追溯字段校验（全仓唯一实现）
//!
//! 兼容说明：原 `crate::services::inv::*` 路径需要由上层
//! `services/mod.rs` 通过 `pub use super::inv::*;` 重新导出以保持向后兼容。
//!
//! 注意：`move` 与 `return` 同为 Rust 关键字，不能直接作为模块名。
//! 实际文件名为 `inventory_move.rs`（参考 `return_rs.rs` 的命名约定），通过 `as` 别名对外暴露。

use sea_orm::{DatabaseConnection, FromQueryResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod adjust;
pub mod batch;
pub mod count;
pub mod fabric_class;
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
    /// 调拨总金额（对应 `inventory_transfer.total_amount`，实体 NOT NULL，默认 0）
    pub total_amount: rust_decimal::Decimal,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub shipped_at: Option<chrono::DateTime<chrono::Utc>>,
    pub received_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// 调出仓库名（LEFT JOIN warehouses 于 from_warehouse_id 取得）
    pub from_warehouse_name: Option<String>,
    /// 调入仓库名（LEFT JOIN warehouses 于 to_warehouse_id 取得）
    pub to_warehouse_name: Option<String>,
    /// 创建人姓名（LEFT JOIN users 于 created_by 取得 `real_name`）
    pub created_by_name: Option<String>,
    pub items: Vec<InventoryTransferItemDetail>,
}

/// 库存调拨表头读模型（单次 JOIN 富化的查询结果）。
///
/// 与 [`InventoryTransferDetail`] 同构但不含 `items`：明细在详情端点单独查询，
/// 列表端点 `items` 恒为空数组，因此表头查询用本视图 `into_model` 落地，
/// 再组装为 [`InventoryTransferDetail`]。JOIN 派生列一律 `Option<String>`。
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct InventoryTransferView {
    pub id: i32,
    pub transfer_no: String,
    pub from_warehouse_id: i32,
    pub to_warehouse_id: i32,
    pub transfer_date: chrono::DateTime<chrono::Utc>,
    pub status: String,
    pub total_quantity: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub shipped_at: Option<chrono::DateTime<chrono::Utc>>,
    pub received_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub from_warehouse_name: Option<String>,
    pub to_warehouse_name: Option<String>,
    pub created_by_name: Option<String>,
}

/// 库存调拨明细项详情
#[derive(Debug, Serialize, Deserialize, Clone, FromQueryResult)]
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
    // 面料行业追溯字段（v14 批次 417 T-P0-1）：入参已收、库中已存，出参必须如实回传
    pub color_no: String,
    pub dye_lot_no: Option<String>,
    pub batch_no: String,
    // 由 LEFT JOIN products 富化（实体仅存 product_id）；JOIN 派生列一律 Option<String>
    pub product_code: Option<String>,
    pub product_name: Option<String>,
    /// 产品等级（products.product_grade）
    pub grade: Option<String>,
    /// 计量单位（products.unit）
    pub unit: Option<String>,
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
    // P1 batch-18 缺陷 6.2：调拨明细强制缸号追溯字段
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub batch_no: Option<String>,
    pub unit_cost: Option<rust_decimal::Decimal>,
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
