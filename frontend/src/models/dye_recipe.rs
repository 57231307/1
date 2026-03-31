//! 染色配方管理模型

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DyeRecipe {
    pub id: i32,
    pub recipe_no: String,
    pub color_code: String,
    pub color_name: String,
    pub fabric_type: Option<String>,
    pub dye_type: Option<String>,
    pub chemical_formula: Option<String>,
    pub temperature: Option<String>,
    pub time_minutes: Option<i32>,
    pub ph_value: Option<String>,
    pub liquor_ratio: Option<String>,
    pub auxiliaries: Option<serde_json::Value>,
    pub status: String,
    pub version: Option<i32>,
    pub parent_recipe_id: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DyeRecipeQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub recipe_no: Option<String>,
    pub color_code: Option<String>,
    pub color_name: Option<String>,
    pub dye_type: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDyeRecipeRequest {
    pub recipe_no: String,
    pub color_code: String,
    pub color_name: String,
    pub fabric_type: Option<String>,
    pub dye_type: Option<String>,
    pub chemical_formula: Option<String>,
    pub temperature: Option<String>,
    pub time_minutes: Option<i32>,
    pub ph_value: Option<String>,
    pub liquor_ratio: Option<String>,
    pub auxiliaries: Option<serde_json::Value>,
    pub status: Option<String>,
    pub version: Option<i32>,
    pub parent_recipe_id: Option<i32>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDyeRecipeRequest {
    pub color_code: Option<String>,
    pub color_name: Option<String>,
    pub fabric_type: Option<String>,
    pub dye_type: Option<String>,
    pub chemical_formula: Option<String>,
    pub temperature: Option<String>,
    pub time_minutes: Option<i32>,
    pub ph_value: Option<String>,
    pub liquor_ratio: Option<String>,
    pub auxiliaries: Option<serde_json::Value>,
    pub status: Option<String>,
    pub remarks: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveRecipeRequest {
    pub approved_by: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVersionRequest {
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 染色配方列表响应
#[derive(Debug, Clone, Deserialize)]
pub struct DyeRecipeListResponse {
    pub items: Vec<DyeRecipe>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}
