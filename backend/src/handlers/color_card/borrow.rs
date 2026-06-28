//! 色卡借出/归还/遗失/损坏 Handler
//!
//! 任务编号: P14 批 2 I-3 第 9 批
//! 拆分原 handlers/color_card_handler.rs 的 5 个借出流程端点
//! 行为完全保持一致（仅结构重构）

use axum::{
    extract::{Path, Query, State},
    Json,
};

use crate::middleware::auth_context::AuthContext;
use crate::models::color_card_borrow_dto::{
    BorrowColorCardDto, ListBorrowRecordsQuery, MarkDamagedColorCardDto, MarkLostColorCardDto,
    ReturnColorCardDto,
};
use crate::models::color_card_response_dto::{BorrowRecordInfo, PagedResponse};
use crate::services::color_card_borrow_service::ColorCardBorrowService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

use super::error_map::borrow_err;
use super::helpers::record_to_info;

/// POST /api/v1/erp/color-cards/borrow - 借出色卡
pub async fn borrow_color_card(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(dto): Json<BorrowColorCardDto>,
) -> Result<Json<ApiResponse<BorrowRecordInfo>>, AppError> {
    let user_id = auth.user_id as i64;
    let service = ColorCardBorrowService::from_state(&state);

    let borrowed_by = dto.borrowed_by.unwrap_or(user_id);

    let record = service
        .borrow(
            dto.color_card_id,
            dto.customer_id,
            borrowed_by,
            dto.expected_return_at,
            dto.purpose,
            dto.notes,
        )
        .await
        .map_err(borrow_err)?;

    Ok(Json(ApiResponse::success(record_to_info(record))))
}

/// POST /api/v1/erp/color-cards/return/:record_id - 归还色卡
pub async fn return_color_card(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(record_id): Path<i64>,
    Json(dto): Json<ReturnColorCardDto>,
) -> Result<Json<ApiResponse<BorrowRecordInfo>>, AppError> {
    let service = ColorCardBorrowService::from_state(&state);

    let record = service
        .return_card(record_id, dto.actual_return_at, dto.notes)
        .await
        .map_err(borrow_err)?;

    Ok(Json(ApiResponse::success(record_to_info(record))))
}

/// POST /api/v1/erp/color-cards/lost/:record_id - 登记遗失
pub async fn mark_lost_color_card(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(record_id): Path<i64>,
    Json(dto): Json<MarkLostColorCardDto>,
) -> Result<Json<ApiResponse<BorrowRecordInfo>>, AppError> {
    let service = ColorCardBorrowService::from_state(&state);

    let record = service
        .mark_lost(record_id, dto.compensation_amount, dto.notes)
        .await
        .map_err(borrow_err)?;

    Ok(Json(ApiResponse::success(record_to_info(record))))
}

/// POST /api/v1/erp/color-cards/damaged/:record_id - 标记损坏
pub async fn mark_damaged_color_card(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(record_id): Path<i64>,
    Json(dto): Json<MarkDamagedColorCardDto>,
) -> Result<Json<ApiResponse<BorrowRecordInfo>>, AppError> {
    let service = ColorCardBorrowService::from_state(&state);

    let record = service
        .mark_damaged(record_id, dto.compensation_amount, dto.notes)
        .await
        .map_err(borrow_err)?;

    Ok(Json(ApiResponse::success(record_to_info(record))))
}

/// GET /api/v1/erp/color-cards/borrow-records - 借出历史
pub async fn list_borrow_records(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ListBorrowRecordsQuery>,
) -> Result<Json<ApiResponse<PagedResponse<BorrowRecordInfo>>>, AppError> {
    let service = ColorCardBorrowService::from_state(&state);
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let (items, total) = service
        .list_records(query)
        .await
        .map_err(borrow_err)?;

    let infos: Vec<BorrowRecordInfo> = items.into_iter().map(record_to_info).collect();
    Ok(Json(ApiResponse::success(PagedResponse {
        items: infos,
        total,
        page,
        page_size,
    })))
}
