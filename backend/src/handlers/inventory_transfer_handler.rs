use crate::utils::app_state::AppState;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::models::dto::PageRequest;
use crate::services::inventory_transfer_service::{
    CreateInventoryTransferRequest, InventoryTransferItemRequest, InventoryTransferService,
    UpdateInventoryTransferRequest,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct InventoryTransferQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub from_warehouse_id: Option<i32>,
    pub to_warehouse_id: Option<i32>,
    pub transfer_no: Option<String>,
}

/// 审核库存调拨请求
#[derive(Debug, Deserialize)]
pub struct ApproveTransferRequest {
    pub approved: bool,
    pub notes: Option<String>,
}

/// 获取库存调拨列表
pub async fn list_transfers(
    State(state): State<AppState>,
    Query(query): Query<InventoryTransferQuery>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let transfer_service = InventoryTransferService::new(state.db.clone());

    let page_req = PageRequest {
        page: query.page.unwrap_or(1),
        page_size: query.page_size.unwrap_or(10),
    };

    let transfers = transfer_service
        .list_transfers(
            page_req,
            query.status,
            query.from_warehouse_id,
            query.to_warehouse_id,
            query.transfer_no,
        )
        .await?;

    let transfers_json: Vec<serde_json::Value> = transfers
        .items
        .into_iter()
        .map(|t| {
            serde_json::to_value(t).map_err(|e| AppError::internal(format!("序列化失败: {}", e)))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(ApiResponse::success(transfers_json)))
}

/// 获取库存调拨详情
pub async fn get_transfer(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let transfer_service = InventoryTransferService::new(state.db.clone());
    let transfer = transfer_service.get_transfer_detail(id).await?;
    let transfer_json = serde_json::to_value(transfer)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success(transfer_json)))
}

/// 创建库存调拨
pub async fn create_transfer(
    State(state): State<AppState>,
    Json(request): Json<CreateInventoryTransferRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let transfer_service = InventoryTransferService::new(state.db.clone());
    let transfer = transfer_service.create_transfer(request).await?;
    let transfer_json = serde_json::to_value(transfer)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        transfer_json,
        "库存调拨单创建成功",
    )))
}

/// 更新库存调拨
pub async fn update_transfer(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(request): Json<UpdateInventoryTransferRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let transfer_service = InventoryTransferService::new(state.db.clone());
    let transfer = transfer_service.update_transfer(id, request).await?;
    let transfer_json = serde_json::to_value(transfer)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        transfer_json,
        "库存调拨单更新成功",
    )))
}

/// 审核库存调拨
pub async fn approve_transfer(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(request): Json<ApproveTransferRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let transfer_service = InventoryTransferService::new(state.db.clone());
    let transfer = transfer_service
        .approve_transfer(id, request.approved, request.notes)
        .await?;
    let transfer_json = serde_json::to_value(transfer)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    let message = if request.approved {
        "库存调拨单已审核"
    } else {
        "库存调拨单已驳回"
    };
    Ok(Json(ApiResponse::success_with_message(
        transfer_json,
        message,
    )))
}

/// 发出库存调拨
pub async fn ship_transfer(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let transfer_service = InventoryTransferService::new(state.db.clone());
    let transfer = transfer_service.ship_transfer(id).await?;
    let transfer_json = serde_json::to_value(transfer)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        transfer_json,
        "库存调拨单已发出",
    )))
}

/// 接收库存调拨
pub async fn receive_transfer(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let transfer_service = InventoryTransferService::new(state.db.clone());
    let transfer = transfer_service.receive_transfer(id).await?;
    let transfer_json = serde_json::to_value(transfer)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        transfer_json,
        "库存调拨单已接收",
    )))
}

/// 删除库存调拨
pub async fn delete_transfer(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let transfer_service = InventoryTransferService::new(state.db.clone());
    transfer_service
        .delete_transfer(id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success_with_message((), "库存调拨单已删除")))
}

/// 列出调拨单的所有明细项
pub async fn list_items(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let transfer_service = InventoryTransferService::new(state.db.clone());
    let items = transfer_service
        .list_items(id)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    let items_json: Vec<serde_json::Value> = items
        .into_iter()
        .map(|item| {
            serde_json::to_value(item).map_err(|e| AppError::internal(format!("序列化失败: {}", e)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(ApiResponse::success(items_json)))
}

/// 向调拨单添加明细
pub async fn add_item(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(request): Json<InventoryTransferItemRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let transfer_service = InventoryTransferService::new(state.db.clone());
    let item = transfer_service
        .add_item(id, request)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    let item_json = serde_json::to_value(item)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        item_json,
        "调拨明细添加成功",
    )))
}

/// 更新调拨单明细
pub async fn update_item(
    State(state): State<AppState>,
    Path(item_id): Path<i32>,
    Json(request): Json<InventoryTransferItemRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let transfer_service = InventoryTransferService::new(state.db.clone());
    let item = transfer_service
        .update_item(item_id, request)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    let item_json = serde_json::to_value(item)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        item_json,
        "调拨明细更新成功",
    )))
}

/// 删除调拨单明细
pub async fn delete_item(
    State(state): State<AppState>,
    Path(item_id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let transfer_service = InventoryTransferService::new(state.db.clone());
    transfer_service
        .delete_item(item_id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success_with_message((), "调拨明细已删除")))
}
