//! 应付核销服务
//!
//! 与后端应付核销API交互

use crate::models::ap_verification::{
    ApVerification, ApVerificationListResponse, ApVerificationQueryParams, AutoVerifyRequest,
    CancelVerificationRequest, ManualVerifyRequest, UnverifiedInvoiceItem, UnverifiedPaymentItem,
};
use crate::services::api::ApiService;

/// 应付核销服务
pub struct ApVerificationService;

impl ApVerificationService {
    /// 查询核销列表
    pub async fn list_verifications(
        params: ApVerificationQueryParams,
    ) -> Result<ApVerificationListResponse, String> {
        let mut query_parts = vec![];

        if let Some(sid) = params.supplier_id {
            query_parts.push(format!("supplier_id={}", sid));
        }
        if let Some(ref vtype) = params.verification_type {
            query_parts.push(format!("verification_type={}", vtype));
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

        let url = format!("/ap-verifications{}", query_string);
        ApiService::get::<ApVerificationListResponse>(&url).await
    }

    /// 获取核销详情
    #[allow(dead_code)]
    pub async fn get_verification(id: i32) -> Result<ApVerification, String> {
        ApiService::get::<ApVerification>(&format!("/ap-verifications/{}", id)).await
    }

    /// 自动核销
    pub async fn auto_verify(supplier_id: i32) -> Result<ApVerification, String> {
        let req = AutoVerifyRequest { supplier_id };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/ap-verifications/auto-verify", &payload).await
    }

    /// 手工核销
    pub async fn manual_verify(req: ManualVerifyRequest) -> Result<ApVerification, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/ap-verifications/manual-verify", &payload).await
    }

    /// 取消核销
    pub async fn cancel_verification(id: i32, reason: String) -> Result<ApVerification, String> {
        let req = CancelVerificationRequest { reason };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post(&format!("/ap-verifications/{}/cancel", id), &payload).await
    }

    /// 获取未核销应付发票列表
    pub async fn get_unverified_invoices(
        supplier_id: i32,
    ) -> Result<Vec<UnverifiedInvoiceItem>, String> {
        let url = format!(
            "/ap-verifications/unverified-invoices?supplier_id={}",
            supplier_id
        );
        ApiService::get::<Vec<UnverifiedInvoiceItem>>(&url).await
    }

    /// 获取未核销付款单列表
    pub async fn get_unverified_payments(
        supplier_id: i32,
    ) -> Result<Vec<UnverifiedPaymentItem>, String> {
        let url = format!(
            "/ap-verifications/unverified-payments?supplier_id={}",
            supplier_id
        );
        ApiService::get::<Vec<UnverifiedPaymentItem>>(&url).await
    }
}
