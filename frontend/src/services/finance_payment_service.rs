//! 财务付款服务
//!
//! 与后端财务付款API交互

use crate::services::api::ApiService;
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
    pub amount: String,
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
    pub amount: String,
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

/// 财务付款服务
pub struct FinancePaymentService;

impl FinancePaymentService {
    /// 查询财务付款列表
    pub async fn list_payments(params: PaymentQueryParams) -> Result<PaymentListResponse, String> {
        let mut query_parts = vec![];

        if let Some(ref status) = params.status {
            query_parts.push(format!("status={}", status));
        }
        if let Some(ref ptype) = params.payment_type {
            query_parts.push(format!("payment_type={}", ptype));
        }
        if let Some(ref sd) = params.start_date {
            query_parts.push(format!("start_date={}", sd));
        }
        if let Some(ref ed) = params.end_date {
            query_parts.push(format!("end_date={}", ed));
        }
        if let Some(p) = params.page {
            query_parts.push(format!("page={}", p));
        }
        if let Some(ps) = params.page_size {
            query_parts.push(format!("page_size={}", ps));
        }

        let query_string = if query_parts.is_empty() {
            String::new()
        } else {
            format!("?{}", query_parts.join("&"))
        };

        let url = format!("/finance-payments{}", query_string);
        ApiService::get::<PaymentListResponse>(&url).await
    }

    /// 获取财务付款详情
    #[allow(dead_code)]
    pub async fn get_payment(id: i32) -> Result<FinancePayment, String> {
        ApiService::get::<FinancePayment>(&format!("/finance-payments/{}", id)).await
    }

    /// 创建财务付款
    #[allow(dead_code)]
    pub async fn create_payment(req: CreatePaymentRequest) -> Result<FinancePayment, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/finance-payments", &payload).await
    }

    /// 更新财务付款
    #[allow(dead_code)]
    pub async fn update_payment(id: i32, req: UpdatePaymentRequest) -> Result<FinancePayment, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/finance-payments/{}", id), &payload).await
    }

    /// 删除财务付款
    pub async fn delete_payment(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/finance-payments/{}", id)).await
    }
}