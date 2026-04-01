use crate::middleware::auth_context::AuthContext;
use crate::models::fixed_asset;
use crate::services::fixed_asset_service::{
    CreateAssetRequest, DisposalRequest, FixedAssetService,
};
use crate::utils::error::AppError;
use crate::utils::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::DatabaseConnection;
use crate::utils::app_state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// 资产查询参数 DTO
#[derive(Debug, Deserialize)]
pub struct AssetQuery {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub asset_category: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 创建资产请求 DTO
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateAssetRequestDto {
    pub asset_no: String,
    pub asset_name: String,
    pub asset_category: Option<String>,
    pub specification: Option<String>,
    pub location: Option<String>,
    pub original_value: rust_decimal::Decimal,
    pub useful_life: i32,
    pub depreciation_method: Option<String>,
    pub purchase_date: chrono::NaiveDate,
    pub put_in_date: chrono::NaiveDate,
    pub supplier_id: Option<i32>,
    pub remark: Option<String>,
}

/// 计提折旧请求 DTO
#[derive(Debug, Deserialize)]
pub struct DepreciateRequest {
    pub period: String,
}

/// 资产处置请求 DTO
#[derive(Debug, Deserialize)]
pub struct DisposalRequestDto {
    pub disposal_type: String,
    pub disposal_value: rust_decimal::Decimal,
    pub disposal_date: chrono::NaiveDate,
    pub reason: String,
    pub buyer_info: Option<String>,
}

/// 获取资产列表
pub async fn list_assets(
    Query(params): Query<AssetQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<fixed_asset::Model>>>, AppError> {
    info!("用户 {} 正在查询资产列表", auth.user_id);

    let service = FixedAssetService::new(db);
    let query_params = crate::services::fixed_asset_service::AssetQueryParams {
        keyword: params.keyword,
        status: params.status,
        asset_category: params.asset_category,
        page: params.page.unwrap_or(0),
        page_size: params.page_size.unwrap_or(10),
    };

    let (assets, _total) = service.get_list(query_params).await?;
    info!("资产列表查询成功，共 {} 条记录", assets.len());

    Ok(Json(ApiResponse::success(assets)))
}

/// 获取资产详情
pub async fn get_asset(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<fixed_asset::Model>>, AppError> {
    info!("用户 {} 正在查询资产详情：{}", auth.user_id, id);

    let service = FixedAssetService::new(db);
    let asset = service.get_by_id(id).await?;
    info!("资产详情查询成功：{}", asset.asset_no);

    Ok(Json(ApiResponse::success(asset)))
}

/// 创建资产
#[axum::debug_handler]
pub async fn create_asset(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateAssetRequestDto>,
) -> Result<Json<ApiResponse<fixed_asset::Model>>, AppError> {
    info!("用户 {} 正在创建资产：{}", auth.user_id, req.asset_no);

    let service = FixedAssetService::new(db);
    let create_req = CreateAssetRequest {
        asset_no: req.asset_no,
        asset_name: req.asset_name,
        asset_category: req.asset_category,
        specification: req.specification,
        location: req.location,
        original_value: req.original_value,
        useful_life: req.useful_life,
        depreciation_method: req.depreciation_method,
        purchase_date: req.purchase_date,
        put_in_date: req.put_in_date,
        supplier_id: req.supplier_id,
        remark: req.remark,
    };

    let asset = service.create(create_req, auth.user_id).await?;
    info!("资产创建成功：{}", asset.asset_no);

    Ok(Json(ApiResponse::success(asset)))
}

/// 计提折旧
#[axum::debug_handler]
pub async fn depreciate_asset(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<DepreciateRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!(
        "用户 {} 正在计提资产 {} 的 {} 折旧",
        auth.user_id, id, req.period
    );

    let service = FixedAssetService::new(db);
    service.depreciate(id, &req.period, auth.user_id).await?;

    let message = format!("资产 {} 折旧计提成功", id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}

/// 处置资产
#[axum::debug_handler]
pub async fn dispose_asset(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<DisposalRequestDto>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在处置资产 {}", auth.user_id, id);

    let service = FixedAssetService::new(db);
    let disposal_req = DisposalRequest {
        disposal_type: req.disposal_type,
        disposal_value: req.disposal_value,
        disposal_date: req.disposal_date,
        reason: req.reason,
        buyer_info: req.buyer_info,
    };

    service.dispose(id, disposal_req, auth.user_id).await?;

    let message = format!("资产 {} 处置成功", id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}

/// 删除资产
pub async fn delete_asset(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在删除资产 {}", auth.user_id, id);

    let service = FixedAssetService::new(db);
    service.delete(id, auth.user_id).await?;

    let message = format!("资产 {} 删除成功", id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}
