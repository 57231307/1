
use crate::middleware::auth_context::AuthContext;
use crate::models::audit_log::{OperationType, Severity};
use crate::models::fixed_asset;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::fixed_asset_service::{
    CreateAssetRequest, DepreciationResult, DisposalRequest, FixedAssetService,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::ApiResponse;
use crate::utils::xlsx_export::{build_xlsx_response_with_watermark, WatermarkConfig, XlsxTable};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use validator::Validate;

/// P1-2j 修复（批次 81 v1 复审）：更新固定资产请求 DTO
/// 替代 update_asset 中的 Json<serde_json::Value>，提供强类型校验
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateAssetDto {
    /// 资产名称：可选
    #[validate(length(max = 100, message = "资产名称长度不能超过100字符"))]
    pub asset_name: Option<String>,
    /// 资产类别：可选
    #[validate(length(max = 50, message = "资产类别长度不能超过50字符"))]
    pub asset_category: Option<String>,
    /// 规格型号：可选
    pub specification: Option<String>,
    /// 使用地点：可选
    pub use_location: Option<String>,
}

/// 资产查询参数 DTO
// V15 P0-S12 修复（Batch 475e）：派生 Clone，export_assets 需要 clone 后覆盖分页参数用于全量导出
#[derive(Debug, Clone, Deserialize)]
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
        // P2 2-12 修复：分页默认值统一为 page=1，原 unwrap_or_default() 默认 0 不符合分页语义
        page: params.page.unwrap_or(1).clamp(1, 10000),
        page_size: params.page_size.unwrap_or(10).clamp(1, 100),
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

/// 查询指定资产的折旧历史记录
///
/// v3 复审 P1-3：折旧记录查询 API
/// GET /api/v1/erp/fixed-assets/:id/depreciation-records
pub async fn list_depreciation_records(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<crate::models::fixed_asset_depreciation_record::Model>>>, AppError>
{
    info!(
        "用户 {} 正在查询资产 {} 的折旧历史记录",
        auth.user_id, id
    );

    let service = FixedAssetService::new(state.db.clone());
    let records = service.list_depreciation_records(id).await?;
    info!(
        "资产 {} 折旧记录查询成功，共 {} 条记录",
        id,
        records.len()
    );

    Ok(Json(ApiResponse::success(records)))
}

/// 查询资产处置记录列表
///
/// v3 复审 P1-8：处置记录查询 API
/// GET /api/v1/erp/fixed-assets/disposals
pub async fn list_disposals(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<crate::models::fixed_asset_disposal::Model>>>, AppError> {
    info!("用户 {} 正在查询资产处置记录列表", auth.user_id);

    let service = FixedAssetService::new(state.db.clone());
    let disposals = service.list_disposals().await?;
    info!("资产处置记录查询成功，共 {} 条记录", disposals.len());

    Ok(Json(ApiResponse::success(disposals)))
}

/// PUT /api/v1/erp/fixed-assets/:id - 更新固定资产
pub async fn update_asset(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateAssetDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 更新固定资产: ID={}", auth.username, id);

    // P1-2j 修复（批次 81 v1 复审）：强类型 DTO + validator 替代 Json<Value>
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let service = FixedAssetService::new(state.db.clone());

    // 获取现有资产
    let mut asset = service.get_by_id(id).await?;

    // 更新字段
    if let Some(name) = req.asset_name {
        asset.asset_name = name;
    }
    if let Some(category) = req.asset_category {
        asset.asset_category = Some(category);
    }
    if let Some(spec) = req.specification {
        asset.specification = Some(spec);
    }
    if let Some(location) = req.use_location {
        asset.use_location = Some(location);
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
    // 批次 94 P2-9 修复：移除 DTO 中的 user_id 字段，改用 auth.user_id（鉴权上下文真实操作人）
    let results = service
        .batch_calculate_depreciation(req.asset_ids, req.calculation_date, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success(results)))
}

/// 折旧计算请求
// 批次 94 P2-9 修复：移除 user_id 字段（鉴权信息应来自 AuthContext 而非请求体，避免伪造）
#[derive(Debug, Deserialize)]
pub struct BatchDepreciateRequest {
    pub asset_ids: Vec<i32>,
    pub calculation_date: String,
}

/// GET /api/v1/erp/fixed-assets/export - 导出固定资产列表（带水印 + 异步审计日志）
///
/// V15 P0-S12 修复（Batch 475e）：导出接入后端
/// - 注入水印（operator/exported_at/extra 含条数）
/// - 异步审计日志（OperationType::Export）
/// - 直接调 service.get_list 取全量数据
pub async fn export_assets(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<AssetQuery>,
) -> Result<axum::response::Response, AppError> {
    let service = FixedAssetService::new(state.db.clone());

    let query_params = crate::services::fixed_asset_service::AssetQueryParams {
        keyword: query.keyword,
        status: query.status,
        asset_category: query.asset_category,
        page: 1,
        page_size: 10000,
    };

    let (assets, _total) = service.get_list(query_params).await?;
    let row_count = assets.len();

    let assets_json: Vec<serde_json::Value> = assets
        .into_iter()
        .map(|i| serde_json::to_value(i).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;

    let headers: Vec<String> = vec![
        "ID".to_string(),
        "资产编号".to_string(),
        "资产名称".to_string(),
        "资产类别".to_string(),
        "规格型号".to_string(),
        "存放地点".to_string(),
        "原值".to_string(),
        "使用年限".to_string(),
        "折旧方法".to_string(),
        "购置日期".to_string(),
        "启用日期".to_string(),
        "状态".to_string(),
        "备注".to_string(),
        "创建时间".to_string(),
    ];
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(assets_json.len());
    for i in assets_json {
        let obj = i.as_object().ok_or_else(|| {
            AppError::internal("固定资产序列化失败：期望 JSON 对象")
        })?;
        let get_str = |key: &str| -> String {
            obj.get(key)
                .map(|v| {
                    if v.is_null() {
                        String::new()
                    } else if v.is_string() {
                        v.as_str().unwrap_or("").to_string()
                    } else {
                        v.to_string()
                    }
                })
                .unwrap_or_default()
        };
        rows.push(vec![
            get_str("id"),
            get_str("asset_no"),
            get_str("asset_name"),
            get_str("asset_category"),
            get_str("specification"),
            get_str("location"),
            get_str("original_value"),
            get_str("useful_life"),
            get_str("depreciation_method"),
            get_str("purchase_date"),
            get_str("put_in_date"),
            get_str("status"),
            get_str("remark"),
            get_str("created_at"),
        ]);
    }

    let table = XlsxTable {
        sheet_name: "固定资产".to_string(),
        headers,
        rows,
    };

    let filename = format!(
        "fixed_assets_export_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );

    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("fixed_asset".to_string()),
        resource_id: None,
        resource_name: Some(format!("{}.xlsx", filename)),
        description: Some(format!(
            "用户 {} 导出固定资产列表（共 {} 条）",
            auth.username, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/fixed-assets/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": row_count,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);

    let watermark = WatermarkConfig {
        operator: Some(auth.username.clone()),
        ip_address: None,
        exported_at: Some(chrono::Utc::now().to_rfc3339()),
        extra: Some(format!("固定资产导出（共 {} 条）", row_count)),
    };

    build_xlsx_response_with_watermark(&table, &filename, &watermark)
}
