//! 库存处理器请求/响应 DTO 结构体
//!
//! 拆分自 inventory_stock_handler.rs：原 7 个 DTO 独立成文件。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[allow(dead_code, reason = "反序列化输入字段")]
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

#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Serialize)]
pub struct StockResponse {
    pub id: i32,
    pub warehouse_id: i32,
    pub product_id: i32,
    pub quantity_on_hand: Decimal,
    pub quantity_available: Decimal,
    pub quantity_reserved: Decimal,
    pub reorder_point: Decimal,
    /// 库存上限（v11 批次 144 P1-4：新增，支持 OverStock 告警阈值展示）
    pub max_stock_point: Decimal,
    pub bin_location: Option<String>,
    // ===== 面料行业四维库存维度（产品→批次/匹号→色号→缸号）=====
    // 库存行按这组维度唯一区分，响应缺它们时前端四维列表与四维查询都无法成立
    /// 批次号（面料匹号/批次）
    pub batch_no: String,
    /// 色号
    pub color_no: String,
    /// 缸号
    pub dye_lot_no: Option<String>,
    /// 等级（一等品/二等品/等外品）
    pub grade: String,
    /// 库存状态（正常/报废/已删除，见 models::status::purchase_inventory::inventory_stock_status；
    /// 冻结/待检从未有写入方，前端不得提供这两个筛选项）
    pub stock_status: String,
    /// 质量状态（合格/不合格/待检）
    pub quality_status: String,
    /// 已发货数量（销售发货累计）
    pub quantity_shipped: Decimal,
    /// 在途数量（采购收货累计）
    pub quantity_incoming: Decimal,
    /// 数量（米，主计量）
    pub quantity_meters: Decimal,
    /// 数量（公斤，辅计量）
    pub quantity_kg: Decimal,
    // ===== 主数据名称（库存表只存 ID，列表/详情/导出按名称展示）=====
    /// 产品编码
    pub product_code: Option<String>,
    /// 产品名称
    pub product_name: Option<String>,
    /// 仓库名称
    pub warehouse_name: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateStockWithVersionRequest {
    pub quantity_on_hand: Option<Decimal>,
    pub quantity_available: Option<Decimal>,
    pub quantity_reserved: Option<Decimal>,
    pub reorder_point: Option<Decimal>,
    /// 库存上限（v11 批次 144 P1-4：新增，支持 OverStock 告警阈值配置）
    pub max_stock_point: Option<Decimal>,
    pub reorder_quantity: Option<Decimal>,
    pub bin_location: Option<String>,
    pub version: i32,
}

#[allow(dead_code, reason = "反序列化输入字段")]
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
    /// 产品编码/名称关键词（列表筛选栏的输入框，与导出共用同一口径）
    pub keyword: Option<String>,
    /// 色号筛选（面料四维查询维度之一）
    pub color_no: Option<String>,
    /// 缸号筛选（面料四维查询维度之一）
    pub dye_lot_no: Option<String>,
    /// 批次/匹号筛选（面料四维查询维度之一）
    pub batch_no: Option<String>,
    /// 库存状态筛选（取 inventory_stocks.stock_status 的真实主数据值：正常/报废/已删除）；
    /// 不传时按在库口径排除软删除行
    pub stock_status: Option<String>,
}

/// 库存预警行（GET /inventory/stock/alerts 的出参单元）
///
/// 数量类字段是 Decimal 的字符串序列化（与 `StockResponse` 同口径）；产品编码/名称/单位/仓库名称
/// 按 ID 批量带出——预警要能被处置（去补货、找仓管），只给 ID 就成不了决策依据。
/// `alert_type` 取值见 `services::stock_alert::compute_alert_type`，前端词表在
/// `constants/stock-alert-type.ts`，两侧必须同步。
#[derive(Debug, Serialize)]
pub struct StockAlertRow {
    pub id: i32,
    pub product_id: i32,
    pub product_code: Option<String>,
    pub product_name: Option<String>,
    /// 产品计量单位（products.unit）
    pub unit: Option<String>,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    pub quantity_on_hand: String,
    pub quantity_available: String,
    pub quantity_reserved: String,
    /// 补货点（低于它即 low_stock 告警）
    pub reorder_point: String,
    pub max_stock_point: String,
    pub expiry_date: Option<String>,
    pub last_movement_date: Option<String>,
    /// 台账状态（正常/报废/已删除）
    pub stock_status: String,
    pub alert_type: String,
}

/// 预警列表查询入参：分页真实生效（此前 page/page_size 被完全忽略并返回全量）
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct StockAlertQuery {
    pub page: Option<u64>,
    #[serde(rename = "page_size")]
    pub page_size: Option<u64>,
    pub warehouse_id: Option<i32>,
    pub product_id: Option<i32>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct LowStockParams {
    pub warehouse_id: Option<i32>,
    pub product_id: Option<i32>,
    pub batch_no: Option<String>,
}

// ========== 面料行业库存管理接口 ==========

/// 按批次 + 色号查询库存（面料行业版）
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ListStockFabricParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub warehouse_id: Option<i32>,
    pub product_id: Option<i32>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub grade: Option<String>,
}

#[allow(dead_code, reason = "序列化输出字段")]
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

#[allow(dead_code, reason = "反序列化输入字段")]
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

#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Clone, Serialize)]
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

#[allow(dead_code, reason = "序列化输出字段")]
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
