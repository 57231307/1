//! 采购质检 Handler
//!
//! 采购质检 HTTP 接口层

use crate::middleware::auth_context::AuthContext;
use crate::services::purchase_inspection_service::{
    CompleteInspectionRequest, CreateInspectionItemRequest, CreatePurchaseInspectionRequest,
    PurchaseInspectionService, UpdateInspectionItemRequest, UpdatePurchaseInspectionRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 查询采购质检单列表
pub async fn list_inspections(
    Query(params): Query<InspectionQueryParams>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseInspectionService::new(state.db.clone());
    let (inspections, total) = service
        .list_inspections(
            params.page.unwrap_or(1).clamp(1, 1000), // 批次 95 P3-3~8：分页 clamp 防 DoS
            params.page_size.unwrap_or(20).clamp(1, 100),
            params.status,
            params.supplier_id,
        )
        .await?;

    let result = serde_json::to_value(PaginatedResponse::new(
        inspections,
        total,
        params.page.unwrap_or(1).clamp(1, 1000), // 批次 95 P3-3~8：分页 clamp 防 DoS
        params.page_size.unwrap_or(20).clamp(1, 100),
    ))
    .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(ApiResponse::success(result)))
}

/// 获取采购质检单详情
pub async fn get_inspection(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseInspectionService::new(state.db.clone());
    let inspection = service.get_inspection(id).await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(
        inspection,
    )?)))
}

/// 创建采购质检单
#[axum::debug_handler]
pub async fn create_inspection(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreatePurchaseInspectionRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.validate()?;

    let service = PurchaseInspectionService::new(state.db.clone());

    let inspection = service.create_inspection(req, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(inspection)?,
        "采购质检单创建成功",
    )))
}

/// 更新采购质检单
#[axum::debug_handler]
pub async fn update_inspection(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdatePurchaseInspectionRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseInspectionService::new(state.db.clone());

    let inspection = service.update_inspection(id, req, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(inspection)?,
        "采购质检单更新成功",
    )))
}

/// 完成采购质检单
#[axum::debug_handler]
pub async fn complete_inspection(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CompleteInspectionRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.validate()?;

    let service = PurchaseInspectionService::new(state.db.clone());

    let inspection = service.complete_inspection(id, req, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(inspection)?,
        "采购质检单已完成",
    )))
}

// =====================================================
// 请求 DTO
// =====================================================

/// 采购质检单查询参数
#[derive(Debug, Deserialize)]
pub struct InspectionQueryParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
}

/// P1-2i 修复（批次 81 v1 复审）：创建质检明细请求 DTO
/// 替代 create_inspection_item 中的 Json<serde_json::Value>，提供强类型校验
/// 需要 Serialize：handler 将 DTO 序列化为 Value 回显给前端
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct CreateInspectionItemDto {
    /// 产品 ID：必填
    pub product_id: i32,
    /// 检验项目名称：必填
    #[validate(length(min = 1, max = 100, message = "检验项目名称长度必须在1到100字符之间"))]
    pub item_name: String,
    /// 合格数量：必填
    pub qualified_quantity: rust_decimal::Decimal,
    /// 不合格数量：必填
    pub unqualified_quantity: rust_decimal::Decimal,
    /// 备注：可选
    #[validate(length(max = 500, message = "备注长度不能超过500字符"))]
    pub remark: Option<String>,
}

/// P1-2i 修复（批次 81 v1 复审）：更新质检明细请求 DTO
/// 替代 update_inspection_item 中的 Json<serde_json::Value>，所有字段可选
/// 需要 Serialize：handler 将 DTO 序列化为 Value 回显给前端
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct UpdateInspectionItemDto {
    /// 合格数量：可选
    pub qualified_quantity: Option<rust_decimal::Decimal>,
    /// 不合格数量：可选
    pub unqualified_quantity: Option<rust_decimal::Decimal>,
    /// 备注：可选
    #[validate(length(max = 500, message = "备注长度不能超过500字符"))]
    pub remark: Option<String>,
}

// =====================================================
// 质检明细 Handler
// =====================================================

/// 获取质检明细列表
///
/// 批次 131 v9 复审 P0：原返回硬编码空列表 {items: [], total: 0}，
/// 现真实查询 purchase_inspection_items 表。
pub async fn list_inspection_items(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseInspectionService::new(state.db.clone());
    let items = service.list_inspection_items(id).await?;

    let total = items.len() as i64;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
        "inspection_id": id
    }))))
}

/// 创建质检明细
///
/// 批次 131 v9 复审 P0：原仅记日志不落库，现真实 INSERT purchase_inspection_items 表。
pub async fn create_inspection_item(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateInspectionItemDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    // handler DTO 转 service DTO
    let svc_req = CreateInspectionItemRequest {
        product_id: req.product_id,
        item_name: req.item_name,
        qualified_quantity: req.qualified_quantity,
        unqualified_quantity: req.unqualified_quantity,
        remark: req.remark,
    };

    let service = PurchaseInspectionService::new(state.db.clone());
    let item = service.create_inspection_item(id, svc_req).await?;

    tracing::info!("用户 {} 为质检单 {} 创建明细 {}", auth.user_id, id, item.id);

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(&item)?,
        "质检明细创建成功",
    )))
}

/// 更新质检明细
///
/// 批次 131 v9 复审 P0：原仅记日志不更新，现真实 UPDATE purchase_inspection_items 表。
pub async fn update_inspection_item(
    Path((id, item_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateInspectionItemDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    // handler DTO 转 service DTO
    let svc_req = UpdateInspectionItemRequest {
        qualified_quantity: req.qualified_quantity,
        unqualified_quantity: req.unqualified_quantity,
        remark: req.remark,
    };

    let service = PurchaseInspectionService::new(state.db.clone());
    let item = service
        .update_inspection_item(id, item_id, svc_req)
        .await?;

    tracing::info!(
        "用户 {} 更新质检单 {} 的明细 {}",
        auth.user_id,
        id,
        item_id
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(&item)?,
        "质检明细更新成功",
    )))
}

/// 删除质检明细
///
/// 批次 131 v9 复审 P0：原仅记日志不删除，现真实 DELETE purchase_inspection_items 表。
pub async fn delete_inspection_item(
    Path((id, item_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = PurchaseInspectionService::new(state.db.clone());
    let deleted = service.delete_inspection_item(id, item_id).await?;

    if deleted {
        tracing::info!(
            "用户 {} 删除质检单 {} 的明细 {}",
            auth.user_id,
            id,
            item_id
        );
    }

    Ok(Json(ApiResponse::success_with_message(
        (),
        "质检明细删除成功",
    )))
}
