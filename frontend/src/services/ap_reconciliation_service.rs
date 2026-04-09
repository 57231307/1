//! 应付对账服务
//!
//! 与后端应付对账 API 交互

use crate::models::ap_reconciliation::{
    ApReconciliation, ApReconciliationListResponse, ApReconciliationQueryParams, DisputeRequest,
    GenerateReconciliationRequest, SupplierSummary, UpdateReconciliationRequest,
};
use crate::services::api::ApiService;

/// 应付对账服务
pub struct ApReconciliationService;

impl ApReconciliationService {
    /// 查询对账单列表
    pub async fn list_reconciliations(
        params: ApReconciliationQueryParams,
    ) -> Result<ApReconciliationListResponse, String> {
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

        let url = format!("/ap/reconciliations{}", query_string);
        ApiService::get::<ApReconciliationListResponse>(&url).await
    }

    /// 获取对账单详情
    #[allow(dead_code)]
    pub async fn get_reconciliation(id: i32) -> Result<ApReconciliation, String> {
        ApiService::get::<ApReconciliation>(&format!("/ap/reconciliations/{}", id)).await
    }

    /// 生成对账单
    #[allow(dead_code)]
    pub async fn generate_reconciliation(
        req: GenerateReconciliationRequest,
    ) -> Result<ApReconciliation, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/ap/reconciliations", &payload).await
    }

    /// 更新对账单
    #[allow(dead_code)]
    pub async fn update_reconciliation(
        id: i32,
        req: UpdateReconciliationRequest,
    ) -> Result<ApReconciliation, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/ap/reconciliations/{}", id), &payload).await
    }

    /// 删除对账单
    #[allow(dead_code)]
    pub async fn delete_reconciliation(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/ap/reconciliations/{}", id)).await
    }

    /// 确认对账单
    pub async fn confirm_reconciliation(id: i32) -> Result<ApReconciliation, String> {
        ApiService::post::<ApReconciliation, serde_json::Value>(
            &format!("/ap/reconciliations/{}/confirm", id),
            &serde_json::json!({}),
        )
        .await
    }

    /// 提出争议
    pub async fn dispute_reconciliation(
        id: i32,
        reason: String,
    ) -> Result<ApReconciliation, String> {
        let req = DisputeRequest { reason };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post(&format!("/ap/reconciliations/{}/dispute", id), &payload).await
    }

    /// 获取供应商应付汇总
    #[allow(dead_code)]
    pub async fn get_supplier_summary(
        supplier_id: Option<i32>,
    ) -> Result<Vec<SupplierSummary>, String> {
        let query_string = if let Some(sid) = supplier_id {
            format!("?supplier_id={}", sid)
        } else {
            String::new()
        };
        let url = format!("/ap/reconciliations/summary{}", query_string);
        ApiService::get::<Vec<SupplierSummary>>(&url).await
    }
}
