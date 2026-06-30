//! 色卡 CRUD Handler
//!
//! 任务编号: P14 批 2 I-3 第 9 批
//! 拆分原 handlers/color_card_handler.rs 的 5 个 CRUD 端点
//! 行为完全保持一致（仅结构重构）

use axum::{
    extract::{Path, Query, State},
    Json,
};

use crate::middleware::auth_context::AuthContext;
use crate::models::color_card_borrow_dto::ListColorCardsQuery;
use crate::models::color_card_create_dto::{ArchiveColorCardDto, CreateColorCardDto, UpdateColorCardDto};
use crate::models::color_card_response_dto::{ColorCardDetail, ColorCardListItem, ColorItemInfo, PagedResponse};
use crate::services::color_card_crud_service::ColorCardCrudService;
use crate::services::color_card_item_service::ColorCardItemService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

use super::error_map::crud_err;
use super::helpers::item_to_info;

/// GET /api/v1/erp/color-cards - 色卡列表
pub async fn list_color_cards(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ListColorCardsQuery>,
) -> Result<Json<ApiResponse<PagedResponse<ColorCardListItem>>>, AppError> {
    let service = ColorCardCrudService::from_state(&state);
    let page = query.page.unwrap_or(1);
    // v11 批次 36 修复：page_size clamp 防止 DoS
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let (items, total) = service
        .list(
            page,
            page_size,
            query.card_type,
            query.season,
            query.status,
            query.keyword,
        )
        .await
        .map_err(crud_err)?;

    let list: Vec<ColorCardListItem> = items
        .into_iter()
        .map(|m| ColorCardListItem {
            id: m.id,
            card_no: m.card_no,
            card_name: m.card_name,
            card_type: m.card_type,
            season: m.season,
            brand: m.brand,
            total_colors: m.total_colors,
            status: m.status,
            cover_image_url: m.cover_image_url,
            created_at: m.created_at,
        })
        .collect();

    Ok(Json(ApiResponse::success(PagedResponse {
        items: list,
        total,
        page,
        page_size,
    })))
}

/// POST /api/v1/erp/color-cards - 创建色卡
pub async fn create_color_card(
    _auth: AuthContext,
    State(state): State<AppState>,
    Json(dto): Json<CreateColorCardDto>,
) -> Result<Json<ApiResponse<ColorCardListItem>>, AppError> {
    let service = ColorCardCrudService::from_state(&state);

    let created = service.create(dto).await.map_err(crud_err)?;

    Ok(Json(ApiResponse::success(ColorCardListItem {
        id: created.id,
        card_no: created.card_no,
        card_name: created.card_name,
        card_type: created.card_type,
        season: created.season,
        brand: created.brand,
        total_colors: created.total_colors,
        status: created.status,
        cover_image_url: created.cover_image_url,
        created_at: created.created_at,
    })))
}

/// GET /api/v1/erp/color-cards/:id - 色卡详情（含色号列表）
pub async fn get_color_card(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<ColorCardDetail>>, AppError> {
    let crud_svc = ColorCardCrudService::from_state(&state);
    let item_svc = ColorCardItemService::from_state(&state);

    let card = crud_svc.get_by_id(id).await.map_err(crud_err)?;
    let (items, _) = item_svc
        .list(id, 1, 1000)
        .await
        .map_err(super::error_map::item_err)?;

    let item_infos: Vec<ColorItemInfo> = items
        .into_iter()
        .map(item_to_info)
        .collect();

    Ok(Json(ApiResponse::success(ColorCardDetail {
        id: card.id,
        card_no: card.card_no,
        card_name: card.card_name,
        card_type: card.card_type,
        season: card.season,
        brand: card.brand,
        total_colors: card.total_colors,
        status: card.status,
        description: card.description,
        cover_image_url: card.cover_image_url,
        items: item_infos,
        created_at: card.created_at,
        updated_at: card.updated_at,
    })))
}

/// PUT /api/v1/erp/color-cards/:id - 更新色卡
pub async fn update_color_card(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<UpdateColorCardDto>,
) -> Result<Json<ApiResponse<ColorCardListItem>>, AppError> {
    let service = ColorCardCrudService::from_state(&state);

    let updated = service
        .update(id, dto)
        .await
        .map_err(crud_err)?;

    Ok(Json(ApiResponse::success(ColorCardListItem {
        id: updated.id,
        card_no: updated.card_no,
        card_name: updated.card_name,
        card_type: updated.card_type,
        season: updated.season,
        brand: updated.brand,
        total_colors: updated.total_colors,
        status: updated.status,
        cover_image_url: updated.cover_image_url,
        created_at: updated.created_at,
    })))
}

/// DELETE /api/v1/erp/color-cards/:id - 归档色卡
pub async fn archive_color_card(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<ArchiveColorCardDto>,
) -> Result<Json<ApiResponse<ColorCardListItem>>, AppError> {
    let service = ColorCardCrudService::from_state(&state);

    let updated = service
        .archive(id, dto)
        .await
        .map_err(crud_err)?;

    Ok(Json(ApiResponse::success(ColorCardListItem {
        id: updated.id,
        card_no: updated.card_no,
        card_name: updated.card_name,
        card_type: updated.card_type,
        season: updated.season,
        brand: updated.brand,
        total_colors: updated.total_colors,
        status: updated.status,
        cover_image_url: updated.cover_image_url,
        created_at: updated.created_at,
    })))
}
