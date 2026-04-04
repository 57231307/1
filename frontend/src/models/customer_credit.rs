//! 客户信用模型

use serde::{Deserialize, Serialize};

/// 客户信用数据模型
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CustomerCredit {
    pub id: i32,
    pub customer_id: i32,
    pub credit_level: Option<String>,
    pub credit_score: Option<i32>,
    pub credit_limit: Option<String>,
    pub used_credit: Option<String>,
    pub available_credit: Option<String>,
    pub credit_days: Option<i32>,
    pub status: Option<String>,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 客户信用列表响应
#[derive(Debug, Clone, Deserialize)]
pub struct CustomerCreditListResponse {
    pub items: Vec<CustomerCredit>,
    pub total: Option<u64>,
    pub page: u64,
    pub page_size: u64,
}

/// 客户信用查询参数
#[derive(Debug, Clone, Serialize)]
pub struct CreditQueryParams {
    pub customer_id: Option<i32>,
    pub credit_level: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 信用评级设置请求
#[derive(Debug, Clone, Serialize)]
pub struct CreditRatingRequest {
    pub customer_id: i32,
    pub credit_level: String,
    pub credit_score: i32,
    pub credit_limit: String,
    pub credit_days: i32,
    pub remark: Option<String>,
}

/// 信用额度调整请求
#[derive(Debug, Clone, Serialize)]
pub struct CreditLimitAdjustmentRequest {
    pub adjustment_type: String,
    pub amount: String,
    pub reason: String,
}

/// 信用额度占用/释放请求
#[derive(Debug, Clone, Serialize)]
pub struct CreditAmountRequest {
    pub amount: String,
}
