//! 销售分析模型
//!
//! 销售分析相关的数据结构

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesTrendAnalysis {
    pub period: String,
    pub start_date: String,
    pub end_date: String,
    pub total_sales_amount: String,
    pub total_sales_quantity: i64,
    pub average_daily_sales: String,
    pub growth_rate: String,
    pub trend_direction: String,
    pub peak_date: Option<String>,
    pub lowest_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductRanking {
    pub rank: i32,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub product_code: Option<String>,
    pub category_id: Option<i32>,
    pub total_sales_amount: String,
    pub total_sales_quantity: i64,
    pub gross_profit: String,
    pub gross_margin: String,
    pub customer_count: i64,
    pub order_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerRanking {
    pub rank: i32,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub customer_type: String,
    pub total_sales_amount: String,
    pub total_sales_quantity: i64,
    pub gross_profit: String,
    pub order_count: i64,
    pub average_order_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesTarget {
    pub id: i32,
    pub target_type: String,
    pub target_id: i32,
    pub period: String,
    pub target_amount: String,
    pub actual_amount: String,
    pub completion_rate: String,
    pub start_date: String,
    pub end_date: String,
    pub status: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSalesTargetRequest {
    pub target_type: String,
    pub target_id: i32,
    pub period: String,
    pub target_amount: String,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSalesTargetRequest {
    pub target_amount: Option<String>,
    pub status: Option<String>,
}
