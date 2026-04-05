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
    pub customer_id: i32,
    pub required_date: String,
    pub status: String,
    pub shipping_address: Option<String>,
    pub billing_address: Option<String>,
    pub notes: Option<String>,
    pub items: Vec<SalesOrderItemRequest>,
    // 面料行业特有字段
    pub payment_terms: Option<String>,
    pub remarks: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub packaging_requirement: Option<String>,
    pub quality_standard: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SalesOrderItemRequest {
    pub product_id: i32,
    pub quantity: String,
    pub unit_price: String,
    pub discount_percent: Option<String>,
    pub tax_percent: Option<String>,
    pub notes: Option<String>,
    pub color_no: Option<String>,
    pub color_name: Option<String>,
    pub pantone_code: Option<String>,
    pub grade_required: Option<String>,
    pub quantity_meters: Option<String>,
    pub quantity_kg: Option<String>,
    pub gram_weight: Option<String>,
    pub width: Option<String>,
    pub batch_requirement: Option<String>,
    pub dye_lot_requirement: Option<String>,
    pub base_price: Option<String>,
    pub color_extra_cost: Option<String>,
    pub grade_price_diff: Option<String>,
    pub final_price: Option<String>,
}

/// 更新销售订单请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateSalesOrderRequest {
    pub required_date: Option<String>,
    pub status: Option<String>,
    pub shipping_address: Option<String>,
    pub billing_address: Option<String>,
    pub notes: Option<String>,
    pub items: Option<Vec<SalesOrderItemRequest>>,
}
