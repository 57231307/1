//! 财务发票服务
//!
//! 与后端财务发票API交互

use crate::models::finance_invoice::{
    CreateInvoiceRequest, FinanceInvoice, InvoiceListResponse, InvoiceQueryParams,
    UpdateInvoiceRequest, VerifyInvoiceRequest,
};
use crate::services::api::ApiService;

/// 财务发票服务
pub struct FinanceInvoiceService;

impl FinanceInvoiceService {
    /// 查询财务发票列表
    pub async fn list_invoices(params: InvoiceQueryParams) -> Result<InvoiceListResponse, String> {
        let mut query_parts = vec![];

        if let Some(cid) = params.customer_id {
            query_parts.push(format!("customer_id={}", cid));
        }
        if let Some(ref status) = params.status {
            query_parts.push(format!("status={}", status));
        }
        if let Some(ref itype) = params.invoice_type {
            query_parts.push(format!("invoice_type={}", itype));
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

        let url = format!("/finance-invoices{}", query_string);
        ApiService::get::<InvoiceListResponse>(&url).await
    }

    /// 获取财务发票详情
    #[allow(dead_code)]
    pub async fn get_invoice(id: i32) -> Result<FinanceInvoice, String> {
        ApiService::get::<FinanceInvoice>(&format!("/finance-invoices/{}", id)).await
    }

    /// 创建财务发票
    #[allow(dead_code)]
    pub async fn create_invoice(req: CreateInvoiceRequest) -> Result<FinanceInvoice, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/finance-invoices", &payload).await
    }

    /// 更新财务发票
    #[allow(dead_code)]
    pub async fn update_invoice(id: i32, req: UpdateInvoiceRequest) -> Result<FinanceInvoice, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/finance-invoices/{}", id), &payload).await
    }

    /// 删除财务发票
    pub async fn delete_invoice(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/finance-invoices/{}", id)).await
    }

    /// 审核财务发票
    pub async fn approve_invoice(id: i32) -> Result<FinanceInvoice, String> {
        ApiService::post::<FinanceInvoice>(&format!("/finance-invoices/{}/approve", id), &serde_json::json!({})).await
    }

    /// 核销财务发票
    pub async fn verify_invoice(id: i32, payment_method: String) -> Result<FinanceInvoice, String> {
        let req = VerifyInvoiceRequest { payment_method };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post(&format!("/finance-invoices/{}/verify", id), &payload).await
    }
}