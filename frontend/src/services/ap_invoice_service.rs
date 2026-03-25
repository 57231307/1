//! 应付发票服务
//!
//! 与后端应付发票API交互

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

/// 应付发票数据模型
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApInvoice {
    pub id: i32,
    pub invoice_no: String,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub invoice_date: String,
    pub due_date: Option<String>,
    pub invoice_status: String,
    pub invoice_type: String,
    pub currency_code: Option<String>,
    pub exchange_rate: Option<String>,
    pub total_amount: String,
    pub tax_amount: Option<String>,
    pub paid_amount: Option<String>,
    pub outstanding_amount: Option<String>,
    pub source_type: Option<String>,
    pub source_module: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub receipt_id: Option<i32>,
    pub receipt_no: Option<String>,
    pub remarks: Option<String>,
    pub approver_id: Option<i32>,
    pub approver_name: Option<String>,
    pub approved_at: Option<String>,
    pub cancel_reason: Option<String>,
    pub cancelled_at: Option<String>,
    pub cancelled_by: Option<i32>,
    pub creator_id: i32,
    pub creator_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 应付发票明细项
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApInvoiceItem {
    pub id: i32,
    pub invoice_id: i32,
    pub line_no: i32,
    pub product_id: Option<i32>,
    pub product_name: Option<String>,
    pub product_code: Option<String>,
    pub specification: Option<String>,
    pub unit: Option<String>,
    pub quantity: Option<String>,
    pub unit_price: Option<String>,
    pub amount: String,
    pub tax_rate: Option<String>,
    pub tax_amount: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub warehouse_name: Option<String>,
}

/// 应付发票列表响应
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ApInvoiceListResponse {
    pub data: Vec<ApInvoice>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 应付发票查询参数
#[derive(Debug, Clone, Serialize)]
pub struct ApInvoiceQueryParams {
    pub supplier_id: Option<i32>,
    pub invoice_status: Option<String>,
    pub invoice_type: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 创建应付发票请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct CreateApInvoiceRequest {
    pub supplier_id: i32,
    pub invoice_date: String,
    pub due_date: Option<String>,
    pub invoice_type: String,
    pub currency_code: Option<String>,
    pub exchange_rate: Option<String>,
    pub total_amount: String,
    pub tax_amount: Option<String>,
    pub source_type: Option<String>,
    pub source_module: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub remarks: Option<String>,
    pub items: Vec<ApInvoiceItemRequest>,
}

/// 更新应付发票请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct UpdateApInvoiceRequest {
    pub supplier_id: Option<i32>,
    pub invoice_date: Option<String>,
    pub due_date: Option<String>,
    pub invoice_type: Option<String>,
    pub currency_code: Option<String>,
    pub exchange_rate: Option<String>,
    pub total_amount: Option<String>,
    pub tax_amount: Option<String>,
    pub remarks: Option<String>,
    pub items: Option<Vec<ApInvoiceItemRequest>>,
}

/// 应付发票明细项请求
#[derive(Debug, Clone, Serialize)]
pub struct ApInvoiceItemRequest {
    pub line_no: i32,
    pub product_id: Option<i32>,
    pub product_name: Option<String>,
    pub product_code: Option<String>,
    pub specification: Option<String>,
    pub unit: Option<String>,
    pub quantity: Option<String>,
    pub unit_price: Option<String>,
    pub amount: String,
    pub tax_rate: Option<String>,
    pub tax_amount: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub warehouse_id: Option<i32>,
}

/// 取消应付发票请求
#[derive(Debug, Clone, Serialize)]
pub struct CancelInvoiceRequest {
    pub reason: String,
}

/// 自动生成应付发票请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct AutoGenerateRequest {
    pub receipt_id: i32,
}

/// 账龄分析数据
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgingAnalysisItem {
    pub supplier_id: i32,
    pub supplier_name: String,
    pub current_amount: String,
    pub days_1_30: String,
    pub days_31_60: String,
    pub days_61_90: String,
    pub days_over_90: String,
    pub total_outstanding: String,
}

/// 应付余额汇总
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BalanceSummaryItem {
    pub supplier_id: i32,
    pub supplier_name: String,
    pub invoice_count: i32,
    pub total_amount: String,
    pub paid_amount: String,
    pub outstanding_amount: String,
}

/// 应付发票服务
pub struct ApInvoiceService;

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

        let url = format!("/ap-invoices{}", query_string);
        ApiService::get::<ApInvoiceListResponse>(&url).await
    }

    /// 获取应付发票详情
    #[allow(dead_code)]
    pub async fn get_invoice(id: i32) -> Result<ApInvoice, String> {
        ApiService::get::<ApInvoice>(&format!("/ap-invoices/{}", id)).await
    }

    /// 创建应付发票
    #[allow(dead_code)]
    pub async fn create_invoice(req: CreateApInvoiceRequest) -> Result<ApInvoice, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/ap-invoices", &payload).await
    }

    /// 更新应付发票
    #[allow(dead_code)]
    pub async fn update_invoice(id: i32, req: UpdateApInvoiceRequest) -> Result<ApInvoice, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/ap-invoices/{}", id), &payload).await
    }

    /// 删除应付发票
    pub async fn delete_invoice(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/ap-invoices/{}", id)).await
    }

    /// 审核应付发票
    pub async fn approve_invoice(id: i32) -> Result<serde_json::Value, String> {
        ApiService::post(&format!("/ap-invoices/{}/approve", id), &serde_json::json!({})).await
    }

    /// 取消应付发票
    pub async fn cancel_invoice(id: i32, reason: String) -> Result<serde_json::Value, String> {
        let req = CancelInvoiceRequest { reason };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post(&format!("/ap-invoices/{}/cancel", id), &payload).await
    }

    /// 自动生成应付发票
    #[allow(dead_code)]
    pub async fn auto_generate(req: AutoGenerateRequest) -> Result<ApInvoice, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/ap-invoices/auto-generate", &payload).await
    }

    /// 获取账龄分析
    #[allow(dead_code)]
    pub async fn get_aging_analysis(supplier_id: Option<i32>) -> Result<Vec<AgingAnalysisItem>, String> {
        let query_string = if let Some(sid) = supplier_id {
            format!("?supplier_id={}", sid)
        } else {
            String::new()
        };
        let url = format!("/ap-invoices/aging-analysis{}", query_string);
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
        let url = format!("/ap-invoices/balance-summary{}", query_string);
        ApiService::get::<Vec<BalanceSummaryItem>>(&url).await
    }

    /// 获取应付统计报表
    #[allow(dead_code)]
    pub async fn get_statistics() -> Result<serde_json::Value, String> {
        ApiService::get::<serde_json::Value>("/ap-invoices/statistics").await
    }
}