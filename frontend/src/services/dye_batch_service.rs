//! 缸号管理服务

use crate::models::api_response::ApiResponse;
use crate::models::dye_batch::{
    CompleteDyeBatchRequest, CreateDyeBatchRequest, DyeBatch, DyeBatchListResponse, DyeBatchQuery,
    UpdateDyeBatchRequest,
};
use crate::services::api::ApiService;

pub struct DyeBatchService;

impl DyeBatchService {
    pub async fn list(query: DyeBatchQuery) -> Result<DyeBatchListResponse, String> {
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

        let url = if params.is_empty() {
            String::from("/dye-batches")
        } else {
            format!("/dye-batches?{}", params.join("&"))
        };
        let response: ApiResponse<DyeBatchListResponse> = ApiService::get(&url).await?;
        response.into_result()
    }

    pub async fn get(id: i32) -> Result<DyeBatch, String> {
        let response: ApiResponse<DyeBatch> =
            ApiService::get(&format!("/dye-batches/{}", id)).await?;
        response.into_result()
    }

    pub async fn create(req: CreateDyeBatchRequest) -> Result<DyeBatch, String> {
        let response: ApiResponse<DyeBatch> = ApiService::post("/dye-batches", &req).await?;
        response.into_result()
    }

    pub async fn update(id: i32, req: UpdateDyeBatchRequest) -> Result<DyeBatch, String> {
        let response: ApiResponse<DyeBatch> =
            ApiService::put(&format!("/dye-batches/{}", id), &req).await?;
        response.into_result()
    }

    pub async fn delete(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/dye-batches/{}", id)).await
    }

    pub async fn complete(id: i32, req: CompleteDyeBatchRequest) -> Result<DyeBatch, String> {
        let response: ApiResponse<DyeBatch> =
            ApiService::post(&format!("/dye-batches/{}/complete", id), &req).await?;
        response.into_result()
    }

    pub async fn get_by_color(color_code: &str) -> Result<Vec<DyeBatch>, String> {
        let response: ApiResponse<Vec<DyeBatch>> =
            ApiService::get(&format!("/dye-batches/by-color/{}", color_code)).await?;
        response.into_result()
    }
}
