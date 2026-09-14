//! 通用打印 Handler

use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use serde::Deserialize;
use validator::Validate;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::print_service::PrintService;
use tracing::info;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    Json,
    extract::{Path, State},
    response::Response,
};

async fn render_print_docx(
    state: &AppState,
    doc_type: &str,
    doc_id: i32,
) -> Result<Response, AppError> {
    let service = PrintService::new(state.db.clone());
    let print_data = service.get_print_data(doc_type, doc_id).await?;
    let bytes = service.generate_docx(&print_data)?;
    let filename = format!("{}_{}", doc_type, doc_id);
    Ok(crate::utils::docx_export::docx_response(bytes, &filename))
}

/// V15 P1-1-5：异步记录打印操作审计（best-effort，不阻塞响应）
fn record_print_audit(state: &AppState, auth: &AuthContext, doc_type: &str, doc_id: i32) {
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Print,
        severity: Severity::Info,
        resource_type: Some(doc_type.to_string()),
        resource_id: Some(doc_id.to_string()),
        resource_name: Some(format!("{}_print.docx", doc_type)),
        description: Some(format!(
            "用户 {} 打印 {} #{}",
            auth.username, doc_type, doc_id
        )),
        request_method: Some("GET".to_string()),
        request_path: Some(format!("/api/v1/erp/{}/{}", doc_type, doc_id)),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "doc_type": doc_type,
            "doc_id": doc_id,
            "format": "docx",
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);
}

pub async fn sales_order_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "sales_order", doc_id).await?;
    record_print_audit(&state, &auth, "sales_order", doc_id);
    Ok(resp)
}

pub async fn sales_contract_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "sales_contract", doc_id).await?;
    record_print_audit(&state, &auth, "sales_contract", doc_id);
    Ok(resp)
}

pub async fn purchase_order_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "purchase_order", doc_id).await?;
    record_print_audit(&state, &auth, "purchase_order", doc_id);
    Ok(resp)
}

pub async fn purchase_receipt_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "purchase_receipt", doc_id).await?;
    record_print_audit(&state, &auth, "purchase_receipt", doc_id);
    Ok(resp)
}

pub async fn inventory_transfer_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "inventory_transfer", doc_id).await?;
    record_print_audit(&state, &auth, "inventory_transfer", doc_id);
    Ok(resp)
}

/// 会计凭证打印（docx 成品，规则 3 合规）；service 数据层与模板已就绪，A0 补路由接入
pub async fn voucher_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "voucher", doc_id).await?;
    record_print_audit(&state, &auth, "voucher", doc_id);
    Ok(resp)
}

pub async fn after_sales_print_docx(
    Path(doc_id): Path<i64>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "after_sales", doc_id as i32).await?;
    record_print_audit(&state, &auth, "after_sales", doc_id as i32);
    Ok(resp)
}
pub async fn ap_invoice_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "ap_invoice", doc_id).await?;
    record_print_audit(&state, &auth, "ap_invoice", doc_id);
    Ok(resp)
}
pub async fn ap_payment_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "ap_payment", doc_id).await?;
    record_print_audit(&state, &auth, "ap_payment", doc_id);
    Ok(resp)
}
pub async fn ap_payment_request_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "ap_payment_request", doc_id).await?;
    record_print_audit(&state, &auth, "ap_payment_request", doc_id);
    Ok(resp)
}
pub async fn ap_reconciliation_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "ap_reconciliation", doc_id).await?;
    record_print_audit(&state, &auth, "ap_reconciliation", doc_id);
    Ok(resp)
}
pub async fn ar_collection_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "ar_collection", doc_id).await?;
    record_print_audit(&state, &auth, "ar_collection", doc_id);
    Ok(resp)
}
pub async fn ar_reconciliation_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "ar_reconciliation", doc_id).await?;
    record_print_audit(&state, &auth, "ar_reconciliation", doc_id);
    Ok(resp)
}
pub async fn bad_debt_writeoff_print_docx(
    Path(doc_id): Path<i64>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "bad_debt_writeoff", doc_id as i32).await?;
    record_print_audit(&state, &auth, "bad_debt_writeoff", doc_id as i32);
    Ok(resp)
}
pub async fn bom_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "bom", doc_id).await?;
    record_print_audit(&state, &auth, "bom", doc_id);
    Ok(resp)
}
pub async fn bulk_color_approval_print_docx(
    Path(doc_id): Path<i64>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "bulk_color_approval", doc_id as i32).await?;
    record_print_audit(&state, &auth, "bulk_color_approval", doc_id as i32);
    Ok(resp)
}
pub async fn certificate_of_origin_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "certificate_of_origin", doc_id).await?;
    record_print_audit(&state, &auth, "certificate_of_origin", doc_id);
    Ok(resp)
}
pub async fn chemical_requisition_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "chemical_requisition", doc_id).await?;
    record_print_audit(&state, &auth, "chemical_requisition", doc_id);
    Ok(resp)
}
pub async fn color_card_issue_print_docx(
    Path(doc_id): Path<i64>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "color_card_issue", doc_id as i32).await?;
    record_print_audit(&state, &auth, "color_card_issue", doc_id as i32);
    Ok(resp)
}
pub async fn custom_order_print_docx(
    Path(doc_id): Path<i64>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "custom_order", doc_id as i32).await?;
    record_print_audit(&state, &auth, "custom_order", doc_id as i32);
    Ok(resp)
}
pub async fn customer_credit_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "customer_credit", doc_id).await?;
    record_print_audit(&state, &auth, "customer_credit", doc_id);
    Ok(resp)
}
pub async fn dye_batch_card_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "dye_batch_card", doc_id).await?;
    record_print_audit(&state, &auth, "dye_batch_card", doc_id);
    Ok(resp)
}
pub async fn dye_batch_rework_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "dye_batch_rework", doc_id).await?;
    record_print_audit(&state, &auth, "dye_batch_rework", doc_id);
    Ok(resp)
}
pub async fn energy_consumption_record_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "energy_consumption_record", doc_id).await?;
    record_print_audit(&state, &auth, "energy_consumption_record", doc_id);
    Ok(resp)
}
pub async fn export_customs_declaration_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "export_customs_declaration", doc_id).await?;
    record_print_audit(&state, &auth, "export_customs_declaration", doc_id);
    Ok(resp)
}
pub async fn export_inspection_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "export_inspection", doc_id).await?;
    record_print_audit(&state, &auth, "export_inspection", doc_id);
    Ok(resp)
}
pub async fn export_refund_declaration_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "export_refund_declaration", doc_id).await?;
    record_print_audit(&state, &auth, "export_refund_declaration", doc_id);
    Ok(resp)
}
pub async fn fabric_inspection_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "fabric_inspection", doc_id).await?;
    record_print_audit(&state, &auth, "fabric_inspection", doc_id);
    Ok(resp)
}
pub async fn fixed_asset_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "fixed_asset", doc_id).await?;
    record_print_audit(&state, &auth, "fixed_asset", doc_id);
    Ok(resp)
}
pub async fn fixed_asset_count_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "fixed_asset_count", doc_id).await?;
    record_print_audit(&state, &auth, "fixed_asset_count", doc_id);
    Ok(resp)
}
pub async fn foreign_exchange_verification_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "foreign_exchange_verification", doc_id).await?;
    record_print_audit(&state, &auth, "foreign_exchange_verification", doc_id);
    Ok(resp)
}
pub async fn inventory_adjustment_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "inventory_adjustment", doc_id).await?;
    record_print_audit(&state, &auth, "inventory_adjustment", doc_id);
    Ok(resp)
}
pub async fn inventory_write_down_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "inventory_write_down", doc_id).await?;
    record_print_audit(&state, &auth, "inventory_write_down", doc_id);
    Ok(resp)
}
pub async fn lab_dip_request_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "lab_dip_request", doc_id).await?;
    record_print_audit(&state, &auth, "lab_dip_request", doc_id);
    Ok(resp)
}
pub async fn labor_contract_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "labor_contract", doc_id).await?;
    record_print_audit(&state, &auth, "labor_contract", doc_id);
    Ok(resp)
}
pub async fn logistics_waybill_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "logistics_waybill", doc_id).await?;
    record_print_audit(&state, &auth, "logistics_waybill", doc_id);
    Ok(resp)
}
pub async fn material_shortage_print_docx(
    Path(doc_id): Path<i64>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "material_shortage", doc_id as i32).await?;
    record_print_audit(&state, &auth, "material_shortage", doc_id as i32);
    Ok(resp)
}
pub async fn occupational_hazard_monitoring_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "occupational_hazard_monitoring", doc_id).await?;
    record_print_audit(&state, &auth, "occupational_hazard_monitoring", doc_id);
    Ok(resp)
}
pub async fn occupational_health_exam_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "occupational_health_exam", doc_id).await?;
    record_print_audit(&state, &auth, "occupational_health_exam", doc_id);
    Ok(resp)
}
pub async fn outsourcing_order_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "outsourcing_order", doc_id).await?;
    record_print_audit(&state, &auth, "outsourcing_order", doc_id);
    Ok(resp)
}
pub async fn outsourcing_receipt_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "outsourcing_receipt", doc_id).await?;
    record_print_audit(&state, &auth, "outsourcing_receipt", doc_id);
    Ok(resp)
}
pub async fn pollution_permit_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "pollution_permit", doc_id).await?;
    record_print_audit(&state, &auth, "pollution_permit", doc_id);
    Ok(resp)
}
pub async fn ppe_distribution_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "ppe_distribution", doc_id).await?;
    record_print_audit(&state, &auth, "ppe_distribution", doc_id);
    Ok(resp)
}
pub async fn process_route_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "process_route", doc_id).await?;
    record_print_audit(&state, &auth, "process_route", doc_id);
    Ok(resp)
}
pub async fn production_flow_card_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "production_flow_card", doc_id).await?;
    record_print_audit(&state, &auth, "production_flow_card", doc_id);
    Ok(resp)
}
pub async fn production_order_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "production_order", doc_id).await?;
    record_print_audit(&state, &auth, "production_order", doc_id);
    Ok(resp)
}
pub async fn production_recipe_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "production_recipe", doc_id).await?;
    record_print_audit(&state, &auth, "production_recipe", doc_id);
    Ok(resp)
}
pub async fn purchase_contract_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "purchase_contract", doc_id).await?;
    record_print_audit(&state, &auth, "purchase_contract", doc_id);
    Ok(resp)
}
pub async fn purchase_inspection_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "purchase_inspection", doc_id).await?;
    record_print_audit(&state, &auth, "purchase_inspection", doc_id);
    Ok(resp)
}
pub async fn purchase_return_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "purchase_return", doc_id).await?;
    record_print_audit(&state, &auth, "purchase_return", doc_id);
    Ok(resp)
}
pub async fn quality_8d_report_print_docx(
    Path(doc_id): Path<i64>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "quality_8d_report", doc_id as i32).await?;
    record_print_audit(&state, &auth, "quality_8d_report", doc_id as i32);
    Ok(resp)
}
pub async fn quality_inspection_record_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "quality_inspection_record", doc_id).await?;
    record_print_audit(&state, &auth, "quality_inspection_record", doc_id);
    Ok(resp)
}
pub async fn quality_issue_print_docx(
    Path(doc_id): Path<i64>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "quality_issue", doc_id as i32).await?;
    record_print_audit(&state, &auth, "quality_issue", doc_id as i32);
    Ok(resp)
}
pub async fn safety_accident_report_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "safety_accident_report", doc_id).await?;
    record_print_audit(&state, &auth, "safety_accident_report", doc_id);
    Ok(resp)
}
pub async fn sales_delivery_print_docx(
    // 路由为 /orders/{id}/deliveries/{delivery_id}/print（两段参数），
    // axum Path 单值提取多段路径会 500，必须用 tuple 提取并取第二段
    Path((_order_id, delivery_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "sales_delivery", delivery_id).await?;
    record_print_audit(&state, &auth, "sales_delivery", delivery_id);
    Ok(resp)
}
pub async fn sales_quotation_print_docx(
    Path(doc_id): Path<i64>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "sales_quotation", doc_id as i32).await?;
    record_print_audit(&state, &auth, "sales_quotation", doc_id as i32);
    Ok(resp)
}
pub async fn sales_return_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "sales_return", doc_id).await?;
    record_print_audit(&state, &auth, "sales_return", doc_id);
    Ok(resp)
}
pub async fn scheduling_result_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "scheduling_result", doc_id).await?;
    record_print_audit(&state, &auth, "scheduling_result", doc_id);
    Ok(resp)
}
pub async fn social_insurance_record_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "social_insurance_record", doc_id).await?;
    record_print_audit(&state, &auth, "social_insurance_record", doc_id);
    Ok(resp)
}
pub async fn solid_waste_disposal_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "solid_waste_disposal", doc_id).await?;
    record_print_audit(&state, &auth, "solid_waste_disposal", doc_id);
    Ok(resp)
}
pub async fn supplier_evaluation_record_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "supplier_evaluation_record", doc_id).await?;
    record_print_audit(&state, &auth, "supplier_evaluation_record", doc_id);
    Ok(resp)
}
pub async fn unqualified_product_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "unqualified_product", doc_id).await?;
    record_print_audit(&state, &auth, "unqualified_product", doc_id);
    Ok(resp)
}
pub async fn wage_record_print_docx(
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "wage_record", doc_id).await?;
    record_print_audit(&state, &auth, "wage_record", doc_id);
    Ok(resp)
}

/// 打印模板列表响应
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Clone, serde::Serialize)]
pub struct PrintTemplateDto {
    pub id: i32,
    pub name: String,
    pub doc_type: String,
    pub template_content: String,
    pub is_default: bool,
    pub created_at: String,
}

/// 批次 126 v8 复审 P2 修复：系统内置打印模板静态列表；设计说明：打印模板为系统内置（对应 PrintService 支持的 6 种单据类型）， 不需要动态 CRUD 管理
/// 模板内容字段为简短描述（实际渲染逻辑在 PrintService.generate_docx）。 若未来需支持用户自定义模板，可新增 print_templates 表 + model + service。
pub fn builtin_print_templates() -> Vec<PrintTemplateDto> {
    vec![
        PrintTemplateDto {
            id: 1,
            name: "销售订单打印模板".to_string(),
            doc_type: "sales_order".to_string(),
            template_content: "标准销售订单打印模板（含客户信息、订单明细、金额合计）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 2,
            name: "销售合同打印模板".to_string(),
            doc_type: "sales_contract".to_string(),
            template_content: "标准销售合同打印模板（含合同条款、双方信息、签章位置）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 3,
            name: "采购订单打印模板".to_string(),
            doc_type: "purchase_order".to_string(),
            template_content: "标准采购订单打印模板（含供应商信息、采购明细、金额合计）"
                .to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 4,
            name: "采购收货单打印模板".to_string(),
            doc_type: "purchase_receipt".to_string(),
            template_content: "标准采购收货单打印模板（含收货明细、质检结果、入库确认）"
                .to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 5,
            name: "库存调拨单打印模板".to_string(),
            doc_type: "inventory_transfer".to_string(),
            template_content: "标准库存调拨单打印模板（含调出/调入仓库、调拨明细）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 6,
            name: "会计凭证打印模板".to_string(),
            doc_type: "voucher".to_string(),
            template_content: "标准会计凭证打印模板（含科目分录、借贷金额、凭证摘要）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 7,
            name: "售后处理单打印模板".to_string(),
            doc_type: "after_sales".to_string(),
            template_content: "售后处理单打印模板（含处理方案、客户反馈）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 8,
            name: "应付发票打印模板".to_string(),
            doc_type: "ap_invoice".to_string(),
            template_content: "应付发票打印模板（含发票信息、供应商信息）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 9,
            name: "应付账款付款单打印模板".to_string(),
            doc_type: "ap_payment".to_string(),
            template_content: "应付账款付款单打印模板（含付款明细、供应商信息）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 10,
            name: "付款申请单打印模板".to_string(),
            doc_type: "ap_payment_request".to_string(),
            template_content: "付款申请单打印模板（含申请金额、审批流程）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 11,
            name: "应付对账单打印模板".to_string(),
            doc_type: "ap_reconciliation".to_string(),
            template_content: "应付对账单打印模板（含对账明细、余额确认）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 12,
            name: "应收账款收款单打印模板".to_string(),
            doc_type: "ar_collection".to_string(),
            template_content: "应收账款收款单打印模板（含收款明细、核销记录）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 13,
            name: "应收对账单打印模板".to_string(),
            doc_type: "ar_reconciliation".to_string(),
            template_content: "应收对账单打印模板（含对账明细、余额确认）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 14,
            name: "坏账核销打印模板".to_string(),
            doc_type: "bad_debt_writeoff".to_string(),
            template_content: "坏账核销打印模板（含核销原因、审批流程）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 15,
            name: "BOM物料清单打印模板".to_string(),
            doc_type: "bom".to_string(),
            template_content: "BOM物料清单打印模板（含物料组成、用量）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 16,
            name: "大货色审批单打印模板".to_string(),
            doc_type: "bulk_color_approval".to_string(),
            template_content: "大货色审批单打印模板（含审批流程、色差数据）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 17,
            name: "原产地证书打印模板".to_string(),
            doc_type: "certificate_of_origin".to_string(),
            template_content: "原产地证书打印模板（含原产地信息、签发机构）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 18,
            name: "化工领料单打印模板".to_string(),
            doc_type: "chemical_requisition".to_string(),
            template_content: "化工领料单打印模板（含领用物料、用途说明）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 19,
            name: "色卡发放单打印模板".to_string(),
            doc_type: "color_card_issue".to_string(),
            template_content: "色卡发放单打印模板（含客户信息、色号、发放记录）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 20,
            name: "来样定制单打印模板".to_string(),
            doc_type: "custom_order".to_string(),
            template_content: "来样定制单打印模板（含客户需求、工艺要求）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 21,
            name: "客户信用额度打印模板".to_string(),
            doc_type: "customer_credit".to_string(),
            template_content: "客户信用额度打印模板（含信用额度、账期）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 22,
            name: "染色批次卡打印模板".to_string(),
            doc_type: "dye_batch_card".to_string(),
            template_content: "染色批次卡打印模板（含配方、工艺参数、色号）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 23,
            name: "染色批次返工打印模板".to_string(),
            doc_type: "dye_batch_rework".to_string(),
            template_content: "染色批次返工打印模板（含返工原因、处理方案）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 24,
            name: "能源消耗记录打印模板".to_string(),
            doc_type: "energy_consumption_record".to_string(),
            template_content: "能源消耗记录打印模板（含能耗数据、分析结果）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 25,
            name: "出口报关单打印模板".to_string(),
            doc_type: "export_customs_declaration".to_string(),
            template_content: "出口报关单打印模板（含报关信息、HS编码）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 26,
            name: "出口报检单打印模板".to_string(),
            doc_type: "export_inspection".to_string(),
            template_content: "出口报检单打印模板（含报检信息、检验检疫要求）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 27,
            name: "出口退税申报打印模板".to_string(),
            doc_type: "export_refund_declaration".to_string(),
            template_content: "出口退税申报打印模板（含退税明细、申报信息）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 28,
            name: "验布记录打印模板".to_string(),
            doc_type: "fabric_inspection".to_string(),
            template_content: "验布记录打印模板（含疵点记录、检验结果）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 29,
            name: "固定资产打印模板".to_string(),
            doc_type: "fixed_asset".to_string(),
            template_content: "固定资产打印模板（含资产信息、折旧方法）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 30,
            name: "固定资产盘点打印模板".to_string(),
            doc_type: "fixed_asset_count".to_string(),
            template_content: "固定资产盘点打印模板（含盘点明细、差异记录）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 31,
            name: "外汇核销打印模板".to_string(),
            doc_type: "foreign_exchange_verification".to_string(),
            template_content: "外汇核销打印模板（含核销明细、收汇信息）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 32,
            name: "库存调整单打印模板".to_string(),
            doc_type: "inventory_adjustment".to_string(),
            template_content: "库存调整单打印模板（含调整原因、调整明细）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 33,
            name: "库存减值打印模板".to_string(),
            doc_type: "inventory_write_down".to_string(),
            template_content: "库存减值打印模板（含减值原因、减值金额）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 34,
            name: "打样申请单打印模板".to_string(),
            doc_type: "lab_dip_request".to_string(),
            template_content: "打样申请单打印模板（含客户色号、打样要求）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 35,
            name: "劳动合同打印模板".to_string(),
            doc_type: "labor_contract".to_string(),
            template_content: "劳动合同打印模板（含合同条款、双方信息）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 36,
            name: "物流运单打印模板".to_string(),
            doc_type: "logistics_waybill".to_string(),
            template_content: "物流运单打印模板（含收发货人、货物信息）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 37,
            name: "物料短缺报告打印模板".to_string(),
            doc_type: "material_shortage".to_string(),
            template_content: "物料短缺报告打印模板（含短缺物料、影响分析）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 38,
            name: "职业危害监测打印模板".to_string(),
            doc_type: "occupational_hazard_monitoring".to_string(),
            template_content: "职业危害监测打印模板（含监测数据、结论）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 39,
            name: "职业健康检查打印模板".to_string(),
            doc_type: "occupational_health_exam".to_string(),
            template_content: "职业健康检查打印模板（含体检项目、结果）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 40,
            name: "委外订单打印模板".to_string(),
            doc_type: "outsourcing_order".to_string(),
            template_content: "委外订单打印模板（含委外工序、供应商信息）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 41,
            name: "委外收货单打印模板".to_string(),
            doc_type: "outsourcing_receipt".to_string(),
            template_content: "委外收货单打印模板（含收货明细、质检结果）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 42,
            name: "排污许可证打印模板".to_string(),
            doc_type: "pollution_permit".to_string(),
            template_content: "排污许可证打印模板（含许可排放量、有效期）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 43,
            name: "劳保用品发放打印模板".to_string(),
            doc_type: "ppe_distribution".to_string(),
            template_content: "劳保用品发放打印模板（含发放明细、签收记录）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 44,
            name: "工艺路线打印模板".to_string(),
            doc_type: "process_route".to_string(),
            template_content: "工艺路线打印模板（含工序步骤、工艺参数）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 45,
            name: "生产流转卡打印模板".to_string(),
            doc_type: "production_flow_card".to_string(),
            template_content: "生产流转卡打印模板（含工序流转、质量记录）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 46,
            name: "生产工单打印模板".to_string(),
            doc_type: "production_order".to_string(),
            template_content: "生产工单打印模板（含产品信息、数量、交期）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 47,
            name: "生产配方打印模板".to_string(),
            doc_type: "production_recipe".to_string(),
            template_content: "生产配方打印模板（含染料配方、工艺参数）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 48,
            name: "采购合同打印模板".to_string(),
            doc_type: "purchase_contract".to_string(),
            template_content: "采购合同打印模板（含合同条款、供应商信息）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 49,
            name: "采购质检单打印模板".to_string(),
            doc_type: "purchase_inspection".to_string(),
            template_content: "采购质检单打印模板（含检验项目、合格率）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 50,
            name: "采购退货单打印模板".to_string(),
            doc_type: "purchase_return".to_string(),
            template_content: "采购退货单打印模板（含退货原因、退货明细）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 51,
            name: "8D质量报告打印模板".to_string(),
            doc_type: "quality_8d_report".to_string(),
            template_content: "8D质量报告打印模板（含问题分析、纠正措施）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 52,
            name: "质检记录打印模板".to_string(),
            doc_type: "quality_inspection_record".to_string(),
            template_content: "质检记录打印模板（含检验项目、评分结果）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 53,
            name: "质量问题单打印模板".to_string(),
            doc_type: "quality_issue".to_string(),
            template_content: "质量问题单打印模板（含问题描述、处理方案）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 54,
            name: "安全事故报告打印模板".to_string(),
            doc_type: "safety_accident_report".to_string(),
            template_content: "安全事故报告打印模板（含事故经过、处理措施）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 55,
            name: "销售发货单打印模板".to_string(),
            doc_type: "sales_delivery".to_string(),
            template_content: "销售发货单打印模板（含发货明细、物流信息）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 56,
            name: "销售报价单打印模板".to_string(),
            doc_type: "sales_quotation".to_string(),
            template_content: "销售报价单打印模板（含报价明细、有效期）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 57,
            name: "销售退货单打印模板".to_string(),
            doc_type: "sales_return".to_string(),
            template_content: "销售退货单打印模板（含退货原因、退货明细）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 58,
            name: "排程结果打印模板".to_string(),
            doc_type: "scheduling_result".to_string(),
            template_content: "排程结果打印模板（含排产计划、资源分配）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 59,
            name: "社保缴纳记录打印模板".to_string(),
            doc_type: "social_insurance_record".to_string(),
            template_content: "社保缴纳记录打印模板（含缴费基数、险种明细）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 60,
            name: "固废处置打印模板".to_string(),
            doc_type: "solid_waste_disposal".to_string(),
            template_content: "固废处置打印模板（含处置方式、合规记录）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 61,
            name: "供应商评级打印模板".to_string(),
            doc_type: "supplier_evaluation_record".to_string(),
            template_content: "供应商评级打印模板（含评分指标、评级结果）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 62,
            name: "不合格品处理打印模板".to_string(),
            doc_type: "unqualified_product".to_string(),
            template_content: "不合格品处理打印模板（含处置方式、原因分析）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        PrintTemplateDto {
            id: 63,
            name: "工资记录打印模板".to_string(),
            doc_type: "wage_record".to_string(),
            template_content: "工资记录打印模板（含计件工资、工序明细）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
    ]
}

/// 获取打印模板列表：从模板存储读取（含系统内置模板 + 用户新建的模板）
pub async fn list_print_templates(
    State(_): State<AppState>,
    _auth: AuthContext,
) -> Result<axum::Json<ApiResponse<Vec<PrintTemplateRecord>>>, AppError> {
    let records = lock_print_templates()?;
    Ok(axum::Json(ApiResponse::success(records.clone())))
}

/// 获取单个打印模板详情，找不到时返回 404 not_found
pub async fn get_print_template(
    Path(id): Path<i32>,
    State(_): State<AppState>,
    _auth: AuthContext,
) -> Result<axum::Json<ApiResponse<PrintTemplateRecord>>, AppError> {
    let records = lock_print_templates()?;
    let record = records
        .iter()
        .find(|r| r.id == id)
        .ok_or_else(|| AppError::not_found(format!("打印模板 {} 不存在", id)))?;
    Ok(axum::Json(ApiResponse::success(record.clone())))
}

// ---------- 打印模板最小 CRUD（内存存储，预填充系统内置模板） ----------

static PRINT_TEMPLATE_STORE: OnceLock<Mutex<Vec<PrintTemplateRecord>>> = OnceLock::new();

fn print_template_store() -> &'static Mutex<Vec<PrintTemplateRecord>> {
    PRINT_TEMPLATE_STORE.get_or_init(|| {
        Mutex::new(
            builtin_print_templates()
                .into_iter()
                .map(print_template_record_from_builtin)
                .collect(),
        )
    })
}

fn lock_print_templates() -> Result<MutexGuard<'static, Vec<PrintTemplateRecord>>, AppError> {
    print_template_store()
        .lock()
        .map_err(|_| AppError::internal("打印模板存储不可用"))
}

fn print_template_record_from_builtin(dto: PrintTemplateDto) -> PrintTemplateRecord {
    let module = infer_print_module(&dto.doc_type);
    let template_type = infer_print_template_type(&dto.doc_type);
    PrintTemplateRecord {
        id: dto.id,
        template_code: dto.doc_type,
        template_name: dto.name,
        description: dto.template_content.clone(),
        module,
        template_type,
        paper_size: "A4".to_string(),
        orientation: "portrait".to_string(),
        content: dto.template_content,
        css_styles: String::new(),
        variables: serde_json::json!({}),
        status: "active".to_string(),
        is_default: dto.is_default,
        created_by: 0,
        created_by_name: "system".to_string(),
        created_at: dto.created_at.clone(),
        updated_at: dto.created_at,
    }
}

fn infer_print_module(doc_type: &str) -> String {
    if doc_type.starts_with("sales")
        || doc_type == "after_sales"
        || doc_type == "customer_credit"
    {
        "sales".to_string()
    } else if doc_type.starts_with("purchase") {
        "purchase".to_string()
    } else if doc_type.starts_with("inventory") {
        "inventory".to_string()
    } else if doc_type == "voucher"
        || doc_type.starts_with("ap_")
        || doc_type.starts_with("ar_")
        || doc_type.starts_with("bad_debt")
        || doc_type.starts_with("fixed_asset")
        || doc_type.starts_with("foreign_exchange")
        || doc_type.starts_with("export_refund")
    {
        "finance".to_string()
    } else if doc_type.starts_with("production")
        || doc_type == "bom"
        || doc_type.starts_with("dye_")
        || doc_type.starts_with("process_")
        || doc_type.starts_with("scheduling_")
        || doc_type.starts_with("chemical_")
        || doc_type.starts_with("lab_dip_")
        || doc_type.starts_with("outsourcing_")
        || doc_type.starts_with("energy_")
        || doc_type.starts_with("material_")
        || doc_type.starts_with("bulk_color_")
        || doc_type.starts_with("color_card_")
        || doc_type == "custom_order"
    {
        "production".to_string()
    } else {
        "logistics".to_string()
    }
}

fn infer_print_template_type(doc_type: &str) -> String {
    if doc_type.contains("invoice") {
        "invoice".to_string()
    } else if doc_type.contains("receipt") || doc_type.contains("reconciliation") {
        "receipt".to_string()
    } else if doc_type.contains("report")
        || doc_type.contains("record")
        || doc_type.contains("issue")
        || doc_type.contains("inspection")
    {
        "report".to_string()
    } else {
        "order".to_string()
    }
}

#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Clone, serde::Serialize)]
pub struct PrintTemplateRecord {
    pub id: i32,
    pub template_code: String,
    pub template_name: String,
    pub description: String,
    pub module: String,
    #[serde(rename = "type")]
    pub template_type: String,
    pub paper_size: String,
    pub orientation: String,
    pub content: String,
    pub css_styles: String,
    pub variables: serde_json::Value,
    pub status: String,
    pub is_default: bool,
    pub created_by: i32,
    pub created_by_name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, serde::Serialize)]
pub struct PrintTemplatePreviewResponse {
    pub html: String,
    pub variables: serde_json::Value,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct CreatePrintTemplateRequest {
    #[validate(length(min = 1, max = 100, message = "模板名称长度须为 1-100 字符"))]
    pub template_name: String,
    #[validate(length(max = 100, message = "模板编码长度不得超过 100 字符"))]
    pub template_code: Option<String>,
    #[validate(length(max = 500, message = "描述长度不得超过 500 字符"))]
    pub description: Option<String>,
    pub module: Option<String>,
    #[serde(rename = "type")]
    pub template_type: Option<String>,
    pub paper_size: Option<String>,
    pub orientation: Option<String>,
    pub content: Option<String>,
    pub css_styles: Option<String>,
    pub variables: Option<serde_json::Value>,
    pub is_default: Option<bool>,
    pub status: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct UpdatePrintTemplateRequest {
    #[validate(length(min = 1, max = 100, message = "模板名称长度须为 1-100 字符"))]
    pub template_name: Option<String>,
    #[validate(length(max = 100, message = "模板编码长度不得超过 100 字符"))]
    pub template_code: Option<String>,
    #[validate(length(max = 500, message = "描述长度不得超过 500 字符"))]
    pub description: Option<String>,
    pub module: Option<String>,
    #[serde(rename = "type")]
    pub template_type: Option<String>,
    pub paper_size: Option<String>,
    pub orientation: Option<String>,
    pub content: Option<String>,
    pub css_styles: Option<String>,
    pub variables: Option<serde_json::Value>,
    pub is_default: Option<bool>,
    pub status: Option<String>,
}

fn clear_default_in_module(
    records: &mut [PrintTemplateRecord],
    module: &str,
    except_id: Option<i32>,
) {
    for r in records.iter_mut() {
        if r.module == module && Some(r.id) != except_id {
            r.is_default = false;
        }
    }
}

pub async fn create_print_template(
    State(_state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreatePrintTemplateRequest>,
) -> Result<axum::Json<ApiResponse<PrintTemplateRecord>>, AppError> {
    info!("用户 {} 创建打印模板：{}", auth.username, req.template_name);
    req.validate()?;

    let now = chrono::Utc::now().to_rfc3339();
    let mut records = lock_print_templates()?;
    let next_id = records.iter().map(|r| r.id).max().unwrap_or(0) + 1;
    let is_default = req.is_default.unwrap_or(false);

    if is_default {
        clear_default_in_module(&mut records, &req.module.clone().unwrap_or_default(), None);
    }

    let record = PrintTemplateRecord {
        id: next_id,
        template_code: req
            .template_code
            .unwrap_or_else(|| format!("TPL_{}", next_id)),
        template_name: req.template_name,
        description: req.description.unwrap_or_default(),
        module: req.module.unwrap_or_else(|| "logistics".to_string()),
        template_type: req.template_type.unwrap_or_else(|| "custom".to_string()),
        paper_size: req.paper_size.unwrap_or_else(|| "A4".to_string()),
        orientation: req.orientation.unwrap_or_else(|| "portrait".to_string()),
        content: req.content.unwrap_or_default(),
        css_styles: req.css_styles.unwrap_or_default(),
        variables: req.variables.unwrap_or_else(|| serde_json::json!({})),
        status: req.status.unwrap_or_else(|| "active".to_string()),
        is_default,
        created_by: auth.user_id,
        created_by_name: auth.username.clone(),
        created_at: now.clone(),
        updated_at: now,
    };
    records.push(record.clone());

    Ok(axum::Json(ApiResponse::success_with_message(
        record,
        "打印模板创建成功",
    )))
}

pub async fn update_print_template(
    Path(id): Path<i32>,
    State(_state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdatePrintTemplateRequest>,
) -> Result<axum::Json<ApiResponse<PrintTemplateRecord>>, AppError> {
    info!("用户 {} 更新打印模板 ID: {}", auth.username, id);
    req.validate()?;

    let mut records = lock_print_templates()?;
    let record = records
        .iter_mut()
        .find(|r| r.id == id)
        .ok_or_else(|| AppError::not_found(format!("打印模板 {} 不存在", id)))?;

    if req.is_default == Some(true) {
        let module = record.module.clone();
        clear_default_in_module(&mut records, &module, Some(id));
    }

    if let Some(v) = req.template_name {
        record.template_name = v;
    }
    if let Some(v) = req.template_code {
        record.template_code = v;
    }
    if let Some(v) = req.description {
        record.description = v;
    }
    if let Some(v) = req.module {
        record.module = v;
    }
    if let Some(v) = req.template_type {
        record.template_type = v;
    }
    if let Some(v) = req.paper_size {
        record.paper_size = v;
    }
    if let Some(v) = req.orientation {
        record.orientation = v;
    }
    if let Some(v) = req.content {
        record.content = v;
    }
    if let Some(v) = req.css_styles {
        record.css_styles = v;
    }
    if let Some(v) = req.variables {
        record.variables = v;
    }
    if let Some(v) = req.is_default {
        record.is_default = v;
    }
    if let Some(v) = req.status {
        record.status = v;
    }
    record.updated_at = chrono::Utc::now().to_rfc3339();

    let updated = record.clone();
    Ok(axum::Json(ApiResponse::success_with_message(
        updated,
        "打印模板更新成功",
    )))
}

pub async fn delete_print_template(
    Path(id): Path<i32>,
    State(_state): State<AppState>,
    auth: AuthContext,
) -> Result<axum::Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 删除打印模板 ID: {}", auth.username, id);

    let mut records = lock_print_templates()?;
    let before = records.len();
    records.retain(|r| r.id != id);
    if records.len() == before {
        return Err(AppError::not_found(format!("打印模板 {} 不存在", id)));
    }

    Ok(axum::Json(ApiResponse::success_with_message(
        (),
        "打印模板删除成功",
    )))
}

pub async fn preview_print_template(
    Path(id): Path<i32>,
    State(_state): State<AppState>,
    auth: AuthContext,
    body: Option<Json<serde_json::Value>>,
) -> Result<axum::Json<ApiResponse<PrintTemplatePreviewResponse>>, AppError> {
    info!("用户 {} 预览打印模板 ID: {}", auth.username, id);

    let records = lock_print_templates()?;
    let record = records
        .iter()
        .find(|r| r.id == id)
        .ok_or_else(|| AppError::not_found(format!("打印模板 {} 不存在", id)))?
        .clone();
    drop(records);

    let variables = body
        .and_then(|Json(v)| v.as_object().cloned())
        .unwrap_or_default();
    let mut html = format!(
        "<html><head><title>{}</title><style>{}</style></head><body><h3>{}</h3><div>{}</div></body></html>",
        record.template_name,
        record.css_styles.replace('\n', " "),
        record.template_name,
        record.content
    );
    for (key, value) in &variables {
        let placeholder = format!("{{{{{}}}}}", key);
        html = html.replace(&placeholder, &value.to_string());
    }

    Ok(axum::Json(ApiResponse::success(PrintTemplatePreviewResponse {
        html,
        variables: serde_json::Value::Object(variables),
    })))
}

pub async fn print_print_template(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    _body: Option<Json<serde_json::Value>>,
) -> Result<axum::Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 使用打印模板打印 ID: {}", auth.username, id);

    let records = lock_print_templates()?;
    records
        .iter()
        .find(|r| r.id == id)
        .ok_or_else(|| AppError::not_found(format!("打印模板 {} 不存在", id)))?;
    drop(records);

    record_print_audit(&state, &auth, "print_template", id);

    Ok(axum::Json(ApiResponse::success_with_message(
        (),
        "打印任务已受理",
    )))
}

pub async fn set_default_print_template(
    Path(id): Path<i32>,
    State(_state): State<AppState>,
    auth: AuthContext,
) -> Result<axum::Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 设置打印模板默认 ID: {}", auth.username, id);

    let mut records = lock_print_templates()?;
    let module = records
        .iter()
        .find(|r| r.id == id)
        .ok_or_else(|| AppError::not_found(format!("打印模板 {} 不存在", id)))?
        .module
        .clone();
    clear_default_in_module(&mut records, &module, Some(id));
    let record = records
        .iter_mut()
        .find(|r| r.id == id)
        .ok_or_else(|| AppError::not_found(format!("打印模板 {} 不存在", id)))?;
    record.is_default = true;
    record.updated_at = chrono::Utc::now().to_rfc3339();

    Ok(axum::Json(ApiResponse::success_with_message(
        (),
        "默认打印模板设置成功",
    )))
}

pub async fn copy_print_template(
    Path(id): Path<i32>,
    State(_state): State<AppState>,
    auth: AuthContext,
) -> Result<axum::Json<ApiResponse<PrintTemplateRecord>>, AppError> {
    info!("用户 {} 复制打印模板 ID: {}", auth.username, id);

    let mut records = lock_print_templates()?;
    let source = records
        .iter()
        .find(|r| r.id == id)
        .ok_or_else(|| AppError::not_found(format!("打印模板 {} 不存在", id)))?
        .clone();
    let next_id = records.iter().map(|r| r.id).max().unwrap_or(0) + 1;

    let now = chrono::Utc::now().to_rfc3339();
    let mut record = source;
    record.id = next_id;
    record.template_code = format!("{}_COPY_{}", record.template_code, next_id);
    record.template_name = format!("{}_副本", record.template_name);
    record.is_default = false;
    record.status = "active".to_string();
    record.created_by = auth.user_id;
    record.created_by_name = auth.username.clone();
    record.created_at = now.clone();
    record.updated_at = now;
    records.push(record.clone());

    Ok(axum::Json(ApiResponse::success_with_message(
        record,
        "打印模板复制成功",
    )))
}
