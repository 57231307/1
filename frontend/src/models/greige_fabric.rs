//! 坯布管理模型

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GreigeFabric {
    pub id: i32,
    pub fabric_no: String,
    pub fabric_name: String,
    pub fabric_type: String,
    pub color_code: Option<String>,
    pub width_cm: Option<String>,
    pub weight_kg: Option<String>,
    pub length_m: Option<String>,
    pub supplier_id: Option<i32>,
    pub batch_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub location: Option<String>,
    pub status: String,
    pub quality_grade: Option<String>,
    pub purchase_date: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GreigeFabricQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub fabric_no: Option<String>,
    pub fabric_name: Option<String>,
    pub fabric_type: Option<String>,
    pub supplier_id: Option<i32>,
    pub warehouse_id: Option<i32>,
    pub status: Option<String>,
    pub quality_grade: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGreigeFabricRequest {
    pub fabric_no: String,
    pub fabric_name: String,
    pub fabric_type: String,
    pub color_code: Option<String>,
    pub width_cm: Option<String>,
    pub weight_kg: Option<String>,
    pub length_m: Option<String>,
    pub supplier_id: Option<i32>,
    pub batch_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub location: Option<String>,
    pub status: Option<String>,
    pub quality_grade: Option<String>,
    pub purchase_date: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGreigeFabricRequest {
    pub fabric_name: Option<String>,
    pub fabric_type: Option<String>,
    pub color_code: Option<String>,
    pub width_cm: Option<String>,
    pub weight_kg: Option<String>,
    pub length_m: Option<String>,
    pub supplier_id: Option<i32>,
    pub batch_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub location: Option<String>,
    pub status: Option<String>,
    pub quality_grade: Option<String>,
    pub remarks: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockInRequest {
    pub warehouse_id: i32,
    pub location: Option<String>,
    pub weight_kg: String,
    pub length_m: String,
    pub quality_grade: Option<String>,
    pub remarks: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockOutRequest {
    pub weight_kg: Option<String>,
    pub length_m: Option<String>,
    pub remarks: Option<String>,
}

/// 坯布列表响应
#[derive(Debug, Clone, Deserialize)]
pub struct GreigeFabricListResponse {
    pub items: Vec<GreigeFabric>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}
