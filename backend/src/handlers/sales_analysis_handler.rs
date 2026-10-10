use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
// V15 P0-S11：导出审计日志写入所需依赖
use crate::models::audit_log::{OperationType, Severity};
use crate::models::sales_analysis;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::sales_analysis_service::{
    CreateSalesTargetInput, CustomerRankingParams, ExportParams, ProductRankingParams,
    SalesAnalysisService, SalesTargetDto, SalesTrendPoint, SalesTrendQueryParams,
    UpdateSalesTargetRequest,
};
use crate::utils::ApiResponse;
use crate::utils::error::AppError;
use crate::utils::xlsx_export::xlsx_response;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct SalesStatisticQuery {
    pub statistic_type: Option<String>,
    pub period: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct TrendQuery {
    /// 分桶粒度：`day`/`week`/`month`/`quarter`/`year`（与 BI 聚合实现点同词表）。
    /// 缺省（键不存在，或空串被 `normalize_empty_query_params` 剔除）＝ `month`；
    /// 非法值不 400：由 service 层回落 `month` 并 `tracing::warn` 留痕（不静默）。
    pub granularity: Option<String>,
    /// 起始日期 `YYYY-MM-DD`。与 `end_date` 成对出现；只给一半或形态非法 → 400 `VALIDATION_ERROR`；
    /// 两者缺省 → 按粒度回看 12 个桶。
    pub start_date: Option<String>,
    /// 结束日期 `YYYY-MM-DD`。`end_date < start_date` → 400 `VALIDATION_ERROR`（机器码判点在复用聚合）。
    pub end_date: Option<String>,
    /// 桶键等值过滤（如月粒度 `2026-08`）。缺省（键不存在，或空串被 `normalize_empty_query_params`
    /// 剔除）＝ **不过滤，返回窗口内全部桶**。该 `Option` 缺省不过滤语义沿用已提交的契约，
    /// 不退回必填，也不得退化成按空串过滤的恒 0 行假过滤。
    pub period: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct RankingQuery {
    pub period: Option<String>,
    pub limit: Option<i64>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct TargetQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

pub async fn list_statistics(
    Query(params): Query<SalesStatisticQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<sales_analysis::Model>>>, AppError> {
    info!("用户 {} 正在查询销售统计列表", auth.user_id);

    let service = SalesAnalysisService::new(state.db.clone());
    let query_params = crate::services::sales_analysis_service::SalesStatisticQueryParams {
        statistic_type: params.statistic_type,
        period: params.period,
        page: params.page.unwrap_or(1).clamp(1, 1000),
        page_size: params.page_size.unwrap_or(10).clamp(1, 100),
    };

    let (statistics, _total) = service.get_statistics_list(query_params).await?;
    info!("销售统计列表查询成功，共 {} 条记录", statistics.len());

    Ok(Json(ApiResponse::success(statistics)))
}

/// 销售趋势查询 —— `GET /api/v1/erp/crm/sales-analysis/trends`（别名 `/trend` 挂同一 handler）
///
/// 做什么：现算 `sales_orders`，按 `granularity` 分桶返回时间序列（聚合本体复用
/// `BiAnalysisService::sales_by_time`，出参行类型 `SalesTrendPoint`，`total_amount→amount`
/// 的键名映射只发生在 service 的唯一映射点）。不再读 `sales_statistics`——该表对实际销售
/// 零写入方（仅 target 行），读它无论怎么过滤都结构性恒空。
/// 谁调：前端销售分析页趋势卡片（`api/sales-analysis.ts::getSalesTrendData`）。
/// 入参：`granularity`（day/week/month/quarter/year，缺省 `month`，非法值回落 `month` 并
/// `tracing::warn` 留痕，不 400）；`start_date`/`end_date`（`YYYY-MM-DD` 且成对，缺省按粒度
/// 回看 12 桶；`end_date < start_date` → 400 + `VALIDATION_ERROR`）；
/// `period`（桶键等值过滤，`Option` 缺省不过滤语义沿用已提交契约，不退回必填）。
/// 返回什么：`ApiResponse<Vec<SalesTrendPoint>>`，`data` 恒为数组；窗口内真无销售 ⇒
/// `200 + []`（真空，与旧"恒空壳表"区分）；桶键升序（定宽前缀，字符串升序＝时间升序）。
/// 存在哪：读 PostgreSQL `sales_orders`（复用点自带数据范围注入、状态排除门与 5min TTL 缓存）。
pub async fn get_trends(
    Query(params): Query<TrendQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<SalesTrendPoint>>>, AppError> {
    info!(
        "用户 {} 正在查询销售趋势，粒度：{:?}，窗口：{:?}..={:?}，桶键过滤：{:?}",
        auth.user_id, params.granularity, params.start_date, params.end_date, params.period
    );

    let service = SalesAnalysisService::new(state.db.clone());
    let trends = service
        .get_trends(
            SalesTrendQueryParams {
                granularity: params.granularity,
                start_date: params.start_date,
                end_date: params.end_date,
                period: params.period,
            },
            auth.to_data_scope_context(),
            state.cache.clone(),
        )
        .await?;
    info!("销售趋势查询成功，共 {} 个桶", trends.len());

    Ok(Json(ApiResponse::success(trends)))
}

pub async fn get_rankings(
    Query(params): Query<RankingQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<sales_analysis::Model>>>, AppError> {
    info!("用户 {} 正在查询销售排名", auth.user_id);

    let service = SalesAnalysisService::new(state.db.clone());
    let rankings = service
        .get_rankings(
            params.period.as_deref(),
            params.limit.unwrap_or(10).clamp(1, 100),
        )
        .await?;
    info!("销售排名查询成功，共 {} 条记录", rankings.len());

    Ok(Json(ApiResponse::success(rankings)))
}

pub async fn get_targets(
    Query(params): Query<TargetQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<sales_analysis::Model>>>, AppError> {
    info!("用户 {} 正在查询销售目标", auth.user_id);

    let service = SalesAnalysisService::new(state.db.clone());
    let (targets, _total) = service
        .get_targets(
            params.page.unwrap_or(1).clamp(1, 1000),
            params.page_size.unwrap_or(10).clamp(1, 100),
        )
        .await?;
    info!("销售目标查询成功，共 {} 条记录", targets.len());

    Ok(Json(ApiResponse::success(targets)))
}

pub async fn create_target(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateSalesTargetInput>,
) -> Result<Json<ApiResponse<sales_analysis::Model>>, AppError> {
    info!("用户 {} 正在创建销售目标", auth.user_id);

    let service = SalesAnalysisService::new(state.db.clone());
    let target = service.create_target(req, auth.user_id).await?;
    info!("销售目标创建成功，ID: {}", target.id);

    Ok(Json(ApiResponse::success(target)))
}

/// GET /api/v1/erp/sales-analysis/stats - 销售概览统计
pub async fn get_stats(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("正在获取销售概览统计");
    let service = SalesAnalysisService::new(state.db.clone());
    let stats = service.get_overview_stats().await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(stats)?)))
}

/// GET /api/v1/erp/sales-analysis/product-ranking - 产品销售排名
pub async fn get_product_ranking(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ProductRankingParams>,
) -> Result<
    Json<ApiResponse<Vec<crate::services::sales_analysis_service::ProductRankingItem>>>,
    AppError,
> {
    info!("正在获取产品销售排名");
    let service = SalesAnalysisService::new(state.db.clone());
    let list = service.product_ranking(params).await?;
    Ok(Json(ApiResponse::success(list)))
}

/// GET /api/v1/erp/sales-analysis/customer-ranking - 客户销售排名
pub async fn get_customer_ranking(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<CustomerRankingParams>,
) -> Result<
    Json<ApiResponse<Vec<crate::services::sales_analysis_service::CustomerRankingItem>>>,
    AppError,
> {
    info!("正在获取客户销售排名");
    let service = SalesAnalysisService::new(state.db.clone());
    let list = service.customer_ranking(params).await?;
    Ok(Json(ApiResponse::success(list)))
}

/// PUT /api/v1/erp/sales-analysis/targets/:period - 更新销售目标
pub async fn update_sales_target(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(period): Path<String>,
    Json(req): Json<UpdateSalesTargetRequest>,
) -> Result<Json<ApiResponse<SalesTargetDto>>, AppError> {
    info!("正在更新销售目标，周期：{}", period);
    let service = SalesAnalysisService::new(state.db.clone());
    let target = service.update_target(&period, req).await?;
    Ok(Json(ApiResponse::success(target)))
}

/// GET /api/v1/erp/sales-analysis/export - 导出销售分析报告；v11 批次 151 P2-A
/// service.export_report 已直接返回 xlsx 字节流， handler 只需构造下载响应，无需 CSV→xlsx 中间转换。
pub async fn export_analysis(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<ExportParams>,
) -> Result<axum::response::Response, AppError> {
    info!("正在导出销售分析报告");
    // V15 P0-S11：提前 clone 查询条件用于审计日志（避免 service 调用 move 后 borrow of moved value）
    let audit_period = params.period.clone();
    let audit_format = params.format.clone();

    let service = SalesAnalysisService::new(state.db.clone());
    let bytes = service.export_report(params).await?;

    let filename = format!(
        "sales_analysis_export_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );

    // V15 P0-S11：导出审计日志写入（best-effort，异步不阻塞响应）
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("sales_analysis".to_string()),
        resource_id: None,
        resource_name: Some(format!("{}.xlsx", filename)),
        description: Some(format!(
            "用户 {} 导出销售分析报告（大小：{} 字节）",
            auth.username,
            bytes.len()
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/sales-analysis/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "size": bytes.len(),
            "period_filter": audit_period,
            "format_filter": audit_format,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);

    Ok(xlsx_response(bytes, &filename))
}
