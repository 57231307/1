//! 库存盘点模块的 DTO/类型定义
//!
//! 拆分自 inventory_count_service.rs（2026-06-05）

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 库存盘点详情响应
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InventoryCountDetail {
    pub id: i32,
    pub count_no: String,
    pub warehouse_id: i32,
    pub count_date: chrono::DateTime<chrono::Utc>,
    pub status: String,
    pub total_items: i32,
    pub counted_items: i32,
    pub variance_items: i32,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub items: Vec<InventoryCountItemDetail>,
}

/// 库存盘点明细项详情
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InventoryCountItemDetail {
    pub id: i32,
    pub count_id: i32,
    pub product_id: i32,
    pub stock_id: i32,
    pub warehouse_id: i32,
    pub quantity_before: Decimal,
    pub quantity_actual: Decimal,
    pub quantity_difference: Decimal,
    pub unit_cost: Decimal,
    pub total_cost: Decimal,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 创建库存盘点请求
#[derive(Debug, Deserialize, Default)]
pub struct CreateInventoryCountRequest;

#[derive(Debug, Deserialize, Default)]
pub struct InventoryCountItemRequest;

/// 更新库存盘点请求
#[derive(Debug, Deserialize, Default)]
pub struct UpdateInventoryCountRequest;
