//! 采购价格模型

use serde::{Deserialize, Serialize};

/// 采购价格模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchasePrice {
    pub id: i32,
    pub product_id: i32,
    pub supplier_id: i32,
    pub price: String,
    pub currency: String,
    pub unit: String,
    pub min_order_qty: Option<String>,
    pub price_type: String,
    pub effective_date: String,
    pub expiry_date: Option<String>,
    pub status: String,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建采购价格请求
#[derive(Debug, Clone, Serialize)]
pub struct CreatePurchasePriceRequest {
    pub product_id: i32,
    pub supplier_id: i32,
    pub price: String,
    pub currency: String,
    pub unit: String,
    pub min_order_qty: Option<String>,
    pub price_type: Option<String>,
    pub effective_date: String,
    pub expiry_date: Option<String>,
}

/// 更新采购价格请求
#[derive(Debug, Clone, Serialize)]
pub struct UpdatePurchasePriceRequest {
    pub price: Option<String>,
    pub expiry_date: Option<String>,
    pub status: Option<String>,
}

/// 审批采购价格请求
#[derive(Debug, Clone, Serialize)]
pub struct ApprovePriceRequest {
    pub approved: bool,
    pub remark: Option<String>,
}

/// 价格趋势分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceTrendAnalysis {
    pub product_id: i32,
    pub supplier_id: i32,
    pub current_price: String,
    pub average_price: String,
    pub min_price: String,
    pub max_price: String,
    pub price_change_rate: String,
    pub trend_direction: String,
    pub history_count: i64,
}
