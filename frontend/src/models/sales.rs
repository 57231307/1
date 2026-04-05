/// 销售订单数据模型
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SalesOrder {
    pub id: i32,
    pub order_no: String,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub total_amount: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 销售订单列表响应
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SalesOrderListResponse {
    #[serde(rename = "data")]
    pub orders: Vec<SalesOrder>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 创建销售订单请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateSalesOrderRequest {
    pub order_no: String,
    pub customer_name: String,
    pub total_amount: String,
}

/// 更新销售订单请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateSalesOrderRequest {
    pub order_no: Option<String>,
    pub customer_name: Option<String>,
    pub total_amount: Option<String>,
    pub status: Option<String>,
}
