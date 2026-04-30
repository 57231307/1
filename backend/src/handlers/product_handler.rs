use axum::{
    extract::{Path, Query, State},
    Json,
};
use crate::utils::app_state::AppState;
use serde::Deserialize;

use crate::models::product;
use crate::models::product_color;
use crate::services::product_service::ProductService;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

/// 查询参数 - 产品列表
#[derive(Debug, Deserialize)]
pub struct ProductListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub category_id: Option<i32>,
    pub status: Option<String>,
    pub search: Option<String>,
}

/// 创建产品请求（面料行业版）
#[derive(Debug, Deserialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub code: String,
    pub category_id: Option<i32>,
    pub specification: Option<String>,
    pub unit: String,
    pub standard_price: Option<f64>,
    pub cost_price: Option<f64>,
    pub description: Option<String>,
    pub status: Option<String>,
    // 面料行业字段
    pub product_type: String,
    pub fabric_composition: Option<String>,
    pub yarn_count: Option<String>,
    pub density: Option<String>,
    pub width: Option<f64>,
    pub gram_weight: Option<f64>,
    pub structure: Option<String>,
    pub finish: Option<String>,
    pub min_order_quantity: Option<f64>,
    pub lead_time: Option<i32>,
}

/// 更新产品请求（面料行业版）
#[derive(Debug, Deserialize)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub specification: Option<String>,
    pub unit: Option<String>,
    pub standard_price: Option<f64>,
    pub cost_price: Option<f64>,
    pub description: Option<String>,
    pub status: Option<String>,
    // 面料行业字段
    pub product_type: Option<String>,
    pub fabric_composition: Option<String>,
    pub yarn_count: Option<String>,
    pub density: Option<String>,
    pub width: Option<f64>,
    pub gram_weight: Option<f64>,
    pub structure: Option<String>,
    pub finish: Option<String>,
    pub min_order_quantity: Option<f64>,
    pub lead_time: Option<i32>,
}

// ========== 色号管理相关结构体 ==========

/// 创建产品色号请求
#[derive(Debug, Deserialize)]
pub struct CreateProductColorRequest {
    pub color_no: String,
    pub color_name: String,
    pub pantone_code: Option<String>,
    pub color_type: String,
    pub dye_formula: Option<String>,
    pub extra_cost: f64,
}

/// 更新产品色号请求
#[derive(Debug, Deserialize)]
pub struct UpdateProductColorRequest {
    pub color_name: Option<String>,
    pub pantone_code: Option<String>,
    pub color_type: Option<String>,
    pub dye_formula: Option<String>,
    pub extra_cost: Option<f64>,
    pub is_active: Option<bool>,
}

/// 批量创建色号请求
#[derive(Debug, Deserialize)]
pub struct BatchCreateColorsRequest {
    pub colors: Vec<CreateProductColorRequest>,
}

/// 获取产品列表
pub async fn list_products(
    State(state): State<AppState>,
    Query(query): Query<ProductListQuery>,
) -> Result<Json<ApiResponse<Vec<product::Model>>>, AppError> {
    let product_service = ProductService::new(state.db.clone());

    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(10);

    let (products, total) = product_service
        .list_products(
            page,
            page_size,
            query.category_id,
            query.status,
            query.search,
        )
        .await?;

    Ok(Json(
        PaginatedResponse::new(products, total, page, page_size).into(),
    ))
}

/// 获取产品详情
pub async fn get_product(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<product::Model>>, AppError> {
    let product_service = ProductService::new(state.db.clone());
    let product = product_service.get_product(id).await?;
    Ok(Json(ApiResponse::success(product)))
}

/// 创建产品
pub async fn create_product(
    State(state): State<AppState>,
    Json(req): Json<CreateProductRequest>,
) -> Result<Json<ApiResponse<product::Model>>, AppError> {
    let product_service = ProductService::new(state.db.clone());

    let product = product_service
        .create_product(
            req.name,
            req.code,
            req.category_id,
            req.specification,
            req.unit,
            req.standard_price,
            req.cost_price,
            req.description,
            req.status.unwrap_or_else(|| "active".to_string()),
            req.product_type,
            req.fabric_composition,
            req.yarn_count,
            req.density,
            req.width,
            req.gram_weight,
            req.structure,
            req.finish,
            req.min_order_quantity,
            req.lead_time,
        )
        .await?;

    Ok(Json(ApiResponse::success_with_msg(product, "产品创建成功")))
}

/// 更新产品
pub async fn update_product(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateProductRequest>,
) -> Result<Json<ApiResponse<product::Model>>, AppError> {
    let product_service = ProductService::new(state.db.clone());

    let product = product_service
        .update_product(
            id,
            req.name,
            req.specification,
            req.unit,
            req.standard_price,
            req.cost_price,
            req.description,
            req.status,
            req.product_type,
            req.fabric_composition,
            req.yarn_count,
            req.density,
            req.width,
            req.gram_weight,
            req.structure,
            req.finish,
            req.min_order_quantity,
            req.lead_time,
        )
        .await?;

    Ok(Json(ApiResponse::success_with_msg(product, "产品更新成功")))
}

/// 删除产品
pub async fn delete_product(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let product_service = ProductService::new(state.db.clone());
    product_service.delete_product(id).await?;
    Ok(Json(ApiResponse::success_with_msg((), "产品删除成功")))
}

// ========== 色号管理接口 ==========

/// 获取产品色号列表
pub async fn list_product_colors(
    State(state): State<AppState>,
    Path(product_id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<product_color::Model>>>, AppError> {
    let product_service = ProductService::new(state.db.clone());
    let colors = product_service.list_product_colors(product_id).await?;
    Ok(Json(ApiResponse::success(colors)))
}

/// 创建产品色号
pub async fn create_product_color(
    State(state): State<AppState>,
    Path(product_id): Path<i32>,
    Json(req): Json<CreateProductColorRequest>,
) -> Result<Json<ApiResponse<product_color::Model>>, AppError> {
    let product_service = ProductService::new(state.db.clone());

    let color = product_service
        .create_product_color(
            product_id,
            req.color_no,
            req.color_name,
            req.pantone_code,
            req.color_type,
            req.dye_formula,
            req.extra_cost,
        )
        .await?;

    Ok(Json(ApiResponse::success_with_msg(color, "色号创建成功")))
}

/// 更新产品色号
pub async fn update_product_color(
    State(state): State<AppState>,
    Path((_product_id, color_id)): Path<(i32, i32)>,
    Json(req): Json<UpdateProductColorRequest>,
) -> Result<Json<ApiResponse<product_color::Model>>, AppError> {
    let product_service = ProductService::new(state.db.clone());

    let color = product_service
        .update_product_color(
            color_id,
            req.color_name,
            req.pantone_code,
            req.color_type,
            req.dye_formula,
            req.extra_cost,
            req.is_active,
        )
        .await?;

    Ok(Json(ApiResponse::success_with_msg(color, "色号更新成功")))
}

/// 删除产品色号
pub async fn delete_product_color(
    State(state): State<AppState>,
    Path((_product_id, color_id)): Path<(i32, i32)>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let product_service = ProductService::new(state.db.clone());
    product_service.delete_product_color(color_id).await?;
    Ok(Json(ApiResponse::success_with_msg((), "色号删除成功")))
}

/// 批量创建色号
pub async fn batch_create_colors(
    State(state): State<AppState>,
    Path(product_id): Path<i32>,
    Json(req): Json<BatchCreateColorsRequest>,
) -> Result<Json<ApiResponse<Vec<product_color::Model>>>, AppError> {
    let product_service = ProductService::new(state.db.clone());

    let colors_input: Vec<_> = req
        .colors
        .into_iter()
        .map(
            |c| crate::services::product_service::CreateProductColorInput {
                color_no: c.color_no,
                color_name: c.color_name,
                pantone_code: c.pantone_code,
                color_type: c.color_type,
                dye_formula: c.dye_formula,
                extra_cost: c.extra_cost,
            },
        )
        .collect();

    let colors = product_service
        .batch_create_product_colors(product_id, colors_input)
        .await?;
    let msg = format!("批量创建{}个色号成功", colors.len());
    Ok(Json(ApiResponse::success_with_msg(colors, &msg)))
}
