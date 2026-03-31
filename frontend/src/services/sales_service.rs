use crate::models::sales::{CreateSalesOrderRequest, SalesOrder, SalesOrderListResponse, UpdateSalesOrderRequest};
use crate::services::api::ApiService;

/// 销售服务
pub struct SalesService;

impl SalesService {
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

    pub async fn delete_order(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/sales/orders/{}", id)).await
    }
}
