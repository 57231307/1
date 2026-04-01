//! 付款申请服务
//!
//! 与后端付款申请API交互

use crate::models::ap_payment_request::{
    ApPaymentRequest, ApPaymentRequestListResponse, ApPaymentRequestQueryParams,
    ApPaymentRequestItemRequest, CreateApPaymentRequest, RejectApPaymentRequest,
    UpdateApPaymentRequest,
};
use crate::services::api::ApiService;

/// 付款申请服务
pub struct ApPaymentRequestService;

impl ApPaymentRequestService {
    /// 查询付款申请列表
    pub async fn list_requests(params: ApPaymentRequestQueryParams) -> Result<ApPaymentRequestListResponse, String> {
        let mut query_parts = vec![];

        if let Some(sid) = params.supplier_id {
            query_parts.push(format!("supplier_id={}", sid));
        }
        if let Some(ref status) = params.approval_status {
            query_parts.push(format!("approval_status={}", status));
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

        let url = format!("/ap-payment-requests{}", query_string);
        ApiService::get::<ApPaymentRequestListResponse>(&url).await
    }

    /// 获取付款申请详情
    #[allow(dead_code)]
    pub async fn get_request(id: i32) -> Result<ApPaymentRequest, String> {
        ApiService::get::<ApPaymentRequest>(&format!("/ap-payment-requests/{}", id)).await
    }

    /// 创建付款申请
    #[allow(dead_code)]
    pub async fn create_request(req: CreateApPaymentRequest) -> Result<ApPaymentRequest, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/ap-payment-requests", &payload).await
    }

    /// 更新付款申请
    #[allow(dead_code)]
    pub async fn update_request(id: i32, req: UpdateApPaymentRequest) -> Result<ApPaymentRequest, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/ap-payment-requests/{}", id), &payload).await
    }

    /// 删除付款申请
    pub async fn delete_request(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/ap-payment-requests/{}", id)).await
    }

    /// 提交付款申请
    pub async fn submit_request(id: i32) -> Result<ApPaymentRequest, String> {
        ApiService::post::<ApPaymentRequest>(&format!("/ap-payment-requests/{}/submit", id), &serde_json::json!({})).await
    }

    /// 审批付款申请
    pub async fn approve_request(id: i32) -> Result<ApPaymentRequest, String> {
        ApiService::post::<ApPaymentRequest>(&format!("/ap-payment-requests/{}/approve", id), &serde_json::json!({})).await
    }

    /// 拒绝付款申请
    pub async fn reject_request(id: i32, reason: String) -> Result<ApPaymentRequest, String> {
        let req = RejectApPaymentRequest { reason };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post(&format!("/ap-payment-requests/{}/reject", id), &payload).await
    }
}
