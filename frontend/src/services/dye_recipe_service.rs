//! 染色配方管理服务

use crate::services::api::ApiService;
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
    pub temperature: Option<f64>,
    pub time_minutes: Option<i32>,
    pub ph_value: Option<f64>,
    pub liquor_ratio: Option<f64>,
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
    pub temperature: Option<f64>,
    pub time_minutes: Option<i32>,
    pub ph_value: Option<f64>,
    pub liquor_ratio: Option<f64>,
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
    pub temperature: Option<f64>,
    pub time_minutes: Option<i32>,
    pub ph_value: Option<f64>,
    pub liquor_ratio: Option<f64>,
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

pub struct DyeRecipeService;

impl DyeRecipeService {
    pub async fn list(query: DyeRecipeQuery) -> Result<Vec<DyeRecipe>, String> {
        let mut params = Vec::new();
        if let Some(page) = query.page {
            params.push(format!("page={}", page));
        }
        if let Some(page_size) = query.page_size {
            params.push(format!("page_size={}", page_size));
        }
        if let Some(recipe_no) = &query.recipe_no {
            params.push(format!("recipe_no={}", recipe_no));
        }
        if let Some(color_code) = &query.color_code {
            params.push(format!("color_code={}", color_code));
        }
        if let Some(color_name) = &query.color_name {
            params.push(format!("color_name={}", color_name));
        }
        if let Some(dye_type) = &query.dye_type {
            params.push(format!("dye_type={}", dye_type));
        }
        if let Some(status) = &query.status {
            params.push(format!("status={}", status));
        }

        let url = format!("/api/v1/erp/dye-recipe?{}", params.join("&"));
        ApiService::get(&url).await
    }

    pub async fn get(id: i32) -> Result<DyeRecipe, String> {
        let url = format!("/api/v1/erp/dye-recipe/{}", id);
        ApiService::get(&url).await
    }

    pub async fn create(req: CreateDyeRecipeRequest) -> Result<DyeRecipe, String> {
        let url = "/api/v1/erp/dye-recipe";
        ApiService::post(url, &req).await
    }

    pub async fn update(id: i32, req: UpdateDyeRecipeRequest) -> Result<DyeRecipe, String> {
        let url = format!("/api/v1/erp/dye-recipe/{}", id);
        ApiService::put(&url, &req).await
    }

    pub async fn delete(id: i32) -> Result<(), String> {
        let url = format!("/api/v1/erp/dye-recipe/{}", id);
        ApiService::delete(&url).await
    }

    pub async fn approve(id: i32, req: ApproveRecipeRequest) -> Result<DyeRecipe, String> {
        let url = format!("/api/v1/erp/dye-recipe/{}/approve", id);
        ApiService::post(&url, &req).await
    }

    pub async fn create_version(id: i32, req: CreateVersionRequest) -> Result<DyeRecipe, String> {
        let url = format!("/api/v1/erp/dye-recipe/{}/version", id);
        ApiService::post(&url, &req).await
    }

    pub async fn get_by_color(color_code: &str) -> Result<Vec<DyeRecipe>, String> {
        let url = format!("/api/v1/erp/dye-recipe/by-color/{}", color_code);
        ApiService::get(&url).await
    }

    pub async fn get_versions(id: i32) -> Result<Vec<DyeRecipe>, String> {
        let url = format!("/api/v1/erp/dye-recipe/{}/versions", id);
        ApiService::get(&url).await
    }
}
