//! 色卡扫码 / 导出 Handler
//!
//! 任务编号: P14 批 2 I-3 第 9 批
//! 拆分原 handlers/color_card_handler.rs 的 2 个端点（scan + export）
//! 行为完全保持一致（仅结构重构）

use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::middleware::auth_context::AuthContext;
use crate::models::audit_log::{OperationType, Severity};
use crate::models::color_card_response_dto::ScanResult;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::color_card_crud_service::ColorCardCrudService;
use crate::services::color_card_item_service::ColorCardItemService;
use crate::services::color_card_scan_service::ColorCardScanService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use crate::utils::xlsx_export::{build_xlsx_response, XlsxTable};

use super::error_map::{crud_err, item_err};

/// GET /api/v1/erp/color-cards/scan/:code - 扫码查询色号详情
pub async fn scan_color_code(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<ScanResult>>, AppError> {
    let service = ColorCardScanService::from_state(&state);

    let result = service.scan_by_code(&code).await?;
    Ok(Json(ApiResponse::success(result)))
}

/// GET /api/v1/erp/color-cards/scan-by-id/:id - 按色号 ID 查询色号详情
/// v11 P1-5 真实实现：接入 ColorCardScanService::scan_by_id，提供按 ID 查询端点
pub async fn scan_color_by_id(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<ScanResult>>, AppError> {
    let service = ColorCardScanService::from_state(&state);
    let result = service.scan_by_id(id).await?;
    Ok(Json(ApiResponse::success(result)))
}

/// GET /api/v1/erp/color-cards/export/:id - 导出色卡为 xlsx
pub async fn export_color_card(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<axum::response::Response, AppError> {
    let item_svc = ColorCardItemService::from_state(&state);
    let crud_svc = ColorCardCrudService::from_state(&state);

    let card = crud_svc.get_by_id(id).await.map_err(crud_err)?;
    let (items, _) = item_svc
        .list(id, 1, 10000)
        .await
        .map_err(item_err)?;
    let row_count = items.len();

    // 规则 3：构造 xlsx 表格（字段名与原 CSV 保持一致）
    let headers = vec![
        "color_code".to_string(),
        "color_name".to_string(),
        "rgb_r".to_string(),
        "rgb_g".to_string(),
        "rgb_b".to_string(),
        "hex_value".to_string(),
        "cmyk_c".to_string(),
        "cmyk_m".to_string(),
        "cmyk_y".to_string(),
        "cmyk_k".to_string(),
        "lab_l".to_string(),
        "lab_a".to_string(),
        "lab_b".to_string(),
        "pantone_code".to_string(),
        "cncs_code".to_string(),
        "custom_code".to_string(),
    ];
    let rows: Vec<Vec<String>> = items
        .iter()
        .map(|item| {
            vec![
                item.color_code.clone(),
                item.color_name.clone(),
                item.rgb_r.to_string(),
                item.rgb_g.to_string(),
                item.rgb_b.to_string(),
                item.hex_value.clone(),
                item.cmyk_c.map(|d| d.to_string()).unwrap_or_default(),
                item.cmyk_m.map(|d| d.to_string()).unwrap_or_default(),
                item.cmyk_y.map(|d| d.to_string()).unwrap_or_default(),
                item.cmyk_k.map(|d| d.to_string()).unwrap_or_default(),
                item.lab_l.map(|d| d.to_string()).unwrap_or_default(),
                item.lab_a.map(|d| d.to_string()).unwrap_or_default(),
                item.lab_b.map(|d| d.to_string()).unwrap_or_default(),
                item.pantone_code.clone().unwrap_or_default(),
                item.cncs_code.clone().unwrap_or_default(),
                item.custom_code.clone().unwrap_or_default(),
            ]
        })
        .collect();
    let table = XlsxTable {
        sheet_name: "色卡".to_string(),
        headers,
        rows,
    };

    let filename = format!("color-card-{}", card.card_no.replace(['/', '\\', ' '], "_"));

    // V15 P1-1-4：色卡导出审计日志写入（best-effort，异步不阻塞响应）
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("color_card".to_string()),
        resource_id: Some(id.to_string()),
        resource_name: Some(format!("{}.xlsx", filename)),
        description: Some(format!(
            "用户 {} 导出色卡 #{}（共 {} 条色号）",
            auth.username, id, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some(format!("/api/v1/erp/color-cards/export/{}", id)),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "color_card_id": id,
            "card_no": card.card_no,
            "total": row_count,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);

    build_xlsx_response(&table, &filename)
}
