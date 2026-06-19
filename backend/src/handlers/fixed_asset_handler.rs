
use crate::middleware::auth_context::AuthContext;
use crate::models::fixed_asset;
use crate::services::fixed_asset_service::{
    CreateAssetRequest, DepreciationResult, DisposalRequest, FixedAssetService,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
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
    pub asset_no: Option<String>,
    pub asset_name: Option<String>,
    pub asset_category: Option<String>,
    pub specification: Option<String>,
    pub location: Option<String>,
    pub original_value: Option<rust_decimal::Decimal>,
    pub useful_life: Option<i32>,
    pub depreciation_method: Option<String>,
    pub purchase_date: Option<chrono::NaiveDate>,
    pub put_in_date: Option<chrono::NaiveDate>,
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

    let service = FixedAssetService::new(state.db.clone());
    let query_params = crate::services::fixed_asset_service::AssetQueryParams {
        keyword: params.keyword,
        status: params.status,
        asset_category: params.asset_category,
        page: params.page.unwrap_or_default(),
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

    let service = FixedAssetService::new(state.db.clone());
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
    info!(
        "用户 {} 正在创建资产：{}",
        auth.user_id,
        req.asset_no.as_deref().unwrap_or("自动生成")
    );

    let service = FixedAssetService::new(state.db.clone());
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

    let service = FixedAssetService::new(state.db.clone());
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

    let service = FixedAssetService::new(state.db.clone());
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

    let service = FixedAssetService::new(state.db.clone());
    service.delete(id, auth.user_id).await?;

    let message = format!("资产 {} 删除成功", id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}

/// PUT /api/v1/erp/fixed-assets/:id - 更新固定资产
pub async fn update_asset(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 更新固定资产: ID={}", auth.username, id);

    let service = FixedAssetService::new(state.db.clone());

    // 获取现有资产
    let mut asset = service.get_by_id(id).await?;

    // 更新字段
    if let Some(name) = req.get("asset_name").and_then(|v| v.as_str()) {
        asset.asset_name = name.to_string();
    }
    if let Some(category) = req.get("asset_category").and_then(|v| v.as_str()) {
        asset.asset_category = Some(category.to_string());
    }
    if let Some(spec) = req.get("specification").and_then(|v| v.as_str()) {
        asset.specification = Some(spec.to_string());
    }
    if let Some(location) = req.get("use_location").and_then(|v| v.as_str()) {
        asset.use_location = Some(location.to_string());
    }

    // 保存更新
    use sea_orm::ActiveModelTrait;
    let mut active_model: crate::models::fixed_asset::ActiveModel = asset.into();
    active_model.updated_at = sea_orm::Set(chrono::Utc::now());

    let updated = active_model.update(&*state.db).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(updated)?,
        "固定资产更新成功",
    )))
}

/// 批量折旧计算
pub async fn batch_depreciate(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<BatchDepreciateRequest>,
) -> Result<Json<ApiResponse<Vec<DepreciationResult>>>, AppError> {
    info!("用户 {} 正在批量计算折旧", auth.username);

    let service = FixedAssetService::new(state.db.clone());
    let results = service
        .batch_calculate_depreciation(req.asset_ids, req.calculation_date, req.user_id)
        .await?;

    Ok(Json(ApiResponse::success(results)))
}

/// 折旧计算请求
#[derive(Debug, Deserialize)]
pub struct BatchDepreciateRequest {
    pub asset_ids: Vec<i32>,
    pub calculation_date: String,
    pub user_id: i32,
}
