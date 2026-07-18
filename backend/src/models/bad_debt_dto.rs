//! 坏账管理 DTO（V15 P0-B01/B02 Batch 481 创建）
//!
//! 包含坏账准备计提（B01）和坏账核销审批（B02）的请求/响应 DTO

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ==================== B01 坏账准备计提 ====================

/// 手动触发计提请求
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RunProvisionRequest {
    pub period_year: i32,
    pub period_month: i32,
}

/// 计提查询
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ListProvisionQuery {
    pub customer_id: Option<i64>,
    pub period_year: Option<i32>,
    pub period_month: Option<i32>,
    pub aging_bucket: Option<String>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 转回请求
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReverseProvisionRequest {
    pub reverse_voucher_id: Option<i64>,
    pub remark: Option<String>,
}

// ==================== B02 坏账核销审批 ====================

/// 申请核销
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CreateWriteoffRequest {
    pub customer_id: i64,
    pub ar_invoice_id: i32,
    pub writeoff_amount: Decimal,
    pub reason: String,
    pub remark: Option<String>,
}

/// 审批操作（一级财务经理 / 二级总经理通用）
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApproveWriteoffRequest {
    pub comment: Option<String>,
}

/// 拒绝核销
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RejectWriteoffRequest {
    pub comment: String,
}

/// 取消核销申请
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CancelWriteoffRequest {
    pub cancel_reason: String,
}

/// 核销查询
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ListWriteoffQuery {
    pub customer_id: Option<i64>,
    pub ar_invoice_id: Option<i32>,
    pub approval_status: Option<String>,
    pub applicant_user_id: Option<i32>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}
