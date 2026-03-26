//! 缸号管理服务

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DyeBatch {
    pub id: i32,
    pub batch_no: String,
    pub color_code: String,
    pub color_name: String,
    pub fabric_type: Option<String>,
    pub weight_kg: Option<f64>,
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
    pub weight_kg: Option<f64>,
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
    pub weight_kg: Option<f64>,
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

pub struct DyeBatchService;

impl DyeBatchService {
    pub async fn list(query: DyeBatchQuery) -> Result<Vec<DyeBatch>, String> {
        let mut params = Vec::new();
        if let Some(page) = query.page {
            params.push(format!("page={}", page));
        }
        if let Some(page_size) = query.page_size {
            params.push(format!("page_size={}", page_size));
        }
        if let Some(batch_no) = &query.batch_no {
            params.push(format!("batch_no={}", batch_no));
        }
        if let Some(color_code) = &query.color_code {
            params.push(format!("color_code={}", color_code));
        }
        if let Some(status) = &query.status {
            params.push(format!("status={}", status));
        }
        if let Some(quality_grade) = &query.quality_grade {
            params.push(format!("quality_grade={}", quality_grade));
        }

        let url = format!("/api/v1/erp/dye-batch?{}", params.join("&"));
        ApiService::get(&url).await
    }

    pub async fn get(id: i32) -> Result<DyeBatch, String> {
        let url = format!("/api/v1/erp/dye-batch/{}", id);
        ApiService::get(&url).await
    }

    pub async fn create(req: CreateDyeBatchRequest) -> Result<DyeBatch, String> {
        let url = "/api/v1/erp/dye-batch";
        ApiService::post(url, &req).await
    }

    pub async fn update(id: i32, req: UpdateDyeBatchRequest) -> Result<DyeBatch, String> {
        let url = format!("/api/v1/erp/dye-batch/{}", id);
        ApiService::put(&url, &req).await
    }

    pub async fn delete(id: i32) -> Result<(), String> {
        let url = format!("/api/v1/erp/dye-batch/{}", id);
        ApiService::delete(&url).await
    }

    pub async fn complete(id: i32, req: CompleteDyeBatchRequest) -> Result<DyeBatch, String> {
        let url = format!("/api/v1/erp/dye-batch/{}/complete", id);
        ApiService::post(&url, &req).await
    }

    pub async fn get_by_color(color_code: &str) -> Result<Vec<DyeBatch>, String> {
        let url = format!("/api/v1/erp/dye-batch/by-color/{}", color_code);
        ApiService::get(&url).await
    }
}
