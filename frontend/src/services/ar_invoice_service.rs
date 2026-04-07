//! 应收发票服务
//!
//! 与后端应收发票API交互

use crate::models::ar_invoice::{
    ArInvoice, ArInvoiceListResponse, ArInvoiceQueryParams, CreateArInvoiceRequest,
    UpdateArInvoiceRequest,
};
use crate::services::api::ApiService;

/// 应收发票服务
pub struct ArInvoiceService;

impl ArInvoiceService {
    /// 查询应收发票列表
    pub async fn list_invoices(
        params: ArInvoiceQueryParams,
    ) -> Result<ArInvoiceListResponse, String> {
        let mut query_parts = vec![];

        if let Some(cid) = params.customer_id {
            query_parts.push(format!("customer_id={}", cid));
        }
        if let Some(ref status) = params.status {
            query_parts.push(format!("status={}", status));
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

        let url = format!("/ar-invoices{}", query_string);
        ApiService::get::<ArInvoiceListResponse>(&url).await
    }

    /// 获取应收发票详情
    #[allow(dead_code)]
    pub async fn get_invoice(id: i32) -> Result<ArInvoice, String> {
        ApiService::get::<ArInvoice>(&format!("/ar-invoices/{}", id)).await
    }

    /// 创建应收发票
    #[allow(dead_code)]
    pub async fn create_invoice(req: CreateArInvoiceRequest) -> Result<ArInvoice, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/ar-invoices", &payload).await
    }

    /// 更新应收发票
    #[allow(dead_code)]
    pub async fn update_invoice(id: i32, req: UpdateArInvoiceRequest) -> Result<ArInvoice, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/ar-invoices/{}", id), &payload).await
    }

    /// 删除应收发票
    pub async fn delete_invoice(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/ar-invoices/{}", id)).await
    }

    /// 审核应收发票
    pub async fn approve_invoice(id: i32) -> Result<serde_json::Value, String> {
        ApiService::post(
            &format!("/ar-invoices/{}/approve", id),
            &serde_json::json!({}),
        )
        .await
    }

    /// 取消应收发票
    pub async fn cancel_invoice(id: i32, reason: String) -> Result<serde_json::Value, String> {
        let payload = serde_json::json!({ "reason": reason });
        ApiService::post(&format!("/ar-invoices/{}/cancel", id), &payload).await
    }

    /// 收款确认
    #[allow(dead_code)]
    pub async fn confirm_receive(
        id: i32,
        receive_amount: String,
    ) -> Result<serde_json::Value, String> {
        let payload = serde_json::json!({ "receive_amount": receive_amount });
        ApiService::post(&format!("/ar-invoices/{}/receive", id), &payload).await
    }

    /// 获取客户应收统计
    #[allow(dead_code)]
    pub async fn get_customer_summary(customer_id: i32) -> Result<serde_json::Value, String> {
        ApiService::get::<serde_json::Value>(&format!(
            "/ar-invoices/customer-summary/{}",
            customer_id
        ))
        .await
    }

    /// 获取账龄分析
    #[allow(dead_code)]
    pub async fn get_aging_analysis(customer_id: Option<i32>) -> Result<serde_json::Value, String> {
        let query_string = if let Some(cid) = customer_id {
            format!("?customer_id={}", cid)
        } else {
            String::new()
        };
        let url = format!("/ar-invoices/aging-analysis{}", query_string);
        ApiService::get::<serde_json::Value>(&url).await
    }
}
