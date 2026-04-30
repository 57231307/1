use rust_decimal::Decimal;
// 财务付款模型
//
// 财务付款相关的数据结构

use serde::{Deserialize, Serialize};

/// 财务付款数据模型
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FinancePayment {
    pub id: i32,
    pub payment_no: String,
    pub payment_type: String,
    pub order_type: Option<String>,
    pub order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub supplier_id: Option<i32>,
    pub amount: Decimal,
    pub status: String,
    pub payment_date: String,
    pub payment_method: Option<String>,
    pub reference_no: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
}

/// 财务付款列表响应
#[derive(Debug, Clone, Deserialize)]
pub struct PaymentListResponse {
    pub payments: Vec<FinancePayment>,
    #[allow(dead_code)]
    pub total: u64,
    #[allow(dead_code)]
    pub page: u64,
    #[allow(dead_code)]
    pub page_size: u64,
}

/// 财务付款查询参数
#[derive(Debug, Clone, Serialize)]
pub struct PaymentQueryParams {
    pub status: Option<String>,
    pub payment_type: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 创建财务付款请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct CreatePaymentRequest {
    pub payment_no: String,
    pub payment_type: String,
    pub order_type: String,
    pub order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub supplier_id: Option<i32>,
    pub amount: Decimal,
    pub payment_date: String,
    pub payment_method: Option<String>,
    pub reference_no: Option<String>,
    pub notes: Option<String>,
}

/// 更新财务付款请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct UpdatePaymentRequest {
    pub payment_no: Option<String>,
    pub payment_type: Option<String>,
    pub order_type: Option<String>,
    pub order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub supplier_id: Option<i32>,
    pub amount: Option<String>,
    pub status: Option<String>,
    pub payment_date: Option<String>,
    pub payment_method: Option<String>,
    pub reference_no: Option<String>,
    pub notes: Option<String>,
}
