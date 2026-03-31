//! 销售价格模型

use serde::{Deserialize, Serialize};

/// 销售价格模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesPrice {
    pub id: i32,
    pub product_id: i32,
    pub customer_id: Option<i32>,
    pub customer_type: Option<String>,
    pub price: String,
    pub currency: String,
    pub unit: String,
    pub min_order_qty: Option<String>,
    pub price_type: String,
    pub price_level: Option<String>,
    pub effective_date: String,
    pub expiry_date: Option<String>,
    pub status: String,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建销售价格请求
#[derive(Debug, Clone, Serialize)]
pub struct CreateSalesPriceRequest {
    pub product_id: i32,
    pub customer_id: Option<i32>,
    pub customer_type: Option<String>,
    pub price: String,
    pub currency: Option<String>,
    pub unit: String,
    pub min_order_qty: Option<String>,
    pub price_type: Option<String>,
    pub price_level: Option<String>,
    pub effective_date: String,
    pub expiry_date: Option<String>,
}

/// 更新销售价格请求
#[derive(Debug, Clone, Serialize)]
pub struct UpdateSalesPriceRequest {
    pub price: Option<String>,
    pub price_level: Option<String>,
    pub expiry_date: Option<String>,
    pub status: Option<String>,
}

/// 审批销售价格请求
#[derive(Debug, Clone, Serialize)]
pub struct ApprovePriceRequest {
    pub approved: bool,
    pub remark: Option<String>,
}
