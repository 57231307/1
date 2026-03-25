//! 应付对账服务
//!
//! 与后端应付对账 API 交互

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

/// 应付对账数据模型
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApReconciliation {
    pub id: i32,
    pub reconciliation_no: String,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub reconciliation_date: String,
    pub period_start: String,
    pub period_end: String,
    pub reconciliation_status: String,
    pub total_amount: String,
    pub confirmed_amount: Option<String>,
    pub disputed_amount: Option<String>,
    pub paid_amount: Option<String>,
    pub outstanding_amount: Option<String>,
    pub currency_code: Option<String>,
    pub exchange_rate: Option<String>,
    pub invoice_count: i32,
    pub confirmed_invoice_count: i32,
    pub disputed_invoice_count: i32,
    pub source_type: Option<String>,
    pub source_module: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub remarks: Option<String>,
    pub confirmed_at: Option<String>,
    pub confirmed_by: Option<i32>,
    pub confirmed_by_name: Option<String>,
    pub disputed_at: Option<String>,
    pub disputed_by: Option<i32>,
    pub disputed_by_name: Option<String>,
    pub dispute_reason: Option<String>,
    pub approver_id: Option<i32>,
    pub approver_name: Option<String>,
    pub approved_at: Option<String>,
    pub creator_id: i32,
    pub creator_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 应付对账明细项
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApReconciliationItem {
    pub id: i32,
    pub reconciliation_id: i32,
    pub invoice_id: Option<i32>,
    pub invoice_no: Option<String>,
    pub invoice_date: Option<String>,
    pub invoice_amount: String,
    pub confirmed_amount: Option<String>,
    pub disputed_amount: Option<String>,
    pub item_status: String,
    pub dispute_reason: Option<String>,
    pub product_id: Option<i32>,
    pub product_name: Option<String>,
    pub product_code: Option<String>,
    pub specification: Option<String>,
    pub unit: Option<String>,
    pub quantity: Option<String>,
    pub unit_price: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub warehouse_name: Option<String>,
    pub remarks: Option<String>,
    pub creator_id: i32,
    pub creator_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 应付对账列表响应
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ApReconciliationListResponse {
    pub items: Vec<ApReconciliation>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 应付对账查询参数
#[derive(Debug, Clone, Serialize)]
pub struct ApReconciliationQueryParams {
    pub supplier_id: Option<i32>,
    pub reconciliation_status: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 生成对账单请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct GenerateReconciliationRequest {
    pub supplier_id: i32,
    pub reconciliation_date: String,
    pub period_start: String,
    pub period_end: String,
    pub currency_code: Option<String>,
    pub exchange_rate: Option<String>,
    pub remarks: Option<String>,
}

/// 更新对账单请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct UpdateReconciliationRequest {
    pub reconciliation_date: Option<String>,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub remarks: Option<String>,
}

/// 确认对账单请求
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct ConfirmReconciliationRequest {
    pub confirmed_amount: String,
    pub remarks: Option<String>,
}

/// 争议请求
#[derive(Debug, Clone, Serialize)]
pub struct DisputeRequest {
    pub reason: String,
}

/// 供应商应付汇总数据
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SupplierSummary {
    pub supplier_id: i32,
    pub supplier_name: String,
    pub total_reconciliations: i32,
    pub total_amount: String,
    pub confirmed_amount: String,
    pub disputed_amount: String,
    pub paid_amount: String,
    pub outstanding_amount: String,
}

/// 应付对账服务
pub struct ApReconciliationService;

impl ApReconciliationService {
    /// 查询对账单列表
    pub async fn list_reconciliations(params: ApReconciliationQueryParams) -> Result<ApReconciliationListResponse, String> {
        let mut query_parts = vec![];

        if let Some(sid) = params.supplier_id {
            query_parts.push(format!("supplier_id={}", sid));
        }
        if let Some(ref status) = params.reconciliation_status {
            query_parts.push(format!("reconciliation_status={}", status));
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

        let url = format!("/ap-reconciliations{}", query_string);
        ApiService::get::<ApReconciliationListResponse>(&url).await
    }

    /// 获取对账单详情
    #[allow(dead_code)]
    pub async fn get_reconciliation(id: i32) -> Result<ApReconciliation, String> {
        ApiService::get::<ApReconciliation>(&format!("/ap-reconciliations/{}", id)).await
    }

    /// 生成对账单
    #[allow(dead_code)]
    pub async fn generate_reconciliation(req: GenerateReconciliationRequest) -> Result<ApReconciliation, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/ap-reconciliations", &payload).await
    }

    /// 更新对账单
    #[allow(dead_code)]
    pub async fn update_reconciliation(id: i32, req: UpdateReconciliationRequest) -> Result<ApReconciliation, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/ap-reconciliations/{}", id), &payload).await
    }

    /// 删除对账单
    #[allow(dead_code)]
    pub async fn delete_reconciliation(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/ap-reconciliations/{}", id)).await
    }

    /// 确认对账单
    pub async fn confirm_reconciliation(id: i32) -> Result<ApReconciliation, String> {
        ApiService::post::<ApReconciliation>(&format!("/ap-reconciliations/{}/confirm", id), &serde_json::json!({})).await
    }

    /// 提出争议
    pub async fn dispute_reconciliation(id: i32, reason: String) -> Result<ApReconciliation, String> {
        let req = DisputeRequest { reason };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post(&format!("/ap-reconciliations/{}/dispute", id), &payload).await
    }

    /// 获取供应商应付汇总
    #[allow(dead_code)]
    pub async fn get_supplier_summary(supplier_id: Option<i32>) -> Result<Vec<SupplierSummary>, String> {
        let query_string = if let Some(sid) = supplier_id {
            format!("?supplier_id={}", sid)
        } else {
            String::new()
        };
        let url = format!("/ap-reconciliations/supplier-summary{}", query_string);
        ApiService::get::<Vec<SupplierSummary>>(&url).await
    }
}