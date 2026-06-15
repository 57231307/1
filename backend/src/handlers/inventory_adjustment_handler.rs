use crate::middleware::auth_context::AuthContext;
use crate::models::inventory_adjustment_item;
use crate::services::inventory_adjustment_service::{
    AdjustmentItemRequest, CreateAdjustmentRequest, InventoryAdjustmentService,
    UpdateAdjustmentRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 创建调整单请求
#[derive(Debug, Deserialize)]
pub struct CreateAdjustmentRequestPayload {
    pub warehouse_id: i32,
    pub adjustment_date: String,
    pub adjustment_type: String,
    pub reason_type: String,
    pub reason_description: Option<String>,
    pub notes: Option<String>,
    pub items: Vec<AdjustmentItemPayload>,
}

#[derive(Debug, Deserialize)]
pub struct AdjustmentItemPayload {
    pub stock_id: i32,
    pub quantity: String,
    pub unit_cost: Option<String>,
    pub notes: Option<String>,
}

/// 调整单响应
#[derive(Debug, Serialize)]
pub struct AdjustmentResponse {
    pub id: i32,
    pub adjustment_no: String,
    pub warehouse_id: i32,
    pub adjustment_date: DateTime<Utc>,
    pub adjustment_type: String,
    pub reason_type: String,
    pub reason_description: Option<String>,
    pub total_quantity: Decimal,
    pub notes: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub items: Vec<AdjustmentItemResponse>,
}

#[derive(Debug, Serialize)]
pub struct AdjustmentItemResponse {
    pub id: i32,
    pub stock_id: i32,
    pub quantity: Decimal,
    pub quantity_before: Decimal,
    pub quantity_after: Decimal,
    pub unit_cost: Option<Decimal>,
    pub amount: Option<Decimal>,
    pub notes: Option<String>,
}

/// 调整单列表响应
#[derive(Debug, Serialize)]
pub struct AdjustmentListResponse {
    pub adjustments: Vec<AdjustmentSummary>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Serialize)]
pub struct AdjustmentSummary {
    pub id: i32,
    pub adjustment_no: String,
    pub warehouse_id: i32,
    pub adjustment_type: String,
    pub reason_type: String,
    pub status: String,
    pub total_quantity: Decimal,
    pub created_at: DateTime<Utc>,
}

/// 创建调整单
pub async fn create_adjustment(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(payload): Json<CreateAdjustmentRequestPayload>,
) -> Result<Json<ApiResponse<AdjustmentResponse>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());

    // 解析日期
    let adjustment_date: DateTime<Utc> = payload
        .adjustment_date
        .parse::<DateTime<Utc>>()
        .map_err(|e| AppError::validation(format!("日期格式错误：{}", e)))?;

    let mut items = Vec::with_capacity(payload.items.len());
    for item in payload.items {
        let quantity = item
            .quantity
            .parse::<Decimal>()
            .map_err(|e| AppError::validation(format!("数量格式错误：{}", e)))?;

        items.push(AdjustmentItemRequest {
            stock_id: item.stock_id,
            quantity,
            unit_cost: item.unit_cost.and_then(|s| s.parse::<Decimal>().ok()),
            notes: item.notes,
        });
    }

    // 转换请求
    let request = CreateAdjustmentRequest {
        warehouse_id: payload.warehouse_id,
        adjustment_date,
        adjustment_type: payload.adjustment_type,
        reason_type: payload.reason_type,
        reason_description: payload.reason_description,
        notes: payload.notes,
        created_by: Some(auth.user_id),
        items,
    };

    let detail = service
        .create_adjustment(request)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(ApiResponse::success(AdjustmentResponse {
        id: detail.adjustment.id,
        adjustment_no: detail.adjustment.adjustment_no,
        warehouse_id: detail.adjustment.warehouse_id,
        adjustment_date: detail.adjustment.adjustment_date,
        adjustment_type: detail.adjustment.adjustment_type,
        reason_type: detail.adjustment.reason_type,
        reason_description: detail.adjustment.reason_description,
        total_quantity: detail.adjustment.total_quantity,
        notes: detail.adjustment.notes,
        status: detail.adjustment.status,
        created_at: detail.adjustment.created_at,
        items: detail
            .items
            .into_iter()
            .map(|item| AdjustmentItemResponse {
                id: item.id,
                stock_id: item.stock_id,
                quantity: item.quantity,
                quantity_before: item.quantity_before,
                quantity_after: item.quantity_after,
                unit_cost: item.unit_cost,
                amount: item.amount,
                notes: item.notes,
            })
            .collect(),
    })))
}

/// 审核调整单
pub async fn approve_adjustment(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<AdjustmentResponse>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());
    let user_id = auth.user_id;

    service
        .approve_adjustment(id, user_id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;

    let detail = service
        .get_adjustment(id)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(ApiResponse::success(AdjustmentResponse {
        id: detail.adjustment.id,
        adjustment_no: detail.adjustment.adjustment_no,
        warehouse_id: detail.adjustment.warehouse_id,
        adjustment_date: detail.adjustment.adjustment_date,
        adjustment_type: detail.adjustment.adjustment_type,
        reason_type: detail.adjustment.reason_type,
        reason_description: detail.adjustment.reason_description,
        total_quantity: detail.adjustment.total_quantity,
        notes: detail.adjustment.notes,
        status: detail.adjustment.status,
        created_at: detail.adjustment.created_at,
        items: detail
            .items
            .into_iter()
            .map(|item| AdjustmentItemResponse {
                id: item.id,
                stock_id: item.stock_id,
                quantity: item.quantity,
                quantity_before: item.quantity_before,
                quantity_after: item.quantity_after,
                unit_cost: item.unit_cost,
                amount: item.amount,
                notes: item.notes,
            })
            .collect(),
    })))
}

/// 驳回调整单
pub async fn reject_adjustment(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<AdjustmentResponse>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());

    service
        .reject_adjustment(id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;

    let detail = service
        .get_adjustment(id)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    // 发送审批拒绝通知
    if let Some(ref event_service) = state.event_notification_service {
        if let Some(created_by) = detail.adjustment.created_by {
            let _ = event_service
                .notify_approval_result(
                    created_by,
                    &detail.adjustment.adjustment_no,
                    false,
                    &auth.username,
                    None,
                )
                .await;
        }
    }

    Ok(Json(ApiResponse::success(AdjustmentResponse {
        id: detail.adjustment.id,
        adjustment_no: detail.adjustment.adjustment_no,
        warehouse_id: detail.adjustment.warehouse_id,
        adjustment_date: detail.adjustment.adjustment_date,
        adjustment_type: detail.adjustment.adjustment_type,
        reason_type: detail.adjustment.reason_type,
        reason_description: detail.adjustment.reason_description,
        total_quantity: detail.adjustment.total_quantity,
        notes: detail.adjustment.notes,
        status: detail.adjustment.status,
        created_at: detail.adjustment.created_at,
        items: detail
            .items
            .into_iter()
            .map(|item| AdjustmentItemResponse {
                id: item.id,
                stock_id: item.stock_id,
                quantity: item.quantity,
                quantity_before: item.quantity_before,
                quantity_after: item.quantity_after,
                unit_cost: item.unit_cost,
                amount: item.amount,
                notes: item.notes,
            })
            .collect(),
    })))
}

/// 查询调整单列表
pub async fn list_adjustments(
    State(state): State<AppState>,
    Query(params): Query<ListAdjustmentsParams>,
) -> Result<Json<ApiResponse<AdjustmentListResponse>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());

    let (adjustments, total) = service
        .list_adjustments(
            params.page.unwrap_or_default(),
            params.page_size.unwrap_or(20),
        )
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(ApiResponse::success(AdjustmentListResponse {
        adjustments: adjustments
            .into_iter()
            .map(|a| AdjustmentSummary {
                id: a.id,
                adjustment_no: a.adjustment_no,
                warehouse_id: a.warehouse_id,
                adjustment_type: a.adjustment_type,
                reason_type: a.reason_type,
                status: a.status,
                total_quantity: a.total_quantity,
                created_at: a.created_at,
            })
            .collect(),
        total,
        page: params.page.unwrap_or_default(),
        page_size: params.page_size.unwrap_or(20),
    })))
}

#[derive(Debug, Deserialize)]
pub struct ListAdjustmentsParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 查询调整单详情
pub async fn get_adjustment(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<AdjustmentResponse>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());

    let detail = service
        .get_adjustment(id)
        .await
        .map_err(|e| AppError::not_found(e.to_string()))?;

    Ok(Json(ApiResponse::success(AdjustmentResponse {
        id: detail.adjustment.id,
        adjustment_no: detail.adjustment.adjustment_no,
        warehouse_id: detail.adjustment.warehouse_id,
        adjustment_date: detail.adjustment.adjustment_date,
        adjustment_type: detail.adjustment.adjustment_type,
        reason_type: detail.adjustment.reason_type,
        reason_description: detail.adjustment.reason_description,
        total_quantity: detail.adjustment.total_quantity,
        notes: detail.adjustment.notes,
        status: detail.adjustment.status,
        created_at: detail.adjustment.created_at,
        items: detail
            .items
            .into_iter()
            .map(|item| AdjustmentItemResponse {
                id: item.id,
                stock_id: item.stock_id,
                quantity: item.quantity,
                quantity_before: item.quantity_before,
                quantity_after: item.quantity_after,
                unit_cost: item.unit_cost,
                amount: item.amount,
                notes: item.notes,
            })
            .collect(),
    })))
}

/// 更新调整单请求 DTO
#[derive(Debug, Deserialize)]
pub struct UpdateAdjustmentRequestPayload {
    pub warehouse_id: Option<i32>,
    pub adjustment_date: Option<String>,
    pub adjustment_type: Option<String>,
    pub reason_type: Option<String>,
    pub reason_description: Option<String>,
    pub notes: Option<String>,
}

/// 更新调整单
pub async fn update_adjustment(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateAdjustmentRequestPayload>,
) -> Result<Json<ApiResponse<AdjustmentResponse>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());

    let adjustment_date = match payload.adjustment_date {
        Some(s) => Some(
            s.parse::<DateTime<Utc>>()
                .map_err(|e| AppError::validation(format!("日期格式错误：{}", e)))?,
        ),
        None => None,
    };

    let req = UpdateAdjustmentRequest {
        warehouse_id: payload.warehouse_id,
        adjustment_date,
        adjustment_type: payload.adjustment_type,
        reason_type: payload.reason_type,
        reason_description: payload.reason_description,
        notes: payload.notes,
    };

    service
        .update_adjustment(id, req)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;

    let detail = service
        .get_adjustment(id)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(ApiResponse::success(AdjustmentResponse {
        id: detail.adjustment.id,
        adjustment_no: detail.adjustment.adjustment_no,
        warehouse_id: detail.adjustment.warehouse_id,
        adjustment_date: detail.adjustment.adjustment_date,
        adjustment_type: detail.adjustment.adjustment_type,
        reason_type: detail.adjustment.reason_type,
        reason_description: detail.adjustment.reason_description,
        total_quantity: detail.adjustment.total_quantity,
        notes: detail.adjustment.notes,
        status: detail.adjustment.status,
        created_at: detail.adjustment.created_at,
        items: detail
            .items
            .into_iter()
            .map(|item| AdjustmentItemResponse {
                id: item.id,
                stock_id: item.stock_id,
                quantity: item.quantity,
                quantity_before: item.quantity_before,
                quantity_after: item.quantity_after,
                unit_cost: item.unit_cost,
                amount: item.amount,
                notes: item.notes,
            })
            .collect(),
    })))
}

/// 删除调整单
pub async fn delete_adjustment(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());
    service
        .delete_adjustment(id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(())))
}

/// 列出调整单的所有明细项
pub async fn list_items(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<AdjustmentItemResponse>>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());
    let items = service
        .list_items(id)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    Ok(Json(ApiResponse::success(
        items
            .into_iter()
            .map(|item| AdjustmentItemResponse {
                id: item.id,
                stock_id: item.stock_id,
                quantity: item.quantity,
                quantity_before: item.quantity_before,
                quantity_after: item.quantity_after,
                unit_cost: item.unit_cost,
                amount: item.amount,
                notes: item.notes,
            })
            .collect(),
    )))
}

/// 向调整单添加明细
pub async fn add_item(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<AdjustmentItemPayload>,
) -> Result<Json<ApiResponse<AdjustmentItemResponse>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());
    let quantity = payload
        .quantity
        .parse::<Decimal>()
        .map_err(|e| AppError::validation(format!("数量格式错误：{}", e)))?;
    let req = AdjustmentItemRequest {
        stock_id: payload.stock_id,
        quantity,
        unit_cost: payload.unit_cost.and_then(|s| s.parse::<Decimal>().ok()),
        notes: payload.notes,
    };
    let item = service
        .add_item(id, req)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(AdjustmentItemResponse {
        id: item.id,
        stock_id: item.stock_id,
        quantity: item.quantity,
        quantity_before: item.quantity_before,
        quantity_after: item.quantity_after,
        unit_cost: item.unit_cost,
        amount: item.amount,
        notes: item.notes,
    })))
}

/// 更新调整单明细
pub async fn update_item(
    State(state): State<AppState>,
    Path(item_id): Path<i32>,
    Json(payload): Json<AdjustmentItemPayload>,
) -> Result<Json<ApiResponse<AdjustmentItemResponse>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());
    let quantity = payload
        .quantity
        .parse::<Decimal>()
        .map_err(|e| AppError::validation(format!("数量格式错误：{}", e)))?;
    let req = AdjustmentItemRequest {
        stock_id: payload.stock_id,
        quantity,
        unit_cost: payload.unit_cost.and_then(|s| s.parse::<Decimal>().ok()),
        notes: payload.notes,
    };
    let item: inventory_adjustment_item::Model = service
        .update_item(item_id, req)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(AdjustmentItemResponse {
        id: item.id,
        stock_id: item.stock_id,
        quantity: item.quantity,
        quantity_before: item.quantity_before,
        quantity_after: item.quantity_after,
        unit_cost: item.unit_cost,
        amount: item.amount,
        notes: item.notes,
    })))
}

/// 删除调整单明细
pub async fn delete_item(
    State(state): State<AppState>,
    Path(item_id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());
    service
        .delete_item(item_id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(ApiResponse::success(())))
}

/// 生成库存调整单号
/// GET /api/v1/erp/inventory/adjustments/generate-no
///
/// 单据号格式：`IA{yyyyMMdd}{4 位流水}`，例如 `IA202605140001`。
/// 数据库列 `inventory_adjustments.adjustment_no` 上的 `UNIQUE` 约束负责最终去重。
pub async fn generate_no(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let adjustment_no = DocumentNumberGenerator::generate_no_with_width(
        &*state.db,
        "IA",
        inventory_adjustment::Entity,
        inventory_adjustment::Column::AdjustmentNo,
        4,
    )
    .await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "adjustment_no": adjustment_no
    }))))
}
