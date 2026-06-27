//! 色卡扫码 / 导出 Handler
//!
//! 任务编号: P14 批 2 I-3 第 9 批
//! 拆分原 handlers/color_card_handler.rs 的 2 个端点（scan + export）
//! 行为完全保持一致（仅结构重构）

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::middleware::auth_context::AuthContext;
use crate::middleware::tenant::extract_tenant_id;
use crate::models::color_card_response_dto::ScanResult;
use crate::services::color_card_crud_service::ColorCardCrudService;
use crate::services::color_card_item_service::ColorCardItemService;
use crate::services::color_card_scan_service::ColorCardScanService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

use super::error_map::{crud_err, item_err};

/// GET /api/v1/erp/color-cards/scan/:code - 扫码查询色号详情
pub async fn scan_color_code(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<ScanResult>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorCardScanService::from_state(&state);

    let result = service.scan_by_code(&code, tenant_id).await?;
    Ok(Json(ApiResponse::success(result)))
}

/// GET /api/v1/erp/color-cards/export/:id - 导出色卡为 CSV
pub async fn export_color_card(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<axum::response::Response, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let item_svc = ColorCardItemService::from_state(&state);
    let crud_svc = ColorCardCrudService::from_state(&state);

    let card = crud_svc.get_by_id(id, tenant_id).await.map_err(crud_err)?;
    let (items, _) = item_svc
        .list(id, tenant_id, 1, 10000)
        .await
        .map_err(item_err)?;

    // 构造 CSV
    let mut csv = String::from("color_code,color_name,rgb_r,rgb_g,rgb_b,hex_value,cmyk_c,cmyk_m,cmyk_y,cmyk_k,lab_l,lab_a,lab_b,pantone_code,cncs_code,custom_code\n");
    for item in items {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            item.color_code,
            super::helpers::csv_escape(&item.color_name),
            item.rgb_r,
            item.rgb_g,
            item.rgb_b,
            item.hex_value,
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
        ));
    }

    let filename = format!("color-card-{}.csv", card.card_no.replace(['/', '\\', ' '], "_"));
    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            axum::http::header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(axum::body::Body::from(csv))
        .map_err(|e| AppError::internal(format!("导出响应构建失败: {e}")))
}
