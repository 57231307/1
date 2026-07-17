
use crate::middleware::auth_context::AuthContext;
use crate::models::sales_price;
use crate::services::sales_price_service::{
    CreateSalesPriceInput, SalesPriceService, UpdateSalesPriceInput,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::{ApiResponse, PaginatedResponse};
// V15 P0-S12/P0-S15 修复（Batch 475d）：导出端点使用水印版 xlsx 工具
use crate::utils::xlsx_export::{build_xlsx_response_with_watermark, WatermarkConfig, XlsxTable};
// V15 P0-S11：导出审计日志写入所需依赖
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use std::sync::Arc;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use tracing::info;

// V15 P0-S12 修复（Batch 475d）：派生 Clone，export_prices 需要 clone 后覆盖分页参数用于全量导出
#[derive(Debug, Clone, Deserialize)]
pub struct SalesPriceQuery {
    pub product_id: Option<i32>,
    pub customer_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ApprovePriceRequest {
    pub approved: bool,
    pub remark: Option<String>,
}

pub async fn list_prices(
    Query(params): Query<SalesPriceQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<sales_price::Model>>>, AppError> {
    info!("用户 {} 正在查询销售价格列表", auth.user_id);

    let service = SalesPriceService::new(state.db.clone());
    let query_params = crate::services::sales_price_service::SalesPriceQueryParams {
        product_id: params.product_id,
        customer_type: params.customer_type,
        status: params.status,
        page: params.page.unwrap_or(1).clamp(1, 1000),
        page_size: params.page_size.unwrap_or(10).clamp(1, 100),
    };

    let (prices, _total) = service.get_prices_list(query_params).await?;
    info!("销售价格列表查询成功，共 {} 条记录", prices.len());

    Ok(Json(ApiResponse::success(prices)))
}

pub async fn get_price(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<sales_price::Model>>, AppError> {
    info!("用户 {} 正在查询销售价格，ID: {}", auth.user_id, id);

    let service = SalesPriceService::new(state.db.clone());
    let price = service.get_price(id).await?;
    info!("销售价格查询成功，ID: {}", price.id);

    Ok(Json(ApiResponse::success(price)))
}

pub async fn create_price(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateSalesPriceInput>,
) -> Result<Json<ApiResponse<sales_price::Model>>, AppError> {
    info!(
        "用户 {} 正在创建销售价格，产品 ID: {}",
        auth.user_id, req.product_id
    );

    let service = SalesPriceService::new(state.db.clone());
    let price = service.create_price(req, auth.user_id).await?;
    info!("销售价格创建成功，ID: {}", price.id);

    Ok(Json(ApiResponse::success(price)))
}

pub async fn approve_price(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
    Json(req): Json<ApprovePriceRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // 批次 199 P1-6：真实接入请求体，原 stub 丢弃 _req 导致 approved=false 仍执行批准
    if !req.approved {
        return Err(AppError::validation(
            "审批拒绝请使用专用拒绝接口，本接口仅处理批准操作",
        ));
    }
    info!(
        "用户 {} 正在批准销售价格，ID: {}，备注: {:?}",
        auth.user_id, id, req.remark
    );

    let service = SalesPriceService::new(state.db.clone());
    service.approve_price(id, auth.user_id).await?;
    info!("销售价格批准成功，ID: {}，备注: {:?}", id, req.remark);

    Ok(Json(ApiResponse::success(())))
}

pub async fn get_price_history(
    State(state): State<AppState>,
    Path(product_id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<sales_price::Model>>>, AppError> {
    info!(
        "用户 {} 正在查询产品 {} 的价格历史",
        auth.user_id, product_id
    );

    let service = SalesPriceService::new(state.db.clone());
    let history = service.get_price_history(product_id).await?;
    info!("价格历史查询成功，共 {} 条记录", history.len());

    Ok(Json(ApiResponse::success(history)))
}

#[derive(Debug, Deserialize)]
pub struct StrategiesQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

pub async fn list_strategies(
    Query(params): Query<StrategiesQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<PaginatedResponse<sales_price::Model>>>, AppError> {
    info!("用户 {} 正在查询销售价格策略", auth.user_id);

    let page = params.page.unwrap_or(1).clamp(1, 1000) as u64; // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100) as u64;

    let service = SalesPriceService::new(state.db.clone());
    let (strategies, total) = service.list_strategies(page, page_size).await?;
    info!("销售价格策略查询成功，共 {} 条记录", strategies.len());

    Ok(Json(ApiResponse::success_paginated(
        strategies, total, page, page_size,
    )))
}

pub async fn update_price(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
    Json(req): Json<UpdateSalesPriceInput>,
) -> Result<Json<ApiResponse<sales_price::Model>>, AppError> {
    info!("用户 {} 正在更新销售价格，ID: {}", auth.user_id, id);

    let service = SalesPriceService::new(state.db.clone());
    let price = service.update_price(id, req).await?;
    info!("销售价格更新成功，ID: {}", price.id);

    Ok(Json(ApiResponse::success(price)))
}

pub async fn delete_price(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 正在删除销售价格，ID: {}", auth.user_id, id);

    let service = SalesPriceService::new(state.db.clone());
    // 批次 94 P2-10：传入真实操作人 user_id 用于审计日志
    service.delete_price(id, auth.user_id).await?;
    info!("销售价格删除成功，ID: {}", id);

    Ok(Json(ApiResponse::success(())))
}

/// GET /api/v1/erp/sales-prices/export - 导出销售价格列表（带水印 + 异步审计日志）
///
/// V15 P0-S12 修复（Batch 475d）：导出接入后端
/// - 注入水印（operator/exported_at/extra 含条数）
/// - 异步审计日志（OperationType::Export）
/// - 直接调 service.get_prices_list 取全量数据（page=1/page_size=10000）
/// - 不复用 list_prices handler 逻辑（保持单一职责）
pub async fn export_prices(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<SalesPriceQuery>,
) -> Result<axum::response::Response, AppError> {
    let service = SalesPriceService::new(state.db.clone());

    // V15 P0-S12 修复（Batch 475d）：导出全量数据
    let query_params = crate::services::sales_price_service::SalesPriceQueryParams {
        product_id: query.product_id,
        customer_type: query.customer_type,
        status: query.status,
        page: 1,
        page_size: 10000,
    };

    let (prices, _total) = service.get_prices_list(query_params).await?;
    let row_count = prices.len();

    // 序列化为 JSON 以统一字段处理
    let prices_json: Vec<serde_json::Value> = prices
        .into_iter()
        .map(|p| serde_json::to_value(p).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;

    // 构造 xlsx 表格数据（14 列）
    let headers: Vec<String> = vec![
        "ID".to_string(),
        "产品ID".to_string(),
        "客户ID".to_string(),
        "客户类型".to_string(),
        "价格".to_string(),
        "币种".to_string(),
        "单位".to_string(),
        "最小订购量".to_string(),
        "价格类型".to_string(),
        "价格等级".to_string(),
        "生效日期".to_string(),
        "到期日期".to_string(),
        "状态".to_string(),
        "创建时间".to_string(),
    ];
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(prices_json.len());
    for p in prices_json {
        let obj = p.as_object().ok_or_else(|| {
            AppError::internal("销售价格序列化失败：期望 JSON 对象")
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
            get_str("product_id"),
            get_str("customer_id"),
            get_str("customer_type"),
            get_str("price"),
            get_str("currency"),
            get_str("unit"),
            get_str("min_order_qty"),
            get_str("price_type"),
            get_str("price_level"),
            get_str("effective_date"),
            get_str("expiry_date"),
            get_str("status"),
            get_str("created_at"),
        ]);
    }

    let table = XlsxTable {
        sheet_name: "销售价格列表".to_string(),
        headers,
        rows,
    };

    let filename = format!(
        "sales_prices_export_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );

    // V15 P0-S11：导出审计日志写入（best-effort，异步不阻塞响应）
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("sales_price".to_string()),
        resource_id: None,
        resource_name: Some(format!("{}.xlsx", filename)),
        description: Some(format!(
            "用户 {} 导出销售价格列表（共 {} 条）",
            auth.username, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/sales-prices/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": row_count,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);

    // V15 P0-S15 修复（Batch 475d）：注入水印（操作员/导出时间/导出条数）
    let watermark = WatermarkConfig {
        operator: Some(auth.username.clone()),
        ip_address: None,
        exported_at: Some(chrono::Utc::now().to_rfc3339()),
        extra: Some(format!("销售价格导出（共 {} 条）", row_count)),
    };

    build_xlsx_response_with_watermark(&table, &filename, &watermark)
}
