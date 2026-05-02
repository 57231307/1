//! 应付发票服务
//!
//! 与后端应付发票API交互

use crate::models::ap_invoice::{
    AgingAnalysisItem, ApInvoice, ApInvoiceListResponse,
    ApInvoiceQueryParams, AutoGenerateRequest, BalanceSummaryItem, CancelInvoiceRequest,
    CreateApInvoiceRequest, UpdateApInvoiceRequest,
};
use crate::services::api::ApiService;
use crate::services::crud_service::CrudService;

/// 应付发票服务
pub struct ApInvoiceService;

impl CrudService for ApInvoiceService {
    type Model = ApInvoice;
    type ListResponse = ApInvoiceListResponse;
    type CreateRequest = CreateApInvoiceRequest;
    type UpdateRequest = UpdateApInvoiceRequest;

    fn base_path() -> &'static str {
        "/ap/invoices"
    }
}


impl ApInvoiceService {
    /// 查询应付发票列表
    pub async fn list_invoices(params: ApInvoiceQueryParams) -> Result<ApInvoiceListResponse, String> {
        let mut query_parts = vec![];

        if let Some(sid) = params.supplier_id {
            query_parts.push(format!("supplier_id={}", sid));
        }
        if let Some(ref status) = params.invoice_status {
            query_parts.push(format!("invoice_status={}", status));
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

        let url = format!("/ap/invoices{}", query_string);
        ApiService::get::<ApInvoiceListResponse>(&url).await
    }

    /// 获取应付发票详情
    #[allow(dead_code)]
    pub async fn get_invoice(id: i32) -> Result<ApInvoice, String> {
        ApiService::get::<ApInvoice>(&format!("/ap/invoices/{}", id)).await
    }

    /// 创建应付发票
    #[allow(dead_code)]
    pub async fn create_invoice(req: CreateApInvoiceRequest) -> Result<ApInvoice, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/ap/invoices", &payload).await
    }

    /// 更新应付发票
    #[allow(dead_code)]
    pub async fn update_invoice(id: i32, req: UpdateApInvoiceRequest) -> Result<ApInvoice, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/ap/invoices/{}", id), &payload).await
    }

    /// 删除应付发票
    pub async fn delete_invoice(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/ap/invoices/{}", id)).await
    }

    /// 审核应付发票
    pub async fn approve_invoice(id: i32) -> Result<serde_json::Value, String> {
        ApiService::post(&format!("/ap/invoices/{}/approve", id), &serde_json::json!({})).await
    }

    /// 取消应付发票
    pub async fn cancel_invoice(id: i32, reason: String) -> Result<serde_json::Value, String> {
        let req = CancelInvoiceRequest { reason };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post(&format!("/ap/invoices/{}/cancel", id), &payload).await
    }

    /// 自动生成应付发票
    #[allow(dead_code)]
    pub async fn auto_generate(req: AutoGenerateRequest) -> Result<ApInvoice, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/ap/invoices/auto-generate", &payload).await
    }

    /// 获取账龄分析
    #[allow(dead_code)]
    pub async fn get_aging_analysis(supplier_id: Option<i32>) -> Result<Vec<AgingAnalysisItem>, String> {
        let query_string = if let Some(sid) = supplier_id {
            format!("?supplier_id={}", sid)
        } else {
            String::new()
        };
        let url = format!("/ap/invoices/aging{}", query_string);
        ApiService::get::<Vec<AgingAnalysisItem>>(&url).await
    }

    /// 获取应付余额汇总
    #[allow(dead_code)]
    pub async fn get_balance_summary(supplier_id: Option<i32>) -> Result<Vec<BalanceSummaryItem>, String> {
        let query_string = if let Some(sid) = supplier_id {
            format!("?supplier_id={}", sid)
        } else {
            String::new()
        };
        let url = format!("/ap/invoices/balance-summary{}", query_string);
        ApiService::get::<Vec<BalanceSummaryItem>>(&url).await
    }

    /// 获取应付统计报表
    #[allow(dead_code)]
    pub async fn get_statistics() -> Result<serde_json::Value, String> {
        ApiService::get::<serde_json::Value>("/ap/invoices/statistics").await
    }
}
