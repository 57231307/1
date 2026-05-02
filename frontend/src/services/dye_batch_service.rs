//! 缸号管理服务

use crate::models::api_response::ApiResponse;
use crate::models::dye_batch::{
    CompleteDyeBatchRequest, CreateDyeBatchRequest, DyeBatch, DyeBatchListResponse, DyeBatchQuery,
    UpdateDyeBatchRequest,
};
use crate::services::api::ApiService;
use crate::services::crud_service::CrudService;

pub struct DyeBatchService;

impl CrudService for DyeBatchService {
    type Model = DyeBatch;
    type ListResponse = DyeBatchListResponse;
    type CreateRequest = CreateDyeBatchRequest;
    type UpdateRequest = UpdateDyeBatchRequest;

    fn base_path() -> &'static str {
        "/dye-batch"
    }
}


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
            String::from("/dye-batch")
        } else {
            format!("/dye-batch?{}", params.join("&"))
        };
        let response: ApiResponse<DyeBatchListResponse> = ApiService::get(&url).await?;
        response.into_result()
    }

    pub async fn complete(id: i32, req: CompleteDyeBatchRequest) -> Result<DyeBatch, String> {
        let response: ApiResponse<DyeBatch> = ApiService::post(&format!("/dye-batch/{}/complete", id), &req).await?;
        response.into_result()
    }

    pub async fn get_by_color(color_code: &str) -> Result<Vec<DyeBatch>, String> {
        let response: ApiResponse<Vec<DyeBatch>> = ApiService::get(&format!("/dye-batch/by-color/{}", color_code)).await?;
        response.into_result()
    }
}
