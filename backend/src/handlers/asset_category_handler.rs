//! 资产分类 Handler

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::asset_category;
use crate::services::asset_category_service::{
    AssetCategoryQueryParams, AssetCategoryService, CreateAssetCategoryRequest,
    UpdateAssetCategoryRequest,
};
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use rust_decimal::Decimal;
use serde::Deserialize;
use tracing::info;

/// 查询参数 DTO
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CategoryQuery {
    pub is_active: Option<bool>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 创建请求 DTO
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CreateCategoryDto {
    pub category_code: String,
    pub category_name: String,
    pub parent_id: Option<i32>,
    pub default_useful_life: Option<i32>,
    pub default_depreciation_method: Option<String>,
    pub default_salvage_rate: Option<Decimal>,
    pub description: Option<String>,
}

/// 更新请求 DTO
///
/// 可空列三态（对齐 RFC 7386）：键缺席=保持原值、显式 `null`=清空为 NULL、有值=覆盖。
/// serde_json 会把请求体里的显式 `null` 折叠成外层 `None`，因此必须挂
/// `deserialize_with = "double_option"`，否则"清空"与"未提供"同物、清空操作被静默丢弃。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct UpdateCategoryDto {
    pub category_name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub parent_id: Option<Option<i32>>,
    #[serde(default, deserialize_with = "double_option")]
    pub default_useful_life: Option<Option<i32>>,
    #[serde(default, deserialize_with = "double_option")]
    pub default_depreciation_method: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub default_salvage_rate: Option<Option<Decimal>>,
    #[serde(default, deserialize_with = "double_option")]
    pub description: Option<Option<String>>,
    pub is_active: Option<bool>,
}

/// 把 JSON 值解成"外层是否存在该键"与"内层是否为 null"两层可辨的形态。
/// 范式同 `department_handler::double_option`。
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

/// 创建资产分类
pub async fn create_category(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateCategoryDto>,
) -> Result<Json<ApiResponse<asset_category::Model>>, AppError> {
    info!(
        "用户 {} 正在创建资产分类：{}",
        auth.user_id, req.category_name
    );

    let service = AssetCategoryService::new(state.db.clone());
    let category = service
        .create(
            CreateAssetCategoryRequest {
                category_code: req.category_code,
                category_name: req.category_name,
                parent_id: req.parent_id,
                default_useful_life: req.default_useful_life,
                default_depreciation_method: req.default_depreciation_method,
                default_salvage_rate: req.default_salvage_rate,
                description: req.description,
            },
            auth.user_id,
        )
        .await?;

    Ok(Json(ApiResponse::success(category)))
}

/// 查询资产分类列表
pub async fn list_categories(
    Query(params): Query<CategoryQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<PaginatedResponse<asset_category::Model>>>, AppError> {
    info!("用户 {} 正在查询资产分类列表", auth.user_id);

    let service = AssetCategoryService::new(state.db.clone());
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    let (categories, total) = service
        .list(AssetCategoryQueryParams {
            is_active: params.is_active,
            page,
            page_size,
        })
        .await?;

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        categories,
        total,
        page as u64,
        page_size as u64,
    ))))
}

/// 获取资产分类详情
pub async fn get_category(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<asset_category::Model>>, AppError> {
    info!("用户 {} 正在查询资产分类 {}", auth.user_id, id);

    let service = AssetCategoryService::new(state.db.clone());
    let category = service.get_by_id(id).await?;

    Ok(Json(ApiResponse::success(category)))
}

/// 更新资产分类
pub async fn update_category(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateCategoryDto>,
) -> Result<Json<ApiResponse<asset_category::Model>>, AppError> {
    info!("用户 {} 正在更新资产分类 {}", auth.user_id, id);

    let service = AssetCategoryService::new(state.db.clone());
    let category = service
        .update(
            id,
            UpdateAssetCategoryRequest {
                category_name: req.category_name,
                parent_id: req.parent_id,
                default_useful_life: req.default_useful_life,
                default_depreciation_method: req.default_depreciation_method,
                default_salvage_rate: req.default_salvage_rate,
                description: req.description,
                is_active: req.is_active,
            },
        )
        .await?;

    Ok(Json(ApiResponse::success(category)))
}

/// 删除资产分类
pub async fn delete_category(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在删除资产分类 {}", auth.user_id, id);

    let service = AssetCategoryService::new(state.db.clone());
    service.delete(id).await?;

    Ok(Json(ApiResponse::success(format!(
        "资产分类 {} 已停用",
        id
    ))))
}
