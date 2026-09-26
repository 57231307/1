//! 供应商商品/色号对照表 Handler（sku-mappings）
//!
//! 提供 CRUD、批量导入和 SKU 解析端点。

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::sku_mapping_service::{
    ImportMappingRow, SkuMappingQueryParams, SkuMappingService, UpsertSkuMappingInput,
};
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use tracing::info;

// =====================================================
// 请求 DTO
// =====================================================

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ListMappingsQuery {
    pub product_id: Option<i32>,
    pub supplier_id: Option<i32>,
    pub is_enabled: Option<bool>,
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ResolveQuery {
    pub product_id: i32,
    pub color_id: Option<i32>,
    pub supplier_id: i32,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CreateMappingRequest {
    pub product_id: i32,
    pub product_color_id: Option<i32>,
    pub supplier_id: i32,
    pub supplier_product_id: i32,
    pub supplier_product_color_id: Option<i32>,
    pub supplier_price: Option<String>,
    pub min_order_quantity: Option<String>,
    pub lead_time: Option<i32>,
    pub is_primary: Option<bool>,
    pub priority: Option<i32>,
    pub is_enabled: Option<bool>,
    pub remarks: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct UpdateMappingRequest {
    pub product_id: i32,
    pub product_color_id: Option<i32>,
    pub supplier_id: i32,
    pub supplier_product_id: i32,
    pub supplier_product_color_id: Option<i32>,
    pub supplier_price: Option<String>,
    pub min_order_quantity: Option<String>,
    pub lead_time: Option<i32>,
    pub is_primary: Option<bool>,
    pub priority: Option<i32>,
    pub is_enabled: Option<bool>,
    pub remarks: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ImportMappingsRequest {
    pub rows: Vec<ImportMappingRow>,
}

// =====================================================
// Handlers
// =====================================================

/// GET /sku-mappings
pub async fn list_mappings(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ListMappingsQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<serde_json::Value>>>, AppError> {
    let service = SkuMappingService::new(state.db.clone());
    let query = SkuMappingQueryParams {
        product_id: params.product_id,
        supplier_id: params.supplier_id,
        is_enabled: params.is_enabled,
        keyword: params.keyword,
        page: params.page,
        page_size: params.page_size,
    };

    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let (items, total) = service.list(query).await?;

    let items_json: Vec<serde_json::Value> = items
        .into_iter()
        .map(|d| serde_json::to_value(d).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;

    info!("SKU 对照表列表查询成功，共 {} 条", items_json.len());
    Ok(Json(ApiResponse::success_paginated(
        items_json, total, page, page_size,
    )))
}

/// GET /sku-mappings/{id}
pub async fn get_mapping(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SkuMappingService::new(state.db.clone());
    let dto = service.get(id).await?;
    info!("SKU 对照表查询成功，ID: {}", id);
    Ok(Json(ApiResponse::success(serde_json::to_value(dto)?)))
}

/// POST /sku-mappings
pub async fn create_mapping(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateMappingRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 正在创建 SKU 对照", auth.user_id);

    let service = SkuMappingService::new(state.db.clone());
    let input = to_upsert_input(
        req.product_id,
        req.product_color_id,
        req.supplier_id,
        req.supplier_product_id,
        req.supplier_product_color_id,
        &req.supplier_price,
        &req.min_order_quantity,
        req.lead_time,
        req.is_primary,
        req.priority,
        req.is_enabled,
        req.remarks,
    )?;

    let dto = service.create(input, auth.user_id).await?;
    info!("SKU 对照创建成功，ID: {}", dto.id);
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(dto)?,
        "对照记录创建成功",
    )))
}

/// PUT /sku-mappings/{id}
pub async fn update_mapping(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateMappingRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 正在更新 SKU 对照 ID: {}", auth.user_id, id);

    let service = SkuMappingService::new(state.db.clone());
    let input = to_upsert_input(
        req.product_id,
        req.product_color_id,
        req.supplier_id,
        req.supplier_product_id,
        req.supplier_product_color_id,
        &req.supplier_price,
        &req.min_order_quantity,
        req.lead_time,
        req.is_primary,
        req.priority,
        req.is_enabled,
        req.remarks,
    )?;

    let dto = service.update(id, input, auth.user_id).await?;
    info!("SKU 对照更新成功，ID: {}", id);
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(dto)?,
        "对照记录更新成功",
    )))
}

/// DELETE /sku-mappings/{id}
pub async fn delete_mapping(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 正在删除 SKU 对照 ID: {}", auth.user_id, id);

    let service = SkuMappingService::new(state.db.clone());
    service.delete(id).await?;
    info!("SKU 对照删除成功，ID: {}", id);
    Ok(Json(ApiResponse::success_with_message(
        (),
        "对照记录删除成功",
    )))
}

/// POST /sku-mappings/import
pub async fn import_mappings(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<ImportMappingsRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 正在批量导入 SKU 对照，共 {} 条",
        auth.user_id,
        req.rows.len()
    );

    let service = SkuMappingService::new(state.db.clone());
    let result = service.import_batch(req.rows, auth.user_id).await?;

    info!(
        "SKU 对照批量导入完成: 成功 {}，失败 {}",
        result.success_count, result.error_count
    );
    Ok(Json(ApiResponse::success(serde_json::to_value(result)?)))
}

/// GET /sku-mappings/resolve?product_id=&color_id=&supplier_id=
pub async fn resolve_sku(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ResolveQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SkuMappingService::new(state.db.clone());
    let resolved = service
        .resolve_supplier_sku(params.product_id, params.color_id, params.supplier_id)
        .await?
        .ok_or_else(|| AppError::business("该产品色号暂无供应商对照，请先维护对照表"))?;

    Ok(Json(ApiResponse::success(serde_json::to_value(resolved)?)))
}

// =====================================================
// Helpers
// =====================================================

#[allow(clippy::too_many_arguments)]
fn to_upsert_input(
    product_id: i32,
    product_color_id: Option<i32>,
    supplier_id: i32,
    supplier_product_id: i32,
    supplier_product_color_id: Option<i32>,
    supplier_price: &Option<String>,
    min_order_quantity: &Option<String>,
    lead_time: Option<i32>,
    is_primary: Option<bool>,
    priority: Option<i32>,
    is_enabled: Option<bool>,
    remarks: Option<String>,
) -> Result<UpsertSkuMappingInput, AppError> {
    use rust_decimal::Decimal;
    use std::str::FromStr;

    let sp = match supplier_price {
        Some(s) if !s.is_empty() => Some(
            Decimal::from_str(s)
                .map_err(|_| AppError::validation(format!("supplier_price 格式错误: {}", s)))?,
        ),
        _ => None,
    };
    let moq = match min_order_quantity {
        Some(s) if !s.is_empty() => Some(
            Decimal::from_str(s)
                .map_err(|_| AppError::validation(format!("min_order_quantity 格式错误: {}", s)))?,
        ),
        _ => None,
    };

    Ok(UpsertSkuMappingInput {
        product_id,
        product_color_id,
        supplier_id,
        supplier_product_id,
        supplier_product_color_id,
        supplier_price: sp,
        min_order_quantity: moq,
        lead_time,
        is_primary,
        priority,
        is_enabled,
        remarks,
    })
}
