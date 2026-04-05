use axum::{
    extract::{Path, State},
    Json,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use crate::utils::app_state::AppState;
use crate::services::purchase_return_service::{CreateReturnItemRequest, PurchaseReturnService, UpdateReturnItemRequest};

pub async fn list_items(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<crate::services::purchase_return_service::PurchaseReturnItemDto>>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    let items = service.list_items(id).await?;
    Ok(Json(ApiResponse::success(items)))
}

pub async fn create_item(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    Json(req): Json<CreateReturnItemRequest>,
) -> Result<Json<ApiResponse<crate::models::purchase_return_item::Model>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    let item = service.create_item(id, req).await?;
    Ok(Json(ApiResponse::success(item)))
}

pub async fn update_item(
    Path(item_id): Path<i32>,
    State(state): State<AppState>,
    Json(req): Json<UpdateReturnItemRequest>,
) -> Result<Json<ApiResponse<crate::models::purchase_return_item::Model>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    let item = service.update_item(item_id, req).await?;
    Ok(Json(ApiResponse::success(item)))
}

pub async fn delete_item(
    Path(item_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    service.delete_item(item_id).await?;
    Ok(Json(ApiResponse::success(())))
}
