use crate::utils::app_state::AppState;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::models::dto::PageRequest;
use crate::models::inventory_count;
use crate::services::inventory_count_service::{
    CreateInventoryCountRequest, InventoryCountDetail, InventoryCountItemDetail,
    InventoryCountItemRequest, InventoryCountService, UpdateInventoryCountRequest,
};
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use crate::utils::response::ApiResponse;
use crate::utils::response::PaginatedResponse;

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct InventoryCountQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub warehouse_id: Option<i32>,
    pub count_no: Option<String>,
}

/// 审核盘点请求
#[derive(Debug, Deserialize)]
pub struct ApproveCountRequest {
    pub approved: bool,
    pub notes: Option<String>,
}

/// 获取库存盘点列表
/// GET /api/v1/erp/inventory/counts
pub async fn list_counts(
    State(state): State<AppState>,
    Query(query): Query<InventoryCountQuery>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<PaginatedResponse<InventoryCountDetail>>>, AppError> {
    let count_service = InventoryCountService::new(state.db.clone());

    let page_req = PageRequest {
        page: query.page.unwrap_or(1),
        page_size: query.page_size.unwrap_or(10),
    };

    let counts = count_service
        .list_counts(page_req, query.status, query.warehouse_id, query.count_no)
        .await?;

    let paginated: PaginatedResponse<_> =
        PaginatedResponse::new(counts.items, counts.total, counts.page, counts.page_size);
    Ok(Json(ApiResponse::success(paginated)))
}

/// 获取库存盘点详情
/// GET /api/v1/erp/inventory/counts/:id
pub async fn get_count(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<InventoryCountDetail>>, AppError> {
    let count_service = InventoryCountService::new(state.db.clone());
    let count = count_service.get_count_detail(id).await?;
    Ok(Json(ApiResponse::success(count)))
}

/// 创建库存盘点
/// POST /api/v1/erp/inventory/counts
pub async fn create_count(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(request): Json<CreateInventoryCountRequest>,
) -> Result<Json<ApiResponse<InventoryCountDetail>>, AppError> {
    let count_service = InventoryCountService::new(state.db.clone());
    let count = count_service.create_count(request).await?;
    Ok(Json(ApiResponse::success_with_message(
        count,
        "库存盘点单创建成功",
    )))
}

/// 更新库存盘点
/// PUT /api/v1/erp/inventory/counts/:id
pub async fn update_count(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
    Json(request): Json<UpdateInventoryCountRequest>,
) -> Result<Json<ApiResponse<InventoryCountDetail>>, AppError> {
    let count_service = InventoryCountService::new(state.db.clone());
    let count = count_service.update_count(id, request).await?;
    Ok(Json(ApiResponse::success_with_message(
        count,
        "库存盘点单更新成功",
    )))
}

/// 审核库存盘点
/// POST /api/v1/erp/inventory/counts/:id/approve
pub async fn approve_count(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
    Json(request): Json<ApproveCountRequest>,
) -> Result<Json<ApiResponse<InventoryCountDetail>>, AppError> {
    let count_service = InventoryCountService::new(state.db.clone());
    let count = count_service
        .approve_count(id, request.approved, request.notes)
        .await?;
    let message = if request.approved {
        "库存盘点单已审核"
    } else {
        "库存盘点单已驳回"
    };
    Ok(Json(ApiResponse::success_with_message(count, message)))
}

/// 完成库存盘点并调整库存
/// POST /api/v1/erp/inventory/counts/:id/complete
pub async fn complete_count(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<InventoryCountDetail>>, AppError> {
    let count_service = InventoryCountService::new(state.db.clone());
    let count = count_service.complete_count(id).await?;
    Ok(Json(ApiResponse::success_with_message(
        count,
        "库存盘点已完成，库存已调整",
    )))
}

/// 删除库存盘点
/// DELETE /api/v1/erp/inventory/counts/:id
pub async fn delete_count(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let count_service = InventoryCountService::new(state.db.clone());
    count_service.delete_count(id).await?;
    Ok(Json(ApiResponse::success_with_message(
        (),
        "库存盘点单已删除",
    )))
}

/// 列出盘点单的所有明细项
/// GET /api/v1/erp/inventory/counts/:id/items
pub async fn list_items(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<InventoryCountItemDetail>>>, AppError> {
    let count_service = InventoryCountService::new(state.db.clone());
    let items = count_service.list_items(id).await?;
    Ok(Json(ApiResponse::success(items)))
}

/// 向盘点单添加明细
/// POST /api/v1/erp/inventory/counts/:id/items
pub async fn add_item(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
    Json(request): Json<InventoryCountItemRequest>,
) -> Result<Json<ApiResponse<InventoryCountItemDetail>>, AppError> {
    let count_service = InventoryCountService::new(state.db.clone());
    let item = count_service.add_item(id, request).await?;
    Ok(Json(ApiResponse::success_with_message(
        item,
        "盘点明细添加成功",
    )))
}

/// 更新盘点单明细
/// PUT /api/v1/erp/inventory/counts/items/:item_id
pub async fn update_item(
    State(state): State<AppState>,
    Path(item_id): Path<i32>,
    _auth: AuthContext,
    Json(request): Json<InventoryCountItemRequest>,
) -> Result<Json<ApiResponse<InventoryCountItemDetail>>, AppError> {
    let count_service = InventoryCountService::new(state.db.clone());
    let item = count_service.update_item(item_id, request).await?;
    Ok(Json(ApiResponse::success_with_message(
        item,
        "盘点明细更新成功",
    )))
}

/// 删除盘点单明细
/// DELETE /api/v1/erp/inventory/counts/items/:item_id
pub async fn delete_item(
    State(state): State<AppState>,
    Path(item_id): Path<i32>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let count_service = InventoryCountService::new(state.db.clone());
    count_service.delete_item(item_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        (),
        "盘点明细已删除",
    )))
}

/// 生成盘点单号
/// GET /api/v1/erp/inventory/counts/generate-no
///
/// 单据号格式：`IC{yyyyMMdd}{4 位流水}`，例如 `IC202605140001`。
/// 流水部分通过 `DocumentNumberGenerator` 统计当日同前缀已存在的单据数量 + 1 计算得到。
/// 注意：此方法为无锁"读后写"，业务侧应在事务内创建单据时校验唯一性以避免并发冲突。
pub async fn generate_no(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let count_no = DocumentNumberGenerator::generate_no_with_width(
        &*state.db,
        "IC",
        inventory_count::Entity,
        inventory_count::Column::CountNo,
        4,
    )
    .await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "count_no": count_no
    }))))
}
