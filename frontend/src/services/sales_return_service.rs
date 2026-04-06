use crate::models::api_response::{ApiResponse, PaginatedResponse};
use crate::models::sales_return::{SalesReturn, SalesReturnQuery};
use crate::services::api::ApiService;

pub struct SalesReturnService;

impl SalesReturnService {
    pub async fn list(query: SalesReturnQuery) -> Result<PaginatedResponse<SalesReturn>, String> {
        let mut query_params = Vec::new();
        if let Some(page) = query.page {
            query_params.push(format!("page={}", page));
        }
        if let Some(page_size) = query.page_size {
            query_params.push(format!("page_size={}", page_size));
        }
        if let Some(ref status) = query.status {
            query_params.push(format!("status={}", status));
        }
        if let Some(ref return_no) = query.return_no {
            query_params.push(format!("return_no={}", return_no));
        }
        if let Some(customer_id) = query.customer_id {
            query_params.push(format!("customer_id={}", customer_id));
        }

        let query_string = if query_params.is_empty() {
            String::new()
        } else {
            format!("?{}", query_params.join("&"))
        };

        let url = format!("/sales-returns{}", query_string);
        let response: ApiResponse<PaginatedResponse<SalesReturn>> = ApiService::get(&url).await?;
        if response.success {
            Ok(response.data.unwrap_or_else(|| PaginatedResponse {
                data: vec![],
                items: vec![],
                total: 0,
                page: 1,
                page_size: 10,
            }))
        } else {
            Err(response.error.unwrap_or_else(|| "获取列表失败".to_string()))
        }
    }

    pub async fn submit(id: i32) -> Result<SalesReturn, String> {
        let url = format!("/sales-returns/{}/submit", id);
        let empty_body: Option<serde_json::Value> = None;
        let response: ApiResponse<SalesReturn> = ApiService::post(&url, &empty_body).await?;
        if response.success {
            Ok(response.data.unwrap())
        } else {
            Err(response.error.unwrap_or_else(|| "提交失败".to_string()))
        }
    }

    pub async fn approve(id: i32) -> Result<SalesReturn, String> {
        let url = format!("/sales-returns/{}/approve", id);
        let empty_body: Option<serde_json::Value> = None;
        let response: ApiResponse<SalesReturn> = ApiService::post(&url, &empty_body).await?;
        if response.success {
            Ok(response.data.unwrap())
        } else {
            Err(response.error.unwrap_or_else(|| "审批失败".to_string()))
        }
    }
}
