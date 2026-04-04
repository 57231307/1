//! 资金管理模型
//!
//! 资金管理相关的数据结构

use serde::{Deserialize, Serialize};

/// 资金账户数据模型
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FundAccount {
    pub id: i32,
    pub account_name: String,
    pub account_no: String,
    pub account_type: String,
    pub bank_name: Option<String>,
    pub currency: String,
    pub balance: String,
    pub frozen_amount: String,
    pub available_amount: String,
    pub status: String,
    pub opened_date: Option<String>,
    pub remark: Option<String>,
    pub created_by: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 资金账户列表响应
#[derive(Debug, Clone, Deserialize)]
pub struct FundAccountListResponse {
    pub data: Vec<FundAccount>,
}

/// 创建资金账户请求
#[derive(Debug, Clone, Serialize)]
pub struct CreateFundAccountRequest {
    pub account_name: String,
    pub account_no: String,
    pub account_type: String,
    pub bank_name: Option<String>,
    pub currency: String,
    pub opened_date: Option<String>,
    pub remark: Option<String>,
}

/// 资金账户查询参数
#[derive(Debug, Clone, Serialize)]
pub struct FundAccountQueryParams {
    pub account_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}
