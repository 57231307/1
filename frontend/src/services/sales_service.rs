use crate::services::api::ApiService;

/// 销售订单数据模型
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SalesOrder {
    pub id: i32,
    pub order_no: String,
    pub customer_name: String,
    pub total_amount: f64,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 销售订单列表响应
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SalesOrderListResponse {
    pub orders: Vec<SalesOrder>,
    pub total: u64,
}

/// 创建销售订单请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateSalesOrderRequest {
    pub order_no: String,
    pub customer_name: String,
    pub total_amount: f64,
}

/// 更新销售订单请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateSalesOrderRequest {
    pub order_no: Option<String>,
    pub customer_name: Option<String>,
    pub total_amount: Option<f64>,
    pub status: Option<String>,
}

/// 销售服务
pub struct SalesService;

impl SalesService {
    pub async fn list_orders() -> Result<SalesOrderListResponse, String> {
        ApiService::get::<SalesOrderListResponse>("/api/v1/erp/sales/orders").await
    }

    pub async fn get_order(id: i32) -> Result<SalesOrder, String> {
        ApiService::get::<SalesOrder>(&format!("/api/v1/erp/sales/orders/{}", id)).await
    }

    pub async fn create_order(req: CreateSalesOrderRequest) -> Result<SalesOrder, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/api/v1/erp/sales/orders", &payload).await
    }

    pub async fn update_order(id: i32, req: UpdateSalesOrderRequest) -> Result<SalesOrder, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/api/v1/erp/sales/orders/{}", id), &payload).await
    }

    pub async fn delete_order(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/api/v1/erp/sales/orders/{}", id)).await
    }
}
