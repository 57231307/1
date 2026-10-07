use axum::{
    Json,
    extract::{Path, Query, State},
};
use sea_orm::{ConnectionTrait, Statement};
use serde::Deserialize;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::dto::PageRequest;
use crate::models::inventory_transfer;
use crate::services::inv::{
    CreateInventoryTransferRequest, InventoryTransferItemRequest, InventoryTransferService,
    UpdateInventoryTransferItemRequest, UpdateInventoryTransferRequest,
};
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use crate::utils::response::{ApiResponse, PaginatedResponse};

/// 查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
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
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ApproveTransferRequest {
    pub approved: bool,
    pub notes: Option<String>,
}

/// 获取库存调拨列表
pub async fn list_transfers(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<InventoryTransferQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<serde_json::Value>>>, AppError> {
    let transfer_service = InventoryTransferService::new(state.db.clone());

    let page_req = PageRequest {
        page: query.page.unwrap_or(1).clamp(1, 1000), // 批次 95 P3-3~8：分页 clamp 防 DoS
        page_size: query.page_size.unwrap_or(10).clamp(1, 100),
    };
    // PageRequest 不是 Copy，list_transfers 会拿走它 ⇒ 之后再读 page_req 是 use-after-move，
    // 故先把分页两值取出，用于回传给前端分页条。
    let (page, page_size) = (page_req.page, page_req.page_size);
    // V15 P0-S01：提取行级数据权限上下文
    let data_scope_ctx = auth.to_data_scope_context();

    let transfers = transfer_service
        .list_transfers(
            page_req,
            query.status,
            query.from_warehouse_id,
            query.to_warehouse_id,
            query.transfer_no,
            Some(&data_scope_ctx),
        )
        .await?;

    let transfers_json: Vec<serde_json::Value> = transfers
        .items
        .into_iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()?;

    // 透传 service 已算出的总数，分页条据此渲染（原实现丢弃 total 致前端分页失效）
    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        transfers_json,
        transfers.total,
        page,
        page_size,
    ))))
}

/// 获取库存调拨详情
pub async fn get_transfer(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let transfer_service = InventoryTransferService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();
    let transfer = transfer_service
        .get_transfer_detail(id, Some(&data_scope_ctx))
        .await?;
    let transfer_json = serde_json::to_value(transfer)?;
    Ok(Json(ApiResponse::success(transfer_json)))
}

/// 创建库存调拨
/// batch-15 P3: 添加调拨频率限制（每小时最多 10 次）
pub async fn create_transfer(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(request): Json<CreateInventoryTransferRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // batch-15 P3: 检查调拨频率限制
    let one_hour_ago = chrono::Utc::now() - chrono::Duration::hours(1);
    let recent_count_sql = format!(
        "SELECT COUNT(*) as count FROM inventory_transfers WHERE created_by = {} AND created_at >= '{}'",
        auth.user_id,
        one_hour_ago.format("%Y-%m-%d %H:%M:%S")
    );
    let count_result = state
        .db
        .as_ref()
        .query_one_raw(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            recent_count_sql,
        ))
        .await?;
    let recent_count = count_result
        .map(|r| r.try_get::<i64>("", "count").unwrap_or(0))
        .unwrap_or(0);

    if recent_count >= 10 {
        return Err(AppError::business(format!(
            "调拨频率过高：过去 1 小时内已创建 {} 次调拨，最多允许 10 次",
            recent_count
        )));
    }

    let transfer_service = InventoryTransferService::new(state.db.clone());
    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    let transfer = transfer_service
        .create_transfer(request, auth.user_id)
        .await?;
    let transfer_json = serde_json::to_value(transfer)?;
    Ok(Json(ApiResponse::success_with_message(
        transfer_json,
        "库存调拨单创建成功",
    )))
}

/// 更新库存调拨
pub async fn update_transfer(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(request): Json<UpdateInventoryTransferRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let transfer_service = InventoryTransferService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——更新前先校验资源归属（复用 P0-S01 的 get_transfer_detail + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    transfer_service
        .get_transfer_detail(id, Some(&data_scope_ctx))
        .await?;

    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    let transfer = transfer_service
        .update_transfer(id, request, auth.user_id)
        .await?;
    let transfer_json = serde_json::to_value(transfer)?;
    Ok(Json(ApiResponse::success_with_message(
        transfer_json,
        "库存调拨单更新成功",
    )))
}

/// 审核库存调拨
pub async fn approve_transfer(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(request): Json<ApproveTransferRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let transfer_service = InventoryTransferService::new(state.db.clone());
    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    // P1 batch-18 缺陷 6.1：注入 role_id 用于分级审批权限校验
    let transfer = transfer_service
        .approve_transfer(
            id,
            request.approved,
            request.notes,
            auth.user_id,
            auth.role_id,
        )
        .await?;
    let transfer_json = serde_json::to_value(transfer)?;
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
    let transfer_json = serde_json::to_value(transfer)?;
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
    let transfer_json = serde_json::to_value(transfer)?;
    Ok(Json(ApiResponse::success_with_message(
        transfer_json,
        "库存调拨单已接收",
    )))
}

/// 删除库存调拨
pub async fn delete_transfer(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let transfer_service = InventoryTransferService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——删除前先校验资源归属（复用 P0-S01 的 get_transfer_detail + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    transfer_service
        .get_transfer_detail(id, Some(&data_scope_ctx))
        .await?;

    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    // 原 map_err(bad_request) 会把 service 的 not_found/业务拒绝等真实错误类型压成 400，
    // 改为 ? 透传（错误按真正责任模块定性）
    transfer_service.delete_transfer(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        (),
        "库存调拨单已删除",
    )))
}

/// 列出调拨单的所有明细项
pub async fn list_items(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let transfer_service = InventoryTransferService::new(state.db.clone());
    let items = transfer_service.list_items(id).await?;
    let items_json: Vec<serde_json::Value> = items
        .into_iter()
        .map(serde_json::to_value)
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
    // 原 map_err(bad_request) 会把 service 的 not_found/业务拒绝等真实错误类型压成 400，
    // 改为 ? 透传（错误按真正责任模块定性）
    let item = transfer_service.add_item(id, request).await?;
    let item_json = serde_json::to_value(item)?;
    Ok(Json(ApiResponse::success_with_message(
        item_json,
        "调拨明细添加成功",
    )))
}

/// 更新调拨单明细
///
/// 载荷为三态 DTO（UpdateInventoryTransferItemRequest，服务侧 inv/mod.rs 定义）：
/// 键缺席=保持、显式 null=清空（可空列 notes/unit_cost/piece_no 置 NULL；缸号列
/// DDL 为 NOT NULL DEFAULT ''，其"清空"落空串）、有值=覆盖；
/// NOT NULL 列显式 null 由 service 在任何 DB 访问前拒绝。
pub async fn update_item(
    State(state): State<AppState>,
    Path(item_id): Path<i32>,
    Json(request): Json<UpdateInventoryTransferItemRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let transfer_service = InventoryTransferService::new(state.db.clone());
    // 原 map_err(bad_request) 会把 service 的 not_found/业务拒绝（如 NOT NULL 拒清）
    // 压成 400 脱敏文本，改为 ? 透传（错误按真正责任模块定性）
    let item = transfer_service.update_item(item_id, request).await?;
    let item_json = serde_json::to_value(item)?;
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
    // 原 map_err(bad_request) 会把 service 的真实错误类型压成 400，改为 ? 透传
    transfer_service.delete_item(item_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        (),
        "调拨明细已删除",
    )))
}

/// 生成库存调拨单号 GET /api/v1/erp/inventory/transfers/generate-no；单据号格式：`TRF{yyyyMMdd}{3 位流水}`
/// 例如 `TRF20260514001`。前缀/位数与落库权威
/// `generate_transfer_no`（inv/inventory_move.rs，impl_generate_no! "TRF"，默认 3 位）逐字一致，
/// 修复展示码≠落库码双轨缺陷（原展示 "IT"/4 位）。
/// 数据库列 `inventory_transfers.transfer_no` 上的 `UNIQUE` 约束负责最终去重。
pub async fn generate_no(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let transfer_no = DocumentNumberGenerator::generate_no(
        &*state.db,
        "TRF",
        inventory_transfer::Entity,
        inventory_transfer::Column::TransferNo,
    )
    .await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "transfer_no": transfer_no
    }))))
}
