use crate::utils::app_state::AppState;
use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use serde::Deserialize;
use validator::Validate;

use crate::middleware::auth_context::AuthContext;
use crate::models::product;
use crate::models::product_color;
// 批次 213 P2-5 修复（v12 复审）：硬编码 "active" 替换为 master_data 常量
use crate::models::status::master_data;
use crate::services::product_service::{CreateProductArgs, ProductService, UpdateProductArgs};
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
#[derive(Debug, Deserialize, Validate)]
pub struct CreateProductRequest {
    #[validate(length(max = 200, message = "产品名称长度不能超过200个字符"))]
    pub name: Option<String>,
    #[validate(length(max = 50, message = "产品编码长度不能超过50个字符"))]
    pub code: Option<String>,
    pub category_id: Option<i32>,
    #[validate(length(max = 500, message = "规格型号长度不能超过500个字符"))]
    pub specification: Option<String>,
    #[validate(length(max = 20, message = "计量单位长度不能超过20个字符"))]
    pub unit: Option<String>,
    pub standard_price: Option<f64>,
    pub cost_price: Option<f64>,
    #[validate(length(max = 1000, message = "产品描述长度不能超过1000个字符"))]
    pub description: Option<String>,
    #[validate(length(max = 20, message = "状态长度不能超过20个字符"))]
    pub status: Option<String>,
    // 面料行业字段
    #[validate(length(max = 50, message = "产品类型长度不能超过50个字符"))]
    pub product_type: Option<String>,
    #[validate(length(max = 200, message = "面料成分长度不能超过200个字符"))]
    pub fabric_composition: Option<String>,
    #[validate(length(max = 50, message = "纱支长度不能超过50个字符"))]
    pub yarn_count: Option<String>,
    #[validate(length(max = 50, message = "密度长度不能超过50个字符"))]
    pub density: Option<String>,
    pub width: Option<f64>,
    pub gram_weight: Option<f64>,
    #[validate(length(max = 50, message = "组织结构长度不能超过50个字符"))]
    pub structure: Option<String>,
    #[validate(length(max = 100, message = "后整理长度不能超过100个字符"))]
    pub finish: Option<String>,
    pub min_order_quantity: Option<f64>,
    pub lead_time: Option<i32>,
}

/// 更新产品请求（面料行业版）
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProductRequest {
    #[validate(length(max = 200, message = "产品名称长度不能超过200个字符"))]
    pub name: Option<String>,
    #[validate(length(max = 500, message = "规格型号长度不能超过500个字符"))]
    pub specification: Option<String>,
    #[validate(length(max = 20, message = "计量单位长度不能超过20个字符"))]
    pub unit: Option<String>,
    pub standard_price: Option<f64>,
    pub cost_price: Option<f64>,
    #[validate(length(max = 1000, message = "产品描述长度不能超过1000个字符"))]
    pub description: Option<String>,
    #[validate(length(max = 20, message = "状态长度不能超过20个字符"))]
    pub status: Option<String>,
    // 面料行业字段
    #[validate(length(max = 50, message = "产品类型长度不能超过50个字符"))]
    pub product_type: Option<String>,
    #[validate(length(max = 200, message = "面料成分长度不能超过200个字符"))]
    pub fabric_composition: Option<String>,
    #[validate(length(max = 50, message = "纱支长度不能超过50个字符"))]
    pub yarn_count: Option<String>,
    #[validate(length(max = 50, message = "密度长度不能超过50个字符"))]
    pub density: Option<String>,
    pub width: Option<f64>,
    pub gram_weight: Option<f64>,
    #[validate(length(max = 50, message = "组织结构长度不能超过50个字符"))]
    pub structure: Option<String>,
    #[validate(length(max = 100, message = "后整理长度不能超过100个字符"))]
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

/// 导入产品请求
#[derive(Debug, Deserialize)]
pub struct ImportProductsRequest {
    /// CSV 数据，每行一个产品，使用逗号分隔
    /// 第一行为表头，后续为数据
    pub csv_data: String,
}

/// 导出产品查询参数
#[derive(Debug, Deserialize)]
pub struct ExportProductsQuery {
    pub category_id: Option<i32>,
    pub status: Option<String>,
    pub search: Option<String>,
}

/// 获取产品列表
use crate::utils::field_mask::mask_sensitive_fields;

pub async fn list_products(
    Extension(auth): Extension<AuthContext>,
    State(state): State<AppState>,
    Query(query): Query<ProductListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<serde_json::Value>>>, AppError> {
    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());

    let page = query.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

    let (products, total) = product_service
        .list_products(
            page,
            page_size,
            query.category_id,
            query.status,
            query.search,
        )
        .await?;

    // Serialize each product model to Value and mask sensitive fields
    let masked_products: Vec<serde_json::Value> = products
        .into_iter()
        .map(|p| {
            serde_json::to_value(p)
                .map(|val| mask_sensitive_fields(val, &auth))
                .unwrap_or_else(|e| {
                    tracing::error!("Product serialization failed: {:?}", e);
                    serde_json::Value::Null
                })
        })
        .collect();

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        masked_products,
        total,
        page,
        page_size,
    ))))
}

/// 获取产品详情
pub async fn get_product(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<product::Model>>, AppError> {
    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());
    let product = product_service.get_product(id).await?;
    Ok(Json(ApiResponse::success(product)))
}

/// 创建产品
pub async fn create_product(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateProductRequest>,
) -> Result<Json<ApiResponse<product::Model>>, AppError> {
    // 输入验证
    if let Err(e) = req.validate() {
        return Err(AppError::validation(e.to_string()));
    }

    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());

    // 自动生成产品编码
    let code = match req.code {
        Some(c) if !c.is_empty() => c,
        _ => product_service.generate_product_code().await?,
    };

    let product = product_service
        .create_product(CreateProductArgs {
            name: req.name
                .unwrap_or_else(|| format!("产品_{}", chrono::Utc::now().timestamp())),
            code,
            category_id: req.category_id,
            specification: req.specification,
            unit: req.unit.unwrap_or_else(|| "个".to_string()),
            standard_price: req.standard_price,
            cost_price: req.cost_price,
            description: req.description,
            status: req.status.unwrap_or_else(|| master_data::ACTIVE.to_string()),
            product_type: req.product_type.unwrap_or_else(|| "成品".to_string()),
            fabric_composition: req.fabric_composition,
            yarn_count: req.yarn_count,
            density: req.density,
            width: req.width,
            gram_weight: req.gram_weight,
            structure: req.structure,
            finish: req.finish,
            min_order_quantity: req.min_order_quantity,
            lead_time: req.lead_time,
        })
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        product,
        "产品创建成功",
    )))
}

/// 更新产品
pub async fn update_product(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateProductRequest>,
) -> Result<Json<ApiResponse<product::Model>>, AppError> {
    // 输入验证
    if let Err(e) = req.validate() {
        return Err(AppError::validation(e.to_string()));
    }

    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());

    let product = product_service
        .update_product(UpdateProductArgs {
            id,
            name: req.name,
            specification: req.specification,
            unit: req.unit,
            standard_price: req.standard_price,
            cost_price: req.cost_price,
            description: req.description,
            status: req.status,
            product_type: req.product_type,
            fabric_composition: req.fabric_composition,
            yarn_count: req.yarn_count,
            density: req.density,
            width: req.width,
            gram_weight: req.gram_weight,
            structure: req.structure,
            finish: req.finish,
            min_order_quantity: req.min_order_quantity,
            lead_time: req.lead_time,
            // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
            user_id: auth.user_id,
        })
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        product,
        "产品更新成功",
    )))
}

/// 删除产品
pub async fn delete_product(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());
    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    product_service.delete_product(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message((), "产品删除成功")))
}

// ========== 色号管理接口 ==========

/// 获取产品色号列表
pub async fn list_product_colors(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(product_id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<product_color::Model>>>, AppError> {
    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());
    let colors = product_service.list_product_colors(product_id).await?;
    Ok(Json(ApiResponse::success(colors)))
}

/// 创建产品色号
pub async fn create_product_color(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(product_id): Path<i32>,
    Json(req): Json<CreateProductColorRequest>,
) -> Result<Json<ApiResponse<product_color::Model>>, AppError> {
    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());

    let color = product_service
        .create_product_color(
            product_id,
            crate::services::product_service::CreateProductColorInput {
                color_no: req.color_no,
                color_name: req.color_name,
                pantone_code: req.pantone_code,
                color_type: req.color_type,
                dye_formula: req.dye_formula,
                extra_cost: req.extra_cost,
            },
        )
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        color,
        "色号创建成功",
    )))
}

/// 更新产品色号
pub async fn update_product_color(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((_product_id, color_id)): Path<(i32, i32)>,
    Json(req): Json<UpdateProductColorRequest>,
) -> Result<Json<ApiResponse<product_color::Model>>, AppError> {
    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());

    // 批次 330 v10 复审 P3 修复：使用参数对象替代多参数
    let params = crate::services::product_service::UpdateProductColorParams {
        id: color_id,
        color_name: req.color_name,
        pantone_code: req.pantone_code,
        color_type: req.color_type,
        dye_formula: req.dye_formula,
        extra_cost: req.extra_cost,
        is_active: req.is_active,
        user_id: auth.user_id,
    };
    let color = product_service.update_product_color(params).await?;

    Ok(Json(ApiResponse::success_with_message(
        color,
        "色号更新成功",
    )))
}

/// 删除产品色号
pub async fn delete_product_color(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((_product_id, color_id)): Path<(i32, i32)>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());
    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    product_service
        .delete_product_color(color_id, auth.user_id)
        .await?;
    Ok(Json(ApiResponse::success_with_message((), "色号删除成功")))
}

/// 批量创建色号
pub async fn batch_create_colors(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(product_id): Path<i32>,
    Json(req): Json<BatchCreateColorsRequest>,
) -> Result<Json<ApiResponse<Vec<product_color::Model>>>, AppError> {
    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());

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
    Ok(Json(ApiResponse::success_with_message(colors, &msg)))
}

// ========== 数据导入导出接口 ==========

use crate::utils::xlsx_export::{build_xlsx_response, XlsxTable};

/// 导出产品数据
pub async fn export_products(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<ExportProductsQuery>,
) -> Result<axum::response::Response, AppError> {
    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());

    let csv_data = product_service
        .export_products_to_csv(query.category_id, query.status, query.search)
        .await
        .map_err(|e| AppError::internal(format!("导出失败: {}", e)))?;

    // 规则 3：将 service 返回的 CSV 解析为 xlsx 表格
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_data.as_slice());
    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| AppError::internal(format!("CSV解析错误: {}", e)))?
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut rows: Vec<Vec<String>> = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| AppError::internal(format!("CSV解析错误: {}", e)))?;
        rows.push(record.iter().map(|s| s.to_string()).collect());
    }
    let table = XlsxTable {
        sheet_name: "产品列表".to_string(),
        headers,
        rows,
    };

    let filename = format!(
        "products_export_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );

    build_xlsx_response(&table, &filename)
}

/// 导入产品数据
pub async fn import_products(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<ImportProductsRequest>,
) -> Result<Json<ApiResponse<crate::utils::import_export::ImportResult>>, AppError> {
    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());

    let csv_bytes = req.csv_data.into_bytes();

    let result: crate::utils::import_export::ImportResult =
        product_service.import_products_from_csv(&csv_bytes).await?;

    let msg = if result.is_all_success() {
        format!("成功导入 {} 条产品数据", result.success_count)
    } else {
        format!(
            "导入完成：成功 {} 条，失败 {} 条，共 {} 条",
            result.success_count, result.error_count, result.total_count
        )
    };

    Ok(Json(ApiResponse::success_with_message(result, &msg)))
}

/// 获取产品导入模板
pub async fn get_product_import_template(
    State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<axum::response::Response, AppError> {
    let template_data = ProductService::generate_product_import_template()
        .map_err(|e| AppError::internal(format!("模板生成失败: {}", e)))?;

    // 规则 3：将 service 返回的 CSV 模板解析为 xlsx 表格
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(template_data.as_slice());
    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| AppError::internal(format!("CSV解析错误: {}", e)))?
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut rows: Vec<Vec<String>> = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| AppError::internal(format!("CSV解析错误: {}", e)))?;
        rows.push(record.iter().map(|s| s.to_string()).collect());
    }
    let table = XlsxTable {
        sheet_name: "产品导入模板".to_string(),
        headers,
        rows,
    };

    build_xlsx_response(&table, "product_import_template")
}
