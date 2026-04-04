//! 质量检验模型
//!
//! 质量检验相关的数据结构

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectionStandard {
    pub id: i32,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub inspection_type: String,
    pub sample_size: i32,
    pub acceptance_quality_limit: String,
    pub inspection_level: String,
    pub status: String,
    pub created_by: i32,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectionRecord {
    pub id: i32,
    pub record_number: String,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub batch_number: String,
    pub color_code: Option<String>,
    pub quantity: i32,
    pub qualified_quantity: i32,
    pub unqualified_quantity: i32,
    pub inspection_date: String,
    pub inspector_id: i32,
    pub inspector_name: Option<String>,
    pub inspection_result: String,
    pub remark: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityDefect {
    pub id: i32,
    pub record_id: i32,
    pub defect_type: String,
    pub defect_description: String,
    pub quantity: i32,
    pub severity_level: String,
    pub handling_method: String,
    pub handler_id: Option<i32>,
    pub handler_name: Option<String>,
    pub handling_date: Option<String>,
    pub status: String,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityStatistics {
    pub product_id: i32,
    pub product_name: Option<String>,
    pub total_inspection_count: i64,
    pub total_quantity: i64,
    pub total_qualified_quantity: i64,
    pub total_unqualified_quantity: i64,
    pass_rate: String,
    pub defect_count: i64,
    pub top_defect_types: Vec<DefectTypeCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefectTypeCount {
    pub defect_type: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInspectionStandardRequest {
    pub product_id: i32,
    pub inspection_type: String,
    pub sample_size: i32,
    pub acceptance_quality_limit: String,
    pub inspection_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInspectionRecordRequest {
    pub product_id: i32,
    pub batch_number: String,
    pub color_code: Option<String>,
    pub quantity: i32,
    pub inspection_date: String,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateQualityDefectRequest {
    pub record_id: i32,
    pub defect_type: String,
    pub defect_description: String,
    pub quantity: i32,
    pub severity_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleDefectRequest {
    pub handling_method: String,
    pub handler_id: i32,
}
