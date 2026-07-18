//! 色卡仓储管理 - 响应 DTO
//!
//! 设计依据：docs/superpowers/specs/2026-06-16-color-card-design.md §4

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 通用分页响应
#[derive(Debug, Serialize, Clone)]
pub struct PagedResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 色卡列表项
#[derive(Debug, Serialize, Clone)]
pub struct ColorCardListItem {
    pub id: i64,
    pub card_no: String,
    pub card_name: String,
    pub card_type: String,
    pub season: Option<String>,
    pub brand: Option<String>,
    pub total_colors: i32,
    pub status: String,
    pub cover_image_url: Option<String>,
    /// V15 P0-F10：色卡总库存数量
    pub stock_quantity: i32,
    /// V15 P0-F10：已发放数量（可用 = stock_quantity - issued_quantity）
    pub issued_quantity: i32,
    pub created_at: DateTime<Utc>,
}

/// 色卡详情（含色号列表）
#[derive(Debug, Serialize, Clone)]
pub struct ColorCardDetail {
    pub id: i64,
    pub card_no: String,
    pub card_name: String,
    pub card_type: String,
    pub season: Option<String>,
    pub brand: Option<String>,
    pub total_colors: i32,
    pub status: String,
    pub description: Option<String>,
    pub cover_image_url: Option<String>,
    /// V15 P0-F10：色卡总库存数量
    pub stock_quantity: i32,
    /// V15 P0-F10：已发放数量（可用 = stock_quantity - issued_quantity）
    pub issued_quantity: i32,
    pub items: Vec<ColorItemInfo>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 色号信息
#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct ColorItemInfo {
    pub id: i64,
    pub color_code: String,
    pub color_name: String,
    pub rgb_r: i32,
    pub rgb_g: i32,
    pub rgb_b: i32,
    pub cmyk_c: Option<Decimal>,
    pub cmyk_m: Option<Decimal>,
    pub cmyk_y: Option<Decimal>,
    pub cmyk_k: Option<Decimal>,
    pub lab_l: Option<Decimal>,
    pub lab_a: Option<Decimal>,
    pub lab_b: Option<Decimal>,
    pub pantone_code: Option<String>,
    pub cncs_code: Option<String>,
    pub custom_code: Option<String>,
    pub hex_value: String,
    pub dye_recipe_id: Option<i32>,
    pub product_color_price_id: Option<i64>,
    pub swatch_image_url: Option<String>,
    pub sequence: i32,
}

// 借出记录信息（V15 P0-F03 已删除，borrow 模式废弃）
//
// 历史说明：原 BorrowRecordInfo struct 已随 color_card_borrow_records 表重命名为 _legacy
// 而废弃。新的发放记录响应 DTO 在 handlers/color_card/issue.rs 中定义为 IssueRecordInfo。
// 此占位注释保留以说明删除原因，CI 不会因注释报警告。
// 注释类型用 `//`（非 `///`），避免触发 clippy::empty_line_after_doc_comment。

/// 扫码查询响应（包含色彩空间 + 配方 + 价格）
#[derive(Debug, Serialize, Clone)]
pub struct ScanResult {
    pub color_item: ColorItemInfo,
    pub color_card_no: String,
    pub color_card_name: String,
    pub recipe_summary: Option<RecipeSummary>,
    pub price_summary: Option<PriceSummary>,
}

/// 配方摘要（扫码时显示）
#[derive(Debug, Serialize, Clone)]
pub struct RecipeSummary {
    pub id: i64,
    pub recipe_name: String,
    pub fabric_type: Option<String>,
    pub color_no: Option<String>,
    pub temperature: Option<Decimal>,
    pub time_minutes: Option<i32>,
}

/// 价格摘要（扫码时显示）
#[derive(Debug, Serialize, Clone)]
pub struct PriceSummary {
    pub id: i64,
    pub base_price: Decimal,
    pub currency: String,
    pub effective_from: chrono::NaiveDate,
    pub customer_level: Option<String>,
}
