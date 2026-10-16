//! 通用打印 Handler

use std::sync::Arc;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, Order, QueryFilter, QueryOrder,
    Set, TransactionTrait,
};
use serde::Deserialize;
use validator::Validate;

use crate::container::AppState;
use crate::handlers::print_task_handler::{
    CreatePrintTaskRequest, PrintTaskJobRef, submit_print_task,
};
use crate::middleware::auth_context::AuthContext;
use crate::models::audit_log::{OperationType, Severity};
use crate::models::print_template;
use crate::models::status::print::print_template_status;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::print_service::PrintService;
use crate::utils::error::AppError;
use crate::utils::optional_json::OptionalJson;
use crate::utils::response::ApiResponse;
use axum::{
    Json,
    extract::{Path, State},
    response::Response,
};
use tracing::info;

/// 按单据类型渲染打印 docx：先装配打印数据（字段缺失/业务门控以既有 400 形态暴露），
/// 再取持久化模板布局（经 [`print_task_handler::resolve_print_docx_layout`] 单一来源解析，
/// 无可用模板行时 404，绝不以内置默认值代渲染），最后生成 docx 响应。
async fn render_print_docx(
    state: &AppState,
    doc_type: &str,
    doc_id: i32,
) -> Result<Response, AppError> {
    let service = PrintService::new(state.db.clone());
    // 先装配打印数据：字段缺失/业务门控在此以既有 400 形态暴露（先于模板存在性判断）。
    let print_data = service.get_print_data(doc_type, doc_id).await?;
    // 再取持久化模板布局：与打印任务受理链路共用同一解析入口（单一事实来源）。
    let layout =
        crate::handlers::print_task_handler::resolve_print_docx_layout(&*state.db, doc_type)
            .await?;
    let bytes = service.generate_docx(&print_data, &layout).await?;
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

/// 成品布入库打印标签（doc_type inventory_piece_label，路由 /inventory/pieces/{id}/print）
/// 数据装配与 fail-closed 校验（非 dyed/样布业务族拒绝、缺字段逐列点名 VALIDATION 族）
/// 全部收敛在 PrintService::get_inventory_piece_label_print_data，本 handler 仅走既有渲染+审计范式
pub async fn inventory_piece_label_print_docx(
    Path(piece_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Response, AppError> {
    let resp = render_print_docx(&state, "inventory_piece_label", piece_id).await?;
    record_print_audit(&state, &auth, "inventory_piece_label", piece_id);
    Ok(resp)
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

/// 系统内置打印模板种子集合：对应 PrintService 支持的单据类型，是 `print_templates` 表的种子数据源，
/// 在应用启动时 upsert 落库。正文描述字段仅供展示，渲染逻辑在 PrintService.generate_docx；
/// 纸张/方向/边距/条码以 print_templates 表列持久化。
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
        PrintTemplateDto {
            id: 64,
            name: "成品布入库标签打印模板".to_string(),
            doc_type: "inventory_piece_label".to_string(),
            template_content: "成品布入库标签打印模板（含款号、品名、缸号、色号、批次、匹号、米数、重量、幅宽、克重、等级、条码）".to_string(),
            is_default: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
    ]
}

/// 获取打印模板列表：读取 `print_templates` 表全部模板行（按 id 升序）
pub async fn list_print_templates(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<axum::Json<ApiResponse<Vec<PrintTemplateRecord>>>, AppError> {
    let models = print_template::Entity::find()
        .order_by(print_template::Column::Id, Order::Asc)
        .all(&*state.db)
        .await?;
    let records: Vec<PrintTemplateRecord> = models.into_iter().map(to_record).collect();
    Ok(axum::Json(ApiResponse::success(records)))
}

/// 获取单个打印模板详情，找不到时返回 404 not_found
pub async fn get_print_template(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<axum::Json<ApiResponse<PrintTemplateRecord>>, AppError> {
    let model = print_template::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found(format!("打印模板 {} 不存在", id)))?;
    Ok(axum::Json(ApiResponse::success(to_record(model))))
}

/// 将 print_templates 行映射为出参结构：字段名/序列化键与前端契约保持一致，
/// 时间列以 RFC3339 字符串输出（created_at/updated_at 仍为 String）。
fn to_record(m: print_template::Model) -> PrintTemplateRecord {
    PrintTemplateRecord {
        id: m.id,
        template_code: m.template_code,
        template_name: m.template_name,
        description: m.description,
        module: m.module,
        template_type: m.template_type,
        paper_size: m.paper_size,
        orientation: m.orientation,
        content: m.content,
        css_styles: m.css_styles,
        variables: m.variables,
        status: m.status,
        is_default: m.is_default,
        created_by: m.created_by,
        created_by_name: m.created_by_name,
        created_at: m.created_at.to_rfc3339(),
        updated_at: m.updated_at.to_rfc3339(),
    }
}

/// 按 id 取模板行，缺失时返回 404（供 set-default / copy / preview / print 复用）
async fn load_template(
    db: &sea_orm::DatabaseConnection,
    id: i32,
) -> Result<print_template::Model, AppError> {
    print_template::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::not_found(format!("打印模板 {} 不存在", id)))
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

fn now_active_value() -> sea_orm::ActiveValue<sea_orm::entity::prelude::DateTimeWithTimeZone> {
    Set(chrono::Utc::now().into())
}

pub async fn create_print_template(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreatePrintTemplateRequest>,
) -> Result<axum::Json<ApiResponse<PrintTemplateRecord>>, AppError> {
    info!("用户 {} 创建打印模板：{}", auth.username, req.template_name);
    req.validate()?;

    let template_code = req.template_code.clone().unwrap_or_else(|| {
        format!(
            "TPL_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        )
    });
    let module = req
        .module
        .clone()
        .unwrap_or_else(|| "logistics".to_string());
    let is_default = req.is_default.unwrap_or(false);
    // doc_type 为“单据→模板”查找键且列 NOT NULL；出参结构不含该列，
    // 新建时以模板编码兼作 doc_type（与内置种子 template_code 同值口径一致）。
    let doc_type = template_code.clone();

    let active = print_template::ActiveModel {
        id: Default::default(),
        template_code: Set(template_code),
        template_name: Set(req.template_name),
        description: req.description.map(Set).unwrap_or_default(),
        module: Set(module.clone()),
        template_type: req.template_type.map(Set).unwrap_or_default(),
        doc_type: Set(doc_type),
        paper_size: req.paper_size.map(Set).unwrap_or_default(),
        orientation: req.orientation.map(Set).unwrap_or_default(),
        content: req.content.map(Set).unwrap_or_default(),
        css_styles: req.css_styles.map(Set).unwrap_or_default(),
        variables: req.variables.map(Set).unwrap_or_default(),
        status: req.status.map(Set).unwrap_or_default(),
        is_default: Set(is_default),
        created_by: Set(auth.user_id),
        created_by_name: Set(auth.username.clone()),
        ..Default::default()
    };

    if is_default {
        // 同模块至多一个默认模板：先清其余、再落新行，置于同一事务内。
        let txn = state.db.begin().await?;
        print_template::Entity::update_many()
            .col_expr(
                print_template::Column::IsDefault,
                sea_orm::sea_query::Expr::value(false),
            )
            .filter(print_template::Column::Module.eq(module.as_str()))
            .exec(&txn)
            .await?;
        let model = active.insert(&txn).await?;
        txn.commit().await?;
        return Ok(axum::Json(ApiResponse::success_with_message(
            to_record(model),
            "打印模板创建成功",
        )));
    }

    let model = active.insert(&*state.db).await?;
    Ok(axum::Json(ApiResponse::success_with_message(
        to_record(model),
        "打印模板创建成功",
    )))
}

pub async fn update_print_template(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdatePrintTemplateRequest>,
) -> Result<axum::Json<ApiResponse<PrintTemplateRecord>>, AppError> {
    info!("用户 {} 更新打印模板 ID: {}", auth.username, id);
    req.validate()?;

    let existing = load_template(&*state.db, id).await?;
    let module_for_default = req
        .module
        .clone()
        .unwrap_or_else(|| existing.module.clone());

    let mut active: print_template::ActiveModel = existing.into_active_model();
    if let Some(v) = req.template_name {
        active.template_name = Set(v);
    }
    if let Some(v) = req.template_code {
        active.template_code = Set(v);
    }
    if let Some(v) = req.description {
        active.description = Set(v);
    }
    if let Some(v) = req.module {
        active.module = Set(v);
    }
    if let Some(v) = req.template_type {
        active.template_type = Set(v);
    }
    if let Some(v) = req.paper_size {
        active.paper_size = Set(v);
    }
    if let Some(v) = req.orientation {
        active.orientation = Set(v);
    }
    if let Some(v) = req.content {
        active.content = Set(v);
    }
    if let Some(v) = req.css_styles {
        active.css_styles = Set(v);
    }
    if let Some(v) = req.variables {
        active.variables = Set(v);
    }
    if let Some(v) = req.status {
        active.status = Set(v);
    }

    if req.is_default == Some(true) {
        // 同模块至多一个默认模板：清其余、置本行，置于同一事务内。
        let txn = state.db.begin().await?;
        print_template::Entity::update_many()
            .col_expr(
                print_template::Column::IsDefault,
                sea_orm::sea_query::Expr::value(false),
            )
            .filter(print_template::Column::Module.eq(module_for_default.as_str()))
            .filter(print_template::Column::Id.ne(id))
            .exec(&txn)
            .await?;
        active.is_default = Set(true);
        active.updated_at = now_active_value();
        let model = active.update(&txn).await?;
        txn.commit().await?;
        return Ok(axum::Json(ApiResponse::success_with_message(
            to_record(model),
            "打印模板更新成功",
        )));
    }

    if let Some(v) = req.is_default {
        active.is_default = Set(v);
    }
    active.updated_at = now_active_value();
    let model = active.update(&*state.db).await?;
    Ok(axum::Json(ApiResponse::success_with_message(
        to_record(model),
        "打印模板更新成功",
    )))
}

pub async fn delete_print_template(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<axum::Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 删除打印模板 ID: {}", auth.username, id);

    load_template(&*state.db, id).await?;
    print_template::Entity::delete_by_id(id)
        .exec(&*state.db)
        .await?;

    Ok(axum::Json(ApiResponse::success_with_message(
        (),
        "打印模板删除成功",
    )))
}

pub async fn preview_print_template(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    body: OptionalJson<serde_json::Value>,
) -> Result<axum::Json<ApiResponse<PrintTemplatePreviewResponse>>, AppError> {
    info!("用户 {} 预览打印模板 ID: {}", auth.username, id);

    let record = to_record(load_template(&*state.db, id).await?);

    let variables = body
        .0
        .and_then(|v| v.as_object().cloned())
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

    Ok(axum::Json(ApiResponse::success(
        PrintTemplatePreviewResponse {
            html,
            variables: serde_json::Value::Object(variables),
        },
    )))
}

/// 从可选请求体中读取打印所需的单据实例主键（`doc_id`）。缺失即 `None`，由调用方转 fail-visible 400。
/// `doc_id` 为多态单据引用（BIGINT），按 i64 读取；非数字/越界一律视为缺失，不臆造样例打印。
fn extract_doc_id(body: &OptionalJson<serde_json::Value>) -> Option<i64> {
    body.0
        .as_ref()
        .and_then(|v| v.get("doc_id"))
        .and_then(|v| v.as_i64())
}

/// 从可选请求体中读取打印目标符号名覆盖（`target_name`），供模板打印时指定设备/池。
fn extract_target_override(body: &OptionalJson<serde_json::Value>) -> Option<String> {
    body.0
        .as_ref()
        .and_then(|v| v.get("target_name"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// `POST /print-templates/{id}/print`：以某打印模板发起一次真实打印任务（受理 + 后台派发）。
///
/// 模板管理页的“打印”按钮无单据实例上下文：本端点必须经请求体显式提供 `doc_id`（打印哪一张单据），
/// 缺失即 fail-visible 400「打印需指定单据实例」，绝不伪造样例打印或空跑。据模板 id 载入模板取其
/// doc_type 与缺省参数，复用 [`print_task_handler::submit_print_task`] 走与 `POST /print-tasks`
/// 完全相同的受理链路（校验→落 pending→后台生成 docx→派发），成功返回 `{ id, job_code }` 供前端轮询。
pub async fn print_print_template(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    body: OptionalJson<serde_json::Value>,
) -> Result<axum::Json<ApiResponse<PrintTaskJobRef>>, AppError> {
    info!("用户 {} 使用打印模板打印 ID: {}", auth.username, id);

    let template = load_template(&*state.db, id).await?;
    let doc_id = extract_doc_id(&body).ok_or_else(|| {
        AppError::bad_request("打印需指定单据实例（请求体缺少 doc_id，禁止伪造样例打印）")
    })?;

    let req = CreatePrintTaskRequest {
        doc_type: template.doc_type.clone(),
        doc_id,
        target_name: extract_target_override(&body),
        template_id: Some(id),
        copies: None,
        paper_size: None,
        orientation: None,
        margin_top_mm: None,
        margin_bottom_mm: None,
        margin_left_mm: None,
        margin_right_mm: None,
        barcode_enabled: None,
    };
    let job_ref = submit_print_task(&state, &auth, req).await?;

    record_print_audit(&state, &auth, "print_template", id);

    Ok(axum::Json(ApiResponse::success_with_message(
        job_ref,
        "打印任务已受理",
    )))
}

pub async fn set_default_print_template(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<axum::Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 设置打印模板默认 ID: {}", auth.username, id);

    let existing = load_template(&*state.db, id).await?;
    let module = existing.module.clone();

    // 同模块至多一个默认模板：先清其余、再置本行，置于同一事务内。
    let txn = state.db.begin().await?;
    print_template::Entity::update_many()
        .col_expr(
            print_template::Column::IsDefault,
            sea_orm::sea_query::Expr::value(false),
        )
        .filter(print_template::Column::Module.eq(module.as_str()))
        .filter(print_template::Column::Id.ne(id))
        .exec(&txn)
        .await?;
    let mut active: print_template::ActiveModel = existing.into_active_model();
    active.is_default = Set(true);
    active.updated_at = now_active_value();
    active.update(&txn).await?;
    txn.commit().await?;

    Ok(axum::Json(ApiResponse::success_with_message(
        (),
        "默认打印模板设置成功",
    )))
}

pub async fn copy_print_template(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<axum::Json<ApiResponse<PrintTemplateRecord>>, AppError> {
    info!("用户 {} 复制打印模板 ID: {}", auth.username, id);

    let source = load_template(&*state.db, id).await?;
    let new_code = format!(
        "{}_COPY_{}",
        source.template_code,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );

    // 复制为新行：逐列沿用来源模板（含纸张/方向/边距/条码布局），仅重置编码/名称/默认/状态/创建人。
    let active = print_template::ActiveModel {
        id: Default::default(),
        template_code: Set(new_code),
        template_name: Set(format!("{}_副本", source.template_name)),
        description: Set(source.description.clone()),
        module: Set(source.module.clone()),
        template_type: Set(source.template_type.clone()),
        doc_type: Set(source.doc_type.clone()),
        paper_size: Set(source.paper_size.clone()),
        orientation: Set(source.orientation.clone()),
        copies_default: Set(source.copies_default),
        margin_top_mm: Set(source.margin_top_mm),
        margin_bottom_mm: Set(source.margin_bottom_mm),
        margin_left_mm: Set(source.margin_left_mm),
        margin_right_mm: Set(source.margin_right_mm),
        barcode_default: Set(source.barcode_default),
        default_target_name: Set(source.default_target_name.clone()),
        content: Set(source.content.clone()),
        css_styles: Set(source.css_styles.clone()),
        variables: Set(source.variables.clone()),
        is_default: Set(false),
        status: Set(print_template_status::ACTIVE.to_string()),
        created_by: Set(auth.user_id),
        created_by_name: Set(auth.username.clone()),
        ..Default::default()
    };
    let model = active.insert(&*state.db).await?;

    Ok(axum::Json(ApiResponse::success_with_message(
        to_record(model),
        "打印模板复制成功",
    )))
}
