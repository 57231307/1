use crate::models::api_response::ApiResponse;
use crate::models::sales::{
    CreateSalesOrderRequest, SalesOrder, SalesOrderListResponse, UpdateSalesOrderRequest,
};
use crate::services::api::ApiService;

/// 销售服务
pub struct SalesService;

impl SalesService {
    pub async fn submit_order(id: i32) -> Result<SalesOrder, String> {
        let url = format!("/sales/orders/{}/submit", id);
        let empty_body: Option<serde_json::Value> = None;
        let response: ApiResponse<SalesOrder> = ApiService::post(&url, &empty_body).await?;
        if response.success {
            Ok(response.data.unwrap())
        } else {
            Err(response.error.unwrap_or_else(|| "提交失败".to_string()))
        }
    }

    pub async fn list_orders() -> Result<SalesOrderListResponse, String> {
        ApiService::get::<SalesOrderListResponse>("/sales/orders").await
    }

    pub async fn get_order(id: i32) -> Result<SalesOrder, String> {
        ApiService::get::<SalesOrder>(&format!("/sales/orders/{}", id)).await
    }

    pub async fn create_order(req: CreateSalesOrderRequest) -> Result<SalesOrder, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/sales/orders", &payload).await
    }

    pub async fn update_order(id: i32, req: UpdateSalesOrderRequest) -> Result<SalesOrder, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/sales/orders/{}", id), &payload).await
    }

    pub async fn approve_order(id: i32) -> Result<SalesOrder, String> {
        ApiService::post(
            &format!("/sales/orders/{}/approve", id),
            &serde_json::json!({}),
        )
        .await
    }

    pub async fn ship_order(
        id: i32,
        req: crate::models::sales::ShipOrderRequest,
    ) -> Result<SalesOrder, String> {
        ApiService::post(
            &format!("/sales/orders/{}/ship", id),
            &serde_json::to_value(req).unwrap_or_default(),
        )
        .await
    }

    pub async fn complete_order(id: i32) -> Result<SalesOrder, String> {
        ApiService::post(
            &format!("/sales/orders/{}/complete", id),
            &serde_json::json!({}),
        )
        .await
    }

    pub async fn delete_order(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/sales/orders/{}", id)).await
    }
}
