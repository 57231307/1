//! 固定资产模型

use serde::{Deserialize, Serialize};

/// 固定资产数据模型
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FixedAsset {
    pub id: i32,
    pub asset_no: String,
    pub asset_name: String,
    pub asset_category: Option<String>,
    pub specification: Option<String>,
    pub location: Option<String>,
    pub original_value: String,
    pub current_value: Option<String>,
    pub useful_life: i32,
    pub depreciation_method: Option<String>,
    pub purchase_date: String,
    pub put_in_date: String,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 资产列表响应
#[derive(Debug, Clone, Deserialize)]
pub struct AssetListResponse {
    pub items: Vec<FixedAsset>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 资产查询参数
#[derive(Debug, Clone, Serialize)]
pub struct AssetQueryParams {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub asset_category: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 创建资产请求
#[derive(Debug, Clone, Serialize)]
pub struct CreateAssetRequest {
    pub asset_no: String,
    pub asset_name: String,
    pub asset_category: Option<String>,
    pub specification: Option<String>,
    pub location: Option<String>,
    pub original_value: String,
    pub useful_life: i32,
    pub depreciation_method: Option<String>,
    pub purchase_date: String,
    pub put_in_date: String,
    pub supplier_id: Option<i32>,
    pub remark: Option<String>,
}

/// 计提折旧请求
#[derive(Debug, Clone, Serialize)]
pub struct DepreciateRequest {
    pub period: String,
}

/// 资产处置请求
#[derive(Debug, Clone, Serialize)]
pub struct DisposalRequest {
    pub disposal_type: String,
    pub disposal_value: String,
    pub disposal_date: String,
    pub reason: String,
    pub buyer_info: Option<String>,
}
