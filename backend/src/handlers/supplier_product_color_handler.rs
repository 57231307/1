//! 供应商商品色号目录 Handler（supplier-product-colors）
//!
//! list（?supplier_product_id 必填 + ?keyword）、get、create、update。软停用由 update 置
//! is_enabled=false 完成（表被 product_supplier_mappings 以 FK 引用，不提供硬删）。

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::supplier_product_color_service::{
    CreateSupplierProductColorRequest, SupplierProductColorQueryParams,
    SupplierProductColorService, UpdateSupplierProductColorRequest,
};
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use tracing::info;
use validator::Validate;

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ListSupplierProductColorsQuery {
    pub supplier_product_id: Option<i32>,
    pub keyword: Option<String>,
    pub is_enabled: Option<bool>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// GET /supplier-product-colors
pub async fn list_supplier_product_colors(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ListSupplierProductColorsQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<serde_json::Value>>>, AppError> {
    let service = SupplierProductColorService::new(state.db.clone());
    let (items, total, page, page_size) = service
        .list(SupplierProductColorQueryParams {
            supplier_product_id: params.supplier_product_id,
            keyword: params.keyword,
            is_enabled: params.is_enabled,
            page: params.page,
            page_size: params.page_size,
        })
        .await?;

    let items_json: Vec<serde_json::Value> = items
        .into_iter()
        .map(|d| serde_json::to_value(d).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;

    info!("供应商色号列表查询成功，共 {} 条", items_json.len());
    Ok(Json(ApiResponse::success_paginated(
        items_json, total, page, page_size,
    )))
}

/// GET /supplier-product-colors/{id}
pub async fn get_supplier_product_color(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SupplierProductColorService::new(state.db.clone());
    let model = service.get(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// POST /supplier-product-colors
pub async fn create_supplier_product_color(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateSupplierProductColorRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.validate()?;
    info!("用户 {} 正在创建供应商色号", auth.user_id);
    let service = SupplierProductColorService::new(state.db.clone());
    let model = service.create(req).await?;
    info!("供应商色号创建成功，ID: {}", model.id);
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(model)?,
        "供应商色号创建成功",
    )))
}

/// PUT /supplier-product-colors/{id}
pub async fn update_supplier_product_color(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateSupplierProductColorRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.validate()?;
    info!("用户 {} 正在更新供应商色号 ID: {}", auth.user_id, id);
    let service = SupplierProductColorService::new(state.db.clone());
    let model = service.update(id, req).await?;
    info!("供应商色号更新成功，ID: {}", id);
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(model)?,
        "供应商色号更新成功",
    )))
}
