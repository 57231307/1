//! 付款服务
//!
//! 与后端付款API交互

use crate::models::ap_payment::{
    ApPayment, ApPaymentListResponse, ApPaymentQueryParams, CreateApPaymentRequest,
    PaymentScheduleItem, UpdateApPaymentRequest,
};
use crate::services::api::ApiService;
use crate::services::crud_service::CrudService;

/// 付款服务
pub struct ApPaymentService;

impl CrudService for ApPaymentService {
    type Model = ApPayment;
    type ListResponse = ApPaymentListResponse;
    type CreateRequest = CreateApPaymentRequest;
    type UpdateRequest = UpdateApPaymentRequest;

    fn base_path() -> &'static str {
        "/ap-payments"
    }
}


impl ApPaymentService {
    /// 查询付款列表
    pub async fn list_payments(params: ApPaymentQueryParams) -> Result<ApPaymentListResponse, String> {
        let mut query_parts = vec![];

        if let Some(sid) = params.supplier_id {
            query_parts.push(format!("supplier_id={}", sid));
        }
        if let Some(ref status) = params.payment_status {
            query_parts.push(format!("payment_status={}", status));
        }
        if let Some(ref method) = params.payment_method {
            query_parts.push(format!("payment_method={}", method));
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

        let url = format!("/ap-payments{}", query_string);
        ApiService::get::<ApPaymentListResponse>(&url).await
    }

    /// 获取付款详情
    #[allow(dead_code)]
    pub async fn get_payment(id: i32) -> Result<ApPayment, String> {
        ApiService::get::<ApPayment>(&format!("/ap-payments/{}", id)).await
    }

    /// 创建付款
    #[allow(dead_code)]
    pub async fn create_payment(req: CreateApPaymentRequest) -> Result<ApPayment, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/ap-payments", &payload).await
    }

    /// 更新付款
    #[allow(dead_code)]
    pub async fn update_payment(id: i32, req: UpdateApPaymentRequest) -> Result<ApPayment, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/ap-payments/{}", id), &payload).await
    }

    /// 删除付款
    pub async fn delete_payment(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/ap-payments/{}", id)).await
    }

    /// 确认付款
    pub async fn confirm_payment(id: i32) -> Result<ApPayment, String> {
        ApiService::post::<ApPayment, serde_json::Value>(&format!("/ap-payments/{}/confirm", id), &serde_json::json!({})).await
    }

    /// 获取付款计划
    #[allow(dead_code)]
    pub async fn get_payment_schedule(
        supplier_id: Option<i32>,
        start_date: String,
        end_date: String,
    ) -> Result<Vec<PaymentScheduleItem>, String> {
        let mut query_parts = vec![];
        query_parts.push(format!("start_date={}", start_date));
        query_parts.push(format!("end_date={}", end_date));

        if let Some(sid) = supplier_id {
            query_parts.push(format!("supplier_id={}", sid));
        }

        let query_string = format!("?{}", query_parts.join("&"));
        let url = format!("/ap-payments/schedule{}", query_string);
        ApiService::get::<Vec<PaymentScheduleItem>>(&url).await
    }
}
