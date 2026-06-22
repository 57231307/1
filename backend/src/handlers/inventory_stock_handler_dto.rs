//! 库存处理器请求/响应 DTO 结构体
//!
//! 拆分自 inventory_stock_handler.rs：原 7 个 DTO 独立成文件。

use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateStockFabricRequest {
    #[validate(range(min = 1, message = "仓库ID必须大于0"))]
    pub warehouse_id: i32,
    #[validate(range(min = 1, message = "产品ID必须大于0"))]
    pub product_id: i32,
    /// 批次号
    #[validate(length(min = 1, max = 50, message = "批次号长度必须在1-50个字符之间"))]
    pub batch_no: String,
    /// 色号
    #[validate(length(min = 1, max = 50, message = "色号长度必须在1-50个字符之间"))]
    pub color_no: String,
    /// 缸号
    #[validate(length(max = 50, message = "缸号长度不能超过50个字符"))]
    pub dye_lot_no: Option<String>,
    /// 等级
    #[validate(length(min = 1, max = 20, message = "等级长度必须在1-20个字符之间"))]
    pub grade: String,
    /// 数量（米）
    pub quantity_meters: Decimal,
    /// 数量（公斤）- 可选，会自动计算
    pub quantity_kg: Option<Decimal>,
    /// 克重 (g/m²)
    pub gram_weight: Option<Decimal>,
    /// 幅宽 (cm)
    pub width: Option<Decimal>,
    /// 库位 ID
    pub location_id: Option<i32>,
    /// 货架号
    #[validate(length(max = 50, message = "货架号长度不能超过50个字符"))]
    pub shelf_no: Option<String>,
    /// 层号
    #[validate(length(max = 50, message = "层号长度不能超过50个字符"))]
    pub layer_no: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StockResponse {
    pub id: i32,
    pub warehouse_id: i32,
    pub product_id: i32,
    pub quantity_on_hand: Decimal,
    pub quantity_available: Decimal,
    pub quantity_reserved: Decimal,
    pub reorder_point: Decimal,
    pub bin_location: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub struct UpdateStockWithVersionRequest {
    pub quantity_on_hand: Option<Decimal>,
    pub quantity_available: Option<Decimal>,
    pub quantity_reserved: Option<Decimal>,
    pub reorder_point: Option<Decimal>,
    pub reorder_quantity: Option<Decimal>,
    pub bin_location: Option<String>,
    pub version: i32,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ListStockParams {
    #[validate(range(min = 0, message = "页码不能为负数"))]
    pub page: Option<u64>,
    #[validate(range(min = 1, max = 100, message = "每页数量必须在1-100之间"))]
    pub page_size: Option<u64>,
    #[validate(range(min = 1, message = "仓库ID必须大于0"))]
    pub warehouse_id: Option<i32>,
    #[validate(range(min = 1, message = "产品ID必须大于0"))]
    pub product_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct LowStockParams {
    pub warehouse_id: Option<i32>,
    pub product_id: Option<i32>,
    pub batch_no: Option<String>,
}

// ========== 面料行业库存管理接口 ==========

/// 按批次 + 色号查询库存（面料行业版）
pub struct ListStockFabricParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub warehouse_id: Option<i32>,
    pub product_id: Option<i32>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub grade: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StockFabricResponse {
    pub id: i32,
    pub warehouse_id: i32,
    pub product_id: i32,
    pub batch_no: String,
    pub color_no: String,
    pub dye_lot_no: Option<String>,
    pub grade: String,
    pub quantity_on_hand: Decimal,
    pub quantity_available: Decimal,
    pub quantity_reserved: Decimal,
    pub quantity_meters: Decimal,
    pub quantity_kg: Decimal,
    pub gram_weight: Option<Decimal>,
    pub width: Option<Decimal>,
    pub bin_location: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ListTransactionParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub product_id: Option<i32>,
    pub warehouse_id: Option<i32>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub transaction_type: Option<String>,
    pub start_date: Option<chrono::NaiveDateTime>,
    pub end_date: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Serialize)]
pub struct TransactionResponse {
    pub id: i32,
    pub transaction_type: String,
    pub product_id: i32,
    pub warehouse_id: i32,
    pub batch_no: String,
    pub color_no: String,
    pub quantity_meters: Decimal,
    pub quantity_kg: Decimal,
    pub quantity_before_meters: Decimal,
    pub quantity_before_kg: Decimal,
    pub quantity_after_meters: Decimal,
    pub quantity_after_kg: Decimal,
    pub source_bill_type: Option<String>,
    pub source_bill_no: Option<String>,
    pub remarks: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct InventorySummaryItem {
    pub product_id: i32,
    pub product_name: String,
    pub batch_no: String,
    pub color_no: String,
    pub grade: String,
    pub total_quantity_meters: Decimal,
    pub total_quantity_kg: Decimal,
    pub warehouse_name: String,
}
