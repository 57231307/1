//! 缸号管理模型

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DyeBatch {
    pub id: i32,
    pub batch_no: String,
    pub color_code: String,
    pub color_name: String,
    pub fabric_type: Option<String>,
    pub weight_kg: Option<String>,
    pub status: String,
    pub production_date: Option<String>,
    pub completion_date: Option<String>,
    pub quality_grade: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DyeBatchQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub batch_no: Option<String>,
    pub color_code: Option<String>,
    pub status: Option<String>,
    pub quality_grade: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDyeBatchRequest {
    pub batch_no: String,
    pub color_code: String,
    pub color_name: String,
    pub fabric_type: Option<String>,
    pub weight_kg: Option<String>,
    pub status: Option<String>,
    pub production_date: Option<String>,
    pub quality_grade: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDyeBatchRequest {
    pub color_code: Option<String>,
    pub color_name: Option<String>,
    pub fabric_type: Option<String>,
    pub weight_kg: Option<String>,
    pub status: Option<String>,
    pub completion_date: Option<String>,
    pub quality_grade: Option<String>,
    pub remarks: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteDyeBatchRequest {
    pub quality_grade: String,
    pub remarks: Option<String>,
}

/// 缸号列表响应
#[derive(Debug, Clone, Deserialize)]
pub struct DyeBatchListResponse {
    pub items: Vec<DyeBatch>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}
