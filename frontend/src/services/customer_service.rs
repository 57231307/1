//! 客户管理服务 API 客户端
//! 提供客户管理相关的 API 调用方法

use crate::models::customer::{
    CreateCustomerRequest, Customer, CustomerListResponse, CustomerQuery, UpdateCustomerRequest,
};
use crate::services::api::ApiService;

pub struct CustomerService;

impl CustomerService {
    pub async fn list(query: CustomerQuery) -> Result<CustomerListResponse, String> {
        let mut params = Vec::new();
        if let Some(page) = query.page {
            params.push(format!("page={}", page));
        }
        if let Some(page_size) = query.page_size {
            params.push(format!("page_size={}", page_size));
        }
        if let Some(ref status) = query.status {
            params.push(format!("status={}", status));
        }
        if let Some(ref customer_type) = query.customer_type {
            params.push(format!("customer_type={}", customer_type));
        }
        if let Some(ref keyword) = query.keyword {
            params.push(format!("keyword={}", keyword));
        }

        let query_string = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };

        ApiService::get(&format!("/customers{}", query_string)).await
    }

    pub async fn get(id: i32) -> Result<Customer, String> {
        ApiService::get(&format!("/customers/{}", id)).await
    }

    pub async fn create(req: CreateCustomerRequest) -> Result<Customer, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        ApiService::post("/customers", &body).await
    }

    pub async fn update(id: i32, req: UpdateCustomerRequest) -> Result<Customer, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        ApiService::put(&format!("/customers/{}", id), &body).await
    }

    pub async fn delete(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/customers/{}", id)).await
    }
}
