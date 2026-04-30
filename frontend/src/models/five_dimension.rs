//! 五维数据模型
//!
//! 面料五维数据相关的数据结构

use serde::{Deserialize, Serialize};
use serde_json;

/// 五维统计请求参数
#[derive(Debug, Clone, Serialize)]
pub struct FiveDimensionStatsParams {
    pub product_id: Option<i32>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub warehouse_id: Option<i32>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 五维对象
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FiveDimensionItem {
    pub product_id: i32,
    pub product_name: Option<String>,
    pub batch_no: String,
    pub color_no: String,
    pub dye_lot_no: Option<String>,
    pub grade: String,
    pub five_dimension_id: String,
}

/// 仓库分布
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WarehouseDistribution {
    pub warehouse_id: i32,
    pub warehouse_name: String,
    pub quantity_meters: serde_json::Value,
    pub quantity_kg: serde_json::Value,
}

/// 五维统计响应
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct FiveDimensionStatsResponse {
    pub dimension: FiveDimensionItem,
    pub total_meters: serde_json::Value,
    pub total_kg: serde_json::Value,
    pub stock_count: i64,
    pub warehouse_distribution: Vec<WarehouseDistribution>,
}

/// 五维列表响应
#[derive(Debug, Clone, Deserialize)]
pub struct FiveDimensionListResponse {
    pub items: Vec<FiveDimensionStatsResponse>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 五维ID查询请求
#[derive(Debug, Clone, Serialize)]
pub struct FiveDimensionIdRequest {
    pub five_dimension_id: String,
}

/// 五维ID解析响应
#[derive(Debug, Clone, Deserialize)]
pub struct FiveDimensionIdParseResponse {
    pub success: bool,
    pub dimension: Option<FiveDimensionItem>,
    pub error: Option<String>,
}

/// 五维搜索参数
#[derive(Debug, Clone, Serialize)]
pub struct FiveDimensionSearchParams {
    pub keyword: String,
    pub search_type: String,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 五维搜索结果
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct FiveDimensionSearchResponse {
    pub items: Vec<FiveDimensionItem>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}
