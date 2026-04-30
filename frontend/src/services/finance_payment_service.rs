//! 财务付款服务
//!
//! 与后端财务付款API交互

use crate::models::finance_payment::{
    CreatePaymentRequest, FinancePayment, PaymentListResponse, PaymentQueryParams,
    UpdatePaymentRequest,
};
use crate::services::api::ApiService;

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
    pub async fn update_payment(
        id: i32,
        req: UpdatePaymentRequest,
    ) -> Result<FinancePayment, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/finance-payments/{}", id), &payload).await
    }

    /// 删除财务付款
    pub async fn delete_payment(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/finance-payments/{}", id)).await
    }
}
