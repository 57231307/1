//! 色号 Handler
//!
//! 任务编号: P14 批 2 I-3 第 9 批
//! 拆分原 handlers/color_card_handler.rs 的 5 个色号端点（list/create/update/delete/batch_import）
//! 行为完全保持一致（仅结构重构）

use axum::{
    extract::{Path, Query, State},
    Json,
};

use crate::middleware::auth_context::AuthContext;
use crate::middleware::tenant::extract_tenant_id;
use crate::models::color_card_item_dto::{BatchImportItemsDto, ColorItemDto};
use crate::models::color_card_response_dto::{ColorItemInfo, PagedResponse};
use crate::services::color_card_item_service::ColorCardItemService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

use super::error_map::item_err;
use super::helpers::{item_to_info, ListItemsQuery};

/// GET /api/v1/erp/color-cards/:id/items - 色号列表
pub async fn list_color_items(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(query): Query<ListItemsQuery>,
) -> Result<Json<ApiResponse<PagedResponse<ColorItemInfo>>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorCardItemService::from_state(&state);
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(100);

    let (items, total) = service
        .list(id, tenant_id, page, page_size)
        .await
        .map_err(item_err)?;

    let infos: Vec<ColorItemInfo> = items.into_iter().map(item_to_info).collect();

    Ok(Json(ApiResponse::success(PagedResponse {
        items: infos,
        total,
        page,
        page_size,
    })))
}

/// POST /api/v1/erp/color-cards/:id/items - 新增色号
pub async fn create_color_item(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<ColorItemDto>,
) -> Result<Json<ApiResponse<ColorItemInfo>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorCardItemService::from_state(&state);

    let created = service.create(id, tenant_id, dto).await.map_err(item_err)?;
    Ok(Json(ApiResponse::success(item_to_info(created))))
}

/// PUT /api/v1/erp/color-cards/:id/items/:item_id - 更新色号
pub async fn update_color_item(
    auth: AuthContext,
    State(state): State<AppState>,
    Path((id, item_id)): Path<(i64, i64)>,
    Json(dto): Json<ColorItemDto>,
) -> Result<Json<ApiResponse<ColorItemInfo>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorCardItemService::from_state(&state);

    let updated = service
        .update(id, item_id, tenant_id, dto)
        .await
        .map_err(item_err)?;
    Ok(Json(ApiResponse::success(item_to_info(updated))))
}

/// DELETE /api/v1/erp/color-cards/:id/items/:item_id - 删除色号
pub async fn delete_color_item(
    auth: AuthContext,
    State(state): State<AppState>,
    Path((id, item_id)): Path<(i64, i64)>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorCardItemService::from_state(&state);

    service
        .delete(id, item_id, tenant_id)
        .await
        .map_err(item_err)?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /api/v1/erp/color-cards/:id/items/batch - 批量导入色号
pub async fn batch_import_items(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<BatchImportItemsDto>,
) -> Result<Json<ApiResponse<crate::models::color_card_item_dto::BatchImportResponse>>, AppError>
{
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorCardItemService::from_state(&state);

    let result = service
        .batch_import(id, tenant_id, dto.items)
        .await
        .map_err(item_err)?;
    Ok(Json(ApiResponse::success(result)))
}
