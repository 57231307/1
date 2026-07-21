//! 生产订单 Handler
//!
//! 生产订单API端点

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

use sea_orm::{ActiveModelTrait, Set};

use crate::middleware::auth_context::AuthContext;
use crate::services::production_order_service::{
    CreateProductionOrderRequest, ProductionOrderQuery, ProductionOrderService,
    UpdateProductionOrderRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::messages::biz_msg;
use crate::utils::response::{ApiResponse, PaginatedResponse};
// V15 P0-S12/P0-S15 修复（Batch 475c）：导出端点使用水印版 xlsx 工具
use crate::utils::xlsx_export::{build_xlsx_response_with_watermark, WatermarkConfig, XlsxTable};
// V15 P0-S11：导出审计日志写入所需依赖
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use std::sync::Arc;

/// 创建生产订单请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateProductionOrderPayload {
    #[validate(length(min = 1, message = "订单编号不能为空"))]
    pub order_no: String,
    pub sales_order_id: Option<i32>,
    pub product_id: i32,
    pub planned_quantity: Decimal,
    pub planned_start_date: Option<chrono::NaiveDate>,
    pub planned_end_date: Option<chrono::NaiveDate>,
    pub priority: Option<i32>,
    pub work_center_id: Option<i32>,
    pub remarks: Option<String>,
}

/// 更新生产订单请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProductionOrderPayload {
    pub planned_quantity: Option<Decimal>,
    pub planned_start_date: Option<chrono::NaiveDate>,
    pub planned_end_date: Option<chrono::NaiveDate>,
    pub priority: Option<i32>,
    pub work_center_id: Option<i32>,
    pub remarks: Option<String>,
}

/// P1-2f 修复（批次 81 v1 复审）：更新生产订单状态请求 DTO
/// 替代 update_production_order_status 中的 Json<serde_json::Value>，
/// 提供强类型校验 + 状态白名单校验
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProductionOrderStatusDto {
    /// 状态：必填，必须命中白名单
    #[validate(custom(function = "validate_production_order_status"))]
    pub status: String,
    /// 实际产量：可选，字符串形式以便解析 Decimal
    pub actual_quantity: Option<String>,
}

/// P1-2f 修复（批次 81 v1 复审）：生产订单状态白名单校验
/// 仅允许以下状态值，避免任意字符串写入数据库
/// 白名单与 production_order_service::validate_status_transition 保持一致
fn validate_production_order_status(
    status: &str,
) -> Result<(), validator::ValidationError> {
    const ALLOWED: &[&str] = &[
        "DRAFT",
        "SCHEDULED",
        "IN_PROGRESS",
        "COMPLETED",
        "CANCELLED",
    ];
    if !ALLOWED.contains(&status) {
        return Err(validator::ValidationError::new(
            "生产订单状态不在允许的白名单内",
        ));
    }
    Ok(())
}

/// 生产订单响应
#[derive(Debug, Serialize)]
pub struct ProductionOrderResponse {
    pub id: i32,
    pub order_no: String,
    pub sales_order_id: Option<i32>,
    pub product_id: i32,
    pub planned_quantity: Decimal,
    pub actual_quantity: Option<Decimal>,
    pub planned_start_date: Option<chrono::NaiveDate>,
    pub planned_end_date: Option<chrono::NaiveDate>,
    pub status: String,
    pub priority: i32,
    pub work_center_id: Option<i32>,
    pub remarks: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

/// 生产订单列表查询参数
#[derive(Debug, Deserialize)]
pub struct ListProductionOrdersQuery {
    pub status: Option<String>,
    pub product_id: Option<i32>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 创建生产订单
pub async fn create_production_order(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(payload): Json<CreateProductionOrderPayload>,
) -> Result<Json<ApiResponse<ProductionOrderResponse>>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let service = ProductionOrderService::new(state.db.clone());

    let req = CreateProductionOrderRequest {
        order_no: Some(payload.order_no),
        sales_order_id: payload.sales_order_id,
        product_id: payload.product_id,
        planned_quantity: Some(payload.planned_quantity),
        planned_start_date: payload.planned_start_date,
        planned_end_date: payload.planned_end_date,
        priority: payload.priority,
        work_center_id: payload.work_center_id,
        remarks: payload.remarks,
        created_by: auth.user_id,
    };

    let model = service.create(req).await?;

    let response = ProductionOrderResponse {
        id: model.id,
        order_no: model.order_no,
        sales_order_id: model.sales_order_id,
        product_id: model.product_id,
        planned_quantity: model.planned_quantity,
        actual_quantity: model.actual_quantity,
        planned_start_date: model.planned_start_date,
        planned_end_date: model.planned_end_date,
        status: model.status,
        priority: model.priority,
        work_center_id: model.work_center_id,
        remarks: model.remarks,
        created_at: model.created_at,
        updated_at: model.updated_at,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 获取生产订单详情
pub async fn get_production_order(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<ProductionOrderResponse>>, AppError> {
    let service = ProductionOrderService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();

    let model = service
        .get_by_id(id, Some(&data_scope_ctx))
        .await?
        .ok_or_else(|| AppError::not_found("生产订单不存在"))?;

    let response = ProductionOrderResponse {
        id: model.id,
        order_no: model.order_no,
        sales_order_id: model.sales_order_id,
        product_id: model.product_id,
        planned_quantity: model.planned_quantity,
        actual_quantity: model.actual_quantity,
        planned_start_date: model.planned_start_date,
        planned_end_date: model.planned_end_date,
        status: model.status,
        priority: model.priority,
        work_center_id: model.work_center_id,
        remarks: model.remarks,
        created_at: model.created_at,
        updated_at: model.updated_at,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 获取生产订单列表
pub async fn list_production_orders(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<ListProductionOrdersQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<ProductionOrderResponse>>>, AppError> {
    let service = ProductionOrderService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文
    let data_scope_ctx = auth.to_data_scope_context();

    let query_params = ProductionOrderQuery {
        status: query.status,
        product_id: query.product_id,
        page: query.page.unwrap_or(1).clamp(1, 1000), // 批次 95 P3-3~8：分页 clamp 防 DoS
        page_size: query.page_size.unwrap_or(20).clamp(1, 100),
    };

    let (models, total) = service.list(query_params, Some(&data_scope_ctx)).await?;

    let responses: Vec<ProductionOrderResponse> = models
        .into_iter()
        .map(|model| ProductionOrderResponse {
            id: model.id,
            order_no: model.order_no,
            sales_order_id: model.sales_order_id,
            product_id: model.product_id,
            planned_quantity: model.planned_quantity,
            actual_quantity: model.actual_quantity,
            planned_start_date: model.planned_start_date,
            planned_end_date: model.planned_end_date,
            status: model.status,
            priority: model.priority,
            work_center_id: model.work_center_id,
            remarks: model.remarks,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
        .collect();

    Ok(Json(ApiResponse::success_paginated(
        responses,
        total,
        query.page.unwrap_or(1).clamp(1, 1000), // 批次 95 P3-3~8：分页 clamp 防 DoS
        query.page_size.unwrap_or(20).clamp(1, 100),
    )))
}

/// 更新生产订单
pub async fn update_production_order(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateProductionOrderPayload>,
) -> Result<Json<ApiResponse<ProductionOrderResponse>>, AppError> {
    let service = ProductionOrderService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——更新前先校验资源归属（复用 P0-S01 的 get_by_id + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    let _ = service.get_by_id(id, Some(&data_scope_ctx)).await?;

    let req = UpdateProductionOrderRequest {
        planned_quantity: payload.planned_quantity,
        planned_start_date: payload.planned_start_date,
        planned_end_date: payload.planned_end_date,
        priority: payload.priority,
        work_center_id: payload.work_center_id,
        remarks: payload.remarks,
    };

    let model = service.update(id, req).await?;

    let response = ProductionOrderResponse {
        id: model.id,
        order_no: model.order_no,
        sales_order_id: model.sales_order_id,
        product_id: model.product_id,
        planned_quantity: model.planned_quantity,
        actual_quantity: model.actual_quantity,
        planned_start_date: model.planned_start_date,
        planned_end_date: model.planned_end_date,
        status: model.status,
        priority: model.priority,
        work_center_id: model.work_center_id,
        remarks: model.remarks,
        created_at: model.created_at,
        updated_at: model.updated_at,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 审批请求
#[derive(Debug, Deserialize)]
pub struct ApprovalRequest {
    pub approved: bool,
    pub opinion: Option<String>,
}

/// 提交生产订单审批
pub async fn submit_for_approval(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<ProductionOrderResponse>>, AppError> {
    let service = ProductionOrderService::new(state.db.clone());
    let model = service
        .submit_for_approval(id, auth.user_id, &auth.username)
        .await?;

    let response = ProductionOrderResponse {
        id: model.id,
        order_no: model.order_no,
        sales_order_id: model.sales_order_id,
        product_id: model.product_id,
        planned_quantity: model.planned_quantity,
        actual_quantity: model.actual_quantity,
        planned_start_date: model.planned_start_date,
        planned_end_date: model.planned_end_date,
        status: model.status,
        priority: model.priority,
        work_center_id: model.work_center_id,
        remarks: model.remarks,
        created_at: model.created_at,
        updated_at: model.updated_at,
    };

    Ok(Json(ApiResponse::success_with_message(
        response,
        "已提交审批",
    )))
}

/// 审批生产订单
pub async fn approve_production_order(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<ApprovalRequest>,
) -> Result<Json<ApiResponse<ProductionOrderResponse>>, AppError> {
    let service = ProductionOrderService::new(state.db.clone());
    let model = service
        .approve_order(id, auth.user_id, &auth.username, req.approved, req.opinion)
        .await?;

    let response = ProductionOrderResponse {
        id: model.id,
        order_no: model.order_no,
        sales_order_id: model.sales_order_id,
        product_id: model.product_id,
        planned_quantity: model.planned_quantity,
        actual_quantity: model.actual_quantity,
        planned_start_date: model.planned_start_date,
        planned_end_date: model.planned_end_date,
        status: model.status,
        priority: model.priority,
        work_center_id: model.work_center_id,
        remarks: model.remarks,
        created_at: model.created_at,
        updated_at: model.updated_at,
    };

    let message = if req.approved {
        biz_msg::APPROVE_OK
    } else {
        "审批拒绝"
    };
    Ok(Json(ApiResponse::success_with_message(response, message)))
}

/// 删除生产订单（软删除）
pub async fn delete_production_order(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let service = ProductionOrderService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——删除前先校验资源归属（复用 P0-S01 的 get_by_id + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    let _ = service.get_by_id(id, Some(&data_scope_ctx)).await?;
    service.delete(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success("生产订单已取消".to_string())))
}

/// 更新生产进度请求
#[derive(Debug, Deserialize)]
pub struct UpdateProgressRequest {
    pub actual_quantity: Option<Decimal>,
    pub remarks: Option<String>,
}

/// 更新生产订单进度
pub async fn update_production_progress(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateProgressRequest>,
) -> Result<Json<ApiResponse<ProductionOrderResponse>>, AppError> {
    let service = ProductionOrderService::new(state.db.clone());
    let model = service
        .get_by_id(id, None)
        .await?
        .ok_or_else(|| AppError::not_found("生产订单不存在"))?;

    let mut active_model: crate::models::production_order::ActiveModel = model.into();
    if let Some(qty) = payload.actual_quantity {
        active_model.actual_quantity = Set(Some(qty));
    }
    if let Some(remarks) = payload.remarks {
        active_model.remarks = Set(Some(remarks));
    }
    active_model.updated_at = Set(Utc::now());

    let updated = active_model.update(&*state.db).await?;

    let response = ProductionOrderResponse {
        id: updated.id,
        order_no: updated.order_no,
        sales_order_id: updated.sales_order_id,
        product_id: updated.product_id,
        planned_quantity: updated.planned_quantity,
        actual_quantity: updated.actual_quantity,
        planned_start_date: updated.planned_start_date,
        planned_end_date: updated.planned_end_date,
        status: updated.status,
        priority: updated.priority,
        work_center_id: updated.work_center_id,
        remarks: updated.remarks,
        created_at: updated.created_at,
        updated_at: updated.updated_at,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 获取生产订单操作日志
///
/// 批次 132 v9 复审 P1：原返回固定空列表 {logs: []}，
/// 现真实查询 audit_logs 表，按 resource_id = order_id 过滤，按 created_at 倒序返回。
pub async fn get_production_order_logs(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ProductionOrderService::new(state.db.clone());
    let logs = service.get_order_logs(id).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "order_id": id,
        "logs": logs,
        "total": logs.len()
    }))))
}

/// 更新生产订单状态
pub async fn update_production_order_status(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateProductionOrderStatusDto>,
) -> Result<Json<ApiResponse<ProductionOrderResponse>>, AppError> {
    // P1-2f 修复（批次 81 v1 复审）：强类型 DTO + validator + 状态白名单 替代 Json<Value>
    payload
        .validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let service = ProductionOrderService::new(state.db.clone());

    let actual_quantity = payload
        .actual_quantity
        .as_deref()
        .and_then(|s| s.parse::<Decimal>().ok());

    let model = service
        .update_status(id, payload.status, actual_quantity)
        .await?;

    let response = ProductionOrderResponse {
        id: model.id,
        order_no: model.order_no,
        sales_order_id: model.sales_order_id,
        product_id: model.product_id,
        planned_quantity: model.planned_quantity,
        actual_quantity: model.actual_quantity,
        planned_start_date: model.planned_start_date,
        planned_end_date: model.planned_end_date,
        status: model.status,
        priority: model.priority,
        work_center_id: model.work_center_id,
        remarks: model.remarks,
        created_at: model.created_at,
        updated_at: model.updated_at,
    };

    Ok(Json(ApiResponse::success(response)))
}

// ========== 数据导出接口 ==========

/// 生产订单导出表头（14 列）
fn production_order_export_headers() -> Vec<String> {
    vec![
        "ID".to_string(),
        "订单号".to_string(),
        "销售订单ID".to_string(),
        "产品ID".to_string(),
        "计划数量".to_string(),
        "实际数量".to_string(),
        "计划开始日期".to_string(),
        "计划结束日期".to_string(),
        "状态".to_string(),
        "优先级".to_string(),
        "工作中心ID".to_string(),
        "备注".to_string(),
        "创建时间".to_string(),
        "更新时间".to_string(),
    ]
}

/// 将生产订单 model 转换为响应结构（与 list_production_orders handler 字段一致）
fn convert_orders_to_responses(
    models: Vec<crate::models::production_order::Model>,
) -> Vec<ProductionOrderResponse> {
    models
        .into_iter()
        .map(|model| ProductionOrderResponse {
            id: model.id,
            order_no: model.order_no,
            sales_order_id: model.sales_order_id,
            product_id: model.product_id,
            planned_quantity: model.planned_quantity,
            actual_quantity: model.actual_quantity,
            planned_start_date: model.planned_start_date,
            planned_end_date: model.planned_end_date,
            status: model.status,
            priority: model.priority,
            work_center_id: model.work_center_id,
            remarks: model.remarks,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
        .collect()
}

/// 从单条生产订单响应构建 xlsx 行
fn build_production_order_row(r: ProductionOrderResponse) -> Vec<String> {
    vec![
        r.id.to_string(),
        r.order_no,
        r.sales_order_id.map_or(String::new(), |v| v.to_string()),
        r.product_id.to_string(),
        r.planned_quantity.to_string(),
        r.actual_quantity.map_or(String::new(), |v| v.to_string()),
        r.planned_start_date.map_or(String::new(), |d| d.to_string()),
        r.planned_end_date.map_or(String::new(), |d| d.to_string()),
        r.status,
        r.priority.to_string(),
        r.work_center_id.map_or(String::new(), |v| v.to_string()),
        r.remarks.unwrap_or_default(),
        r.created_at.to_string(),
        r.updated_at.to_string(),
    ]
}

/// 构造生产订单 xlsx 表格
fn build_production_orders_table(responses: Vec<ProductionOrderResponse>) -> XlsxTable {
    let headers = production_order_export_headers();
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(responses.len());
    for r in responses {
        rows.push(build_production_order_row(r));
    }
    XlsxTable {
        sheet_name: "生产订单".to_string(),
        headers,
        rows,
    }
}

/// 异步记录生产订单导出操作（审计自身）
fn record_production_orders_export_audit(
    state: &AppState,
    auth: &AuthContext,
    row_count: usize,
    query: &ListProductionOrdersQuery,
    filename: &str,
) {
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("production_order".to_string()),
        resource_id: None,
        resource_name: Some(format!("{}.xlsx", filename)),
        description: Some(format!(
            "用户 {} 导出生产订单（共 {} 条）",
            auth.username, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/production-orders/orders/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": row_count,
            "status_filter": query.status,
            "product_id_filter": query.product_id,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);
}

/// 导出生产订单列表
///
/// V15 P0-S12/P0-S15 修复（Batch 475c）：导出注入水印 + 异步审计日志
///
/// 规则 3：导出统一使用 xlsx 格式
/// V15 P0-S11：导出审计日志写入（best-effort，异步不阻塞响应）
/// V15 P0-S15：水印行在 xlsx 第 0 行（合并所有列），标题行下移到第 1 行，数据行从第 2 行起
///
/// 重要：生产订单表有行级数据权限（V15 P0-S01），必须调 `to_data_scope_context`
/// + `service.list(query, Some(&data_scope_ctx))` 保证数据隔离
pub async fn export_production_orders(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<ListProductionOrdersQuery>,
) -> Result<axum::response::Response, AppError> {
    let service = ProductionOrderService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文（导出与列表查询保持一致的数据隔离）
    let data_scope_ctx = auth.to_data_scope_context();

    // V15 P0-S12 修复（Batch 475c）：导出全量数据（page=1/page_size=10000）
    let query_params = ProductionOrderQuery {
        status: query.status.clone(),
        product_id: query.product_id,
        page: 1,
        page_size: 10000,
    };

    let (models, _total) = service
        .list(query_params, Some(&data_scope_ctx))
        .await?;

    let row_count = models.len();
    let responses = convert_orders_to_responses(models);
    let table = build_production_orders_table(responses);
    let filename = format!(
        "production_orders_export_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    record_production_orders_export_audit(&state, &auth, row_count, &query, &filename);

    // V15 P0-S15 修复（Batch 475c）：注入水印（操作员/导出时间/导出条数）
    let watermark = WatermarkConfig {
        operator: Some(auth.username.clone()),
        ip_address: None,
        exported_at: Some(chrono::Utc::now().to_rfc3339()),
        extra: Some(format!("生产订单导出（共 {} 条）", row_count)),
    };

    build_xlsx_response_with_watermark(&table, &filename, &watermark)
}
