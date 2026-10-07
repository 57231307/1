//! 生产订单 Handler
//!
//! 生产订单API端点

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::Utc;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

use sea_orm::{ActiveModelTrait, EntityTrait, Set};

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::production_order_service::{
    CreateProductionOrderRequest, ProductionOrderDto, ProductionOrderQuery, ProductionOrderService,
    UpdateProductionOrderRequest,
};
use crate::utils::error::AppError;
use crate::utils::messages::biz_msg;
use crate::utils::response::{ApiResponse, PaginatedResponse};
// V15 P0-S12/P0-S15 修复（Batch 475c）：导出端点使用水印版 xlsx 工具
use crate::utils::xlsx_export::{WatermarkConfig, XlsxTable, build_xlsx_response_with_watermark};
// V15 P0-S11：导出审计日志写入所需依赖
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use std::sync::Arc;

/// 创建生产订单请求
///
/// 单号禁手输（缺陷3）：DTO 上**不存在** order_no 字段，单号一律服务端取号
/// （`PO{YYYYMMDD}{3位流水}`，DocumentNumberGenerator），前端已同步不再传该字段。
/// 旧客户端仍携带 order_no 时经 flatten 残差映射识别为非空字符串 → warn 日志（不静默、
/// 不透传服务层）。本 handler 未采用 `Json<Value>` 全量透传形态，仅用 flatten 收集未声明键。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct CreateProductionOrderPayload {
    pub sales_order_id: Option<i32>,
    pub product_id: i32,
    pub planned_quantity: Decimal,
    pub planned_start_date: Option<chrono::NaiveDate>,
    pub planned_end_date: Option<chrono::NaiveDate>,
    pub priority: Option<i32>,
    pub work_center_id: Option<i32>,
    pub remarks: Option<String>,
    /// 未声明键的残差收集（serde flatten）：仅用于识别被禁的 order_no 手输并 warn，
    /// 其中任何内容都不参与建单。
    #[serde(flatten)]
    forwarded_unknown_fields: std::collections::HashMap<String, serde_json::Value>,
}

/// JSON 三态反序列化适配器（RFC 7386 JSON Merge Patch 的"键缺席 ≠ 显式 null"语义所需）。
///
/// 为何需要：serde_json 对 `Option<Option<T>>` 的默认反序列化在遇到 JSON null 时
/// 直接调 visit_none()，把"显式 null"塌成外层 `None`，与"键缺席"不可区分。
/// 本适配器把字段先按内层 `Option<T>` 反序列化再包一层：
/// 键缺席（配合 `#[serde(default)]`）= `None`、显式 null = `Some(None)`、有值 = `Some(Some(v))`。
/// 形态与 handlers/department_handler.rs 中同名私有适配器一致（跨域合并到共享工具需动 utils，超本波授权范围）。
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

/// 更新生产订单请求
///
/// 字段三态语义（对齐 handlers/department_handler.rs::UpdateDepartmentRequest）：
/// 键缺席=保持原值、显式 `null`=清空为 NULL（仅 DB 可空列）、有值=覆盖。
/// NOT NULL 列（planned_quantity/priority，m0007 DDL）显式 null 由 service 入口拒绝。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProductionOrderPayload {
    /// 计划数量：NOT NULL（m0007:78）——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub planned_quantity: Option<Option<Decimal>>,
    /// 计划开始日期：DB 可空（m0007:80）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub planned_start_date: Option<Option<chrono::NaiveDate>>,
    /// 计划结束日期：DB 可空（m0007:81）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub planned_end_date: Option<Option<chrono::NaiveDate>>,
    /// 优先级：NOT NULL DEFAULT 5（m0007:84）——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub priority: Option<Option<i32>>,
    /// 工作中心 ID：DB 可空（m0007:85）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub work_center_id: Option<Option<i32>>,
    /// 备注：DB 可空 TEXT（m0007:86）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub remarks: Option<Option<String>>,
}

/// P1-2f 修复（批次 81 v1 复审）：更新生产订单状态请求 DTO 替代 update_production_order_status
/// 中的 Json<serde_json::Value>， 提供强类型校验 + 状态白名单校验
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProductionOrderStatusDto {
    /// 状态：必填，必须命中白名单
    #[validate(custom(function = "validate_production_order_status"))]
    pub status: String,
    /// 实际产量：可选，字符串形式以便解析 Decimal
    pub actual_quantity: Option<String>,
}

/// P1-2f 修复（批次 81 v1 复审）：生产订单状态白名单校验 仅允许以下状态值，避免任意字符串写入数据库 白名单与
/// production_order_service::validate_status_transition 保持一致
fn validate_production_order_status(status: &str) -> Result<(), validator::ValidationError> {
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
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Serialize)]
pub struct ProductionOrderResponse {
    pub id: i32,
    pub order_no: String,
    pub sales_order_id: Option<i32>,
    pub product_id: i32,
    // 产品名称：仅列表/详情经 LEFT JOIN products 富化后填充；其余写操作响应无 JOIN，为 None。
    pub product_name: Option<String>,
    pub planned_quantity: Decimal,
    pub actual_quantity: Option<Decimal>,
    pub planned_start_date: Option<chrono::NaiveDate>,
    pub planned_end_date: Option<chrono::NaiveDate>,
    pub actual_start_date: Option<chrono::NaiveDate>,
    pub actual_end_date: Option<chrono::NaiveDate>,
    pub status: String,
    pub priority: i32,
    pub work_center_id: Option<i32>,
    pub remarks: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

/// 由实体 Model 构造响应（无 JOIN 的写操作路径：product_name 无来源，置 None）。
fn response_from_model(m: crate::models::production_order::Model) -> ProductionOrderResponse {
    ProductionOrderResponse {
        id: m.id,
        order_no: m.order_no,
        sales_order_id: m.sales_order_id,
        product_id: m.product_id,
        product_name: None,
        planned_quantity: m.planned_quantity,
        actual_quantity: m.actual_quantity,
        planned_start_date: m.planned_start_date,
        planned_end_date: m.planned_end_date,
        actual_start_date: m.actual_start_date,
        actual_end_date: m.actual_end_date,
        status: m.status,
        priority: m.priority,
        work_center_id: m.work_center_id,
        remarks: m.remarks,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

/// 由富化 DTO 构造响应（列表/详情路径：product_name 来自 §5 范式 LEFT JOIN products）。
fn response_from_dto(d: ProductionOrderDto) -> ProductionOrderResponse {
    ProductionOrderResponse {
        id: d.id,
        order_no: d.order_no,
        sales_order_id: d.sales_order_id,
        product_id: d.product_id,
        product_name: d.product_name,
        planned_quantity: d.planned_quantity,
        actual_quantity: d.actual_quantity,
        planned_start_date: d.planned_start_date,
        planned_end_date: d.planned_end_date,
        actual_start_date: d.actual_start_date,
        actual_end_date: d.actual_end_date,
        status: d.status,
        priority: d.priority,
        work_center_id: d.work_center_id,
        remarks: d.remarks,
        created_at: d.created_at,
        updated_at: d.updated_at,
    }
}

/// 生产订单列表查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ListProductionOrdersQuery {
    /// 订单编号模糊筛选：前端列表页有此筛选框，此前后端 DTO 无该字段导致参数被 serde 静默丢弃
    pub order_no: Option<String>,
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
    payload.validate().map_err(AppError::from)?;

    // 单号禁手输（缺陷3）：DTO 无 order_no 字段、服务层请求结构无注入口，
    // 单号一律服务端取号。旧前端仍携带非空 order_no 时经残差映射显式 warn（不静默），
    // 该值不进建单流程——修复前它会被 resolve_order_no 原样入库，可伪造单号/撞 UNIQUE。
    let forwarded_order_no = payload
        .forwarded_unknown_fields
        .get("order_no")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty());
    if let Some(forwarded) = forwarded_order_no {
        tracing::warn!(
            user_id = auth.user_id,
            forwarded_order_no = %forwarded,
            "创建生产订单请求携带 order_no：单据号禁止手输，该值已被服务端忽略，单号由服务端统一生成"
        );
    }

    let service = ProductionOrderService::new(state.db.clone());

    let req = CreateProductionOrderRequest {
        sales_order_id: payload.sales_order_id,
        product_id: payload.product_id,
        planned_quantity: payload.planned_quantity,
        planned_start_date: payload.planned_start_date,
        planned_end_date: payload.planned_end_date,
        priority: payload.priority,
        work_center_id: payload.work_center_id,
        remarks: payload.remarks,
    };

    // 建单人取服务端会话（AuthContext.user_id），请求体不承载身份。
    let model = service.create(req, auth.user_id).await?;

    let response = response_from_model(model);

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

    let response = response_from_dto(model);

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
        order_no: query.order_no,
        status: query.status,
        product_id: query.product_id,
        page: query.page.unwrap_or(1).clamp(1, 1000), // 批次 95 P3-3~8：分页 clamp 防 DoS
        page_size: query.page_size.unwrap_or(20).clamp(1, 100),
    };

    let (models, total) = service.list(query_params, Some(&data_scope_ctx)).await?;

    let responses: Vec<ProductionOrderResponse> =
        models.into_iter().map(response_from_dto).collect();

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

    let response = response_from_model(model);

    Ok(Json(ApiResponse::success(response)))
}

/// 审批请求
#[allow(dead_code, reason = "反序列化输入字段")]
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
    // IDOR 防护：提交审批前先按当前用户数据范围校验资源归属（与 update/delete 的
    // get_by_id(Some(&data_scope_ctx)) 同源），越权由 get_by_id 内部 check_resource_owner
    // 返回 403（permission_denied）；submit_for_approval 服务侧仅 find_by_id+lock_exclusive，无归属校验。
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_by_id(id, Some(&data_scope_ctx)).await?;

    let model = service
        .submit_for_approval(id, auth.user_id, &auth.username)
        .await?;

    let response = response_from_model(model);

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
    // IDOR 防护（缺陷2）：审批前先按当前用户数据范围校验资源归属（与同域
    // update/delete/submit_for_approval 的 get_by_id(Some(&data_scope_ctx)) 同源范式），
    // 越权由 get_by_id 内部归属校验返回 403（permission_denied）。
    // approve_order 服务侧仅 find_by_id+lock_exclusive+状态门，无归属校验，
    // 此前本端点是同域唯一缺归属预检的写端点（任意登录用户可对他人订单执行审批）。
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_by_id(id, Some(&data_scope_ctx)).await?;

    let model = service
        .approve_order(id, auth.user_id, &auth.username, req.approved, req.opinion)
        .await?;

    let response = response_from_model(model);

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
///
/// 三态语义（RFC 7386）：actual_quantity/remarks 均为 DB 可空列
/// （m0007:79 actual_quantity、m0007:86 remarks）——
/// 键缺席=保持原值、显式 `null`=清空为 NULL、有值=覆盖。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct UpdateProgressRequest {
    #[serde(default, deserialize_with = "double_option")]
    pub actual_quantity: Option<Option<Decimal>>,
    #[serde(default, deserialize_with = "double_option")]
    pub remarks: Option<Option<String>>,
}

/// 更新生产订单进度
pub async fn update_production_progress(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateProgressRequest>,
) -> Result<Json<ApiResponse<ProductionOrderResponse>>, AppError> {
    // IDOR 防护：改写前先按当前用户数据范围校验资源归属（与 update/delete 的
    // get_by_id(Some(&data_scope_ctx)) 同源），越权返回 403。下方 find_by_id 仅用于读取回写实体，无归属校验。
    let service = ProductionOrderService::new(state.db.clone());
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_by_id(id, Some(&data_scope_ctx)).await?;

    // 该路径为写操作、直接返回更新后的实体（无需 product_name JOIN），
    // 故用 find_by_id 读取生产订单 Model（get_by_id 现返回富化 DTO，不含回写所需的实体）。
    let model = crate::models::production_order::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("生产订单不存在"))?;

    // 状态门（缺陷1）：actual_quantity/remarks 是成本归集与审计口径的写入口，
    // 必须参与写入方状态机（services/production_order_ops/crud.rs::validate_status_transition
    // + models/status/* 词表）：仅 IN_PROGRESS 允许上报进度。此前"只有 IN_PROGRESS 能报进度"
    // 只存在于前端 ProductionTable.vue 的按钮门控，属 UI 约束而非安全边界——DRAFT/
    // SCHEDULED/PENDING_APPROVAL/APPROVED/REJECTED/COMPLETED/CANCELLED 任意状态可直连
    // POST /{id}/progress 覆写产量。状态值来自用户已可见的订单数据，无内部 ID/敏感量，
    // 用 business_displayable 外显真实拒绝文案（AppError::business 出参会被脱敏）。
    if model.status != crate::models::status::production::PRODUCTION_IN_PROGRESS {
        return Err(AppError::business_displayable(format!(
            "订单当前状态为 {}，仅生产中（IN_PROGRESS）的订单可上报生产进度",
            model.status
        )));
    }

    let mut active_model: crate::models::production_order::ActiveModel = model.into();
    // 三态写入（可空列）：Some(None)=Set(None) 清空、Some(Some(v))=Set(Some(v)) 覆盖、None=不 Set
    if let Some(qty) = payload.actual_quantity {
        active_model.actual_quantity = Set(qty);
    }
    if let Some(remarks) = payload.remarks {
        active_model.remarks = Set(remarks);
    }
    active_model.updated_at = Set(Utc::now());

    let updated = active_model.update(&*state.db).await?;

    let response = response_from_model(updated);

    Ok(Json(ApiResponse::success(response)))
}

/// 获取生产订单操作日志；批次 132 v9 复审 P1：原返回固定空列表 {logs: []}， 现真实查询
/// audit_logs 表，按 resource_id = order_id 过滤，按 created_at 倒序返回。
pub async fn get_production_order_logs(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ProductionOrderService::new(state.db.clone());
    // IDOR 防护：操作日志只读同样需 data-scope。get_order_logs 服务侧无归属校验，
    // 故在 handler 入口先按当前用户数据范围校验该订单归属（复用同文件 update/delete 的
    // get_by_id(Some(&data_scope_ctx)) 范式），越权由 get_by_id 内部 check_resource_owner 返回 403；
    // 订单不存在返回 404（与 get_production_order 同源）。
    let data_scope_ctx = auth.to_data_scope_context();
    let _ = service
        .get_by_id(id, Some(&data_scope_ctx))
        .await?
        .ok_or_else(|| AppError::not_found("生产订单不存在"))?;

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
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateProductionOrderStatusDto>,
) -> Result<Json<ApiResponse<ProductionOrderResponse>>, AppError> {
    // P1-2f 修复（批次 81 v1 复审）：强类型 DTO + validator + 状态白名单 替代 Json<Value>
    payload.validate().map_err(AppError::from)?;

    let service = ProductionOrderService::new(state.db.clone());
    // IDOR 防护：状态改写前先按当前用户数据范围校验资源归属（与 update/delete 的
    // get_by_id(Some(&data_scope_ctx)) 同源），越权返回 403；update_status 服务侧仅 find_by_id+lock_exclusive，无归属校验。
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_by_id(id, Some(&data_scope_ctx)).await?;

    let actual_quantity = payload
        .actual_quantity
        .as_deref()
        .and_then(|s| s.parse::<Decimal>().ok());

    let model = service
        .update_status(id, payload.status, actual_quantity)
        .await?;

    let response = response_from_model(model);

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
fn convert_orders_to_responses(models: Vec<ProductionOrderDto>) -> Vec<ProductionOrderResponse> {
    models.into_iter().map(response_from_dto).collect()
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
        r.planned_start_date
            .map_or(String::new(), |d| d.to_string()),
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
        resource_type: Some(
            crate::services::production_order_ops::types::AUDIT_RESOURCE_TYPE.to_string(),
        ),
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

/// 导出生产订单列表；V15 P0-S12/P0-S15 修复（Batch 475c）：导出注入水印 + 异步审计日志；规则 3：导出统一使用 xlsx 格式 V15 P0-S11：导出审计日志写入（best-effort，异步不阻塞响应） V15 P0-S15：水印行在 xlsx 第
/// 0 行（合并所有列），标题行下移到第 1 行，数据行从第 2 行起；重要：生产订单表有行级数据权限（V15 P0-S01），必须调 `to_data_scope_context` + `service.list(query, Some(&data_scope_ctx))` 保证数据隔离
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
        order_no: query.order_no.clone(),
        status: query.status.clone(),
        product_id: query.product_id,
        page: 1,
        page_size: 10000,
    };

    let (models, _total) = service.list(query_params, Some(&data_scope_ctx)).await?;

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
