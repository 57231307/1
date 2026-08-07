//! 通用打印 Handler

use std::sync::Arc;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::print_service::PrintService;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
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
    Path(doc_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "sales_delivery", doc_id).await?;
    record_print_audit(&state, &auth, "sales_delivery", doc_id);
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
#[derive(serde::Serialize)]
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
fn builtin_print_templates() -> Vec<PrintTemplateDto> {
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

/// 获取打印模板列表；批次 126 v8 复审 P2 修复：从原空列表占位改为返回系统内置 6 种单据打印模板。 模板对应 PrintService 支持的 6
/// 种单据类型（sales_order/sales_contract/purchase_order/ purchase_receipt/inventory_transfer/voucher）。
pub async fn list_print_templates(
    State(_): State<AppState>,
    _auth: AuthContext,
) -> Result<axum::Json<ApiResponse<Vec<PrintTemplateDto>>>, AppError> {
    // V15 P0-S09：注入 AuthContext，强制要求用户已认证；打印模板元数据查询走 read 权限
    Ok(axum::Json(ApiResponse::success(builtin_print_templates())))
}

/// 获取单个打印模板详情；批次 126 v8 复审 P2 修复：从原硬编码 not_found 改为从内置模板列表按 id 查找。 找不到时返回 404 not_found。
pub async fn get_print_template(
    Path(id): Path<i32>,
    State(_): State<AppState>,
    _auth: AuthContext,
) -> Result<axum::Json<ApiResponse<PrintTemplateDto>>, AppError> {
    // V15 P0-S09：注入 AuthContext，强制要求用户已认证；打印模板元数据查询走 read 权限
    let template = builtin_print_templates()
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| AppError::not_found(format!("打印模板 {} 不存在", id)))?;
    Ok(axum::Json(ApiResponse::success(template)))
}

#[cfg(test)]
mod tests {
    //! 打印 Handler 单元测试（批次 394 补测）
    //!
    //! 覆盖目标：
    //! - builtin_print_templates 静态模板列表（5 个测试）

    use super::*;

    /// test_builtin_print_templatesfh6gmb；验证内置打印模板数量为 6（对应 6 种单据类型）
    #[test]
    fn test_builtin_print_templatesfh6gmb() {
        let templates = builtin_print_templates();
        assert_eq!(templates.len(), 63, "应有 6 个内置打印模板");
    }

    /// test_builtin_print_templates_idwyqlx；验证 6 个模板的 id 为 1-6，唯一且连续
    #[test]
    fn test_builtin_print_templates_idwyqlx() {
        let templates = builtin_print_templates();
        let ids: Vec<i32> = templates.iter().map(|t| t.id).collect();
        assert_eq!(ids, (1..=63).collect::<Vec<i32>>(), "id 应为 1-6 连续");

        // 唯一性检查
        let unique_ids: std::collections::HashSet<i32> = ids.iter().copied().collect();
        assert_eq!(unique_ids.len(), 6, "id 应唯一");
    }

    /// test_builtin_print_templates_doc_typewy；验证 6 个模板的 doc_type 互不相同
    #[test]
    fn test_builtin_print_templates_doc_typewy() {
        let templates = builtin_print_templates();
        let doc_types: Vec<&str> = templates.iter().map(|t| t.doc_type.as_str()).collect();
        let unique: std::collections::HashSet<&str> = doc_types.iter().copied().collect();
        assert_eq!(unique.len(), 6, "doc_type 应唯一");
    }

    /// test_builtin_print_templatesqbwmrmb；验证所有内置模板的 is_default 均为 true
    #[test]
    fn test_builtin_print_templatesqbwmrmb() {
        let templates = builtin_print_templates();
        for t in &templates {
            assert!(t.is_default, "模板 {} 应为默认模板", t.name);
        }
    }

    /// test_builtin_print_templatesfg6zdjlx；验证模板覆盖全部 6 种业务单据类型： sales_order /
    /// sales_contract / purchase_order / purchase_receipt / inventory_transfer / voucher
    #[test]
    fn test_builtin_print_templatesfg6zdjlx() {
        let templates = builtin_print_templates();
        let doc_types: Vec<&str> = templates.iter().map(|t| t.doc_type.as_str()).collect();

        let expected = [
            "sales_order",
            "sales_contract",
            "purchase_order",
            "purchase_receipt",
            "inventory_transfer",
            "voucher",
        ];
        for t in &expected {
            assert!(doc_types.contains(t), "应包含单据类型 {}", t);
        }

        // 名称不应为空
        for t in &templates {
            assert!(!t.name.is_empty(), "模板 {} 的名称不应为空", t.doc_type);
            assert!(
                !t.template_content.is_empty(),
                "模板 {} 的内容不应为空",
                t.doc_type
            );
        }
    }
}
