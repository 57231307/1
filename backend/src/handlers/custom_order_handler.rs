//! 定制订单全流程跟踪 Handler
//!
//! 实现 13 个 HTTP 端点：CRUD + 流程推进 + 质检 + 售后
//! 设计依据：docs/superpowers/specs/2026-06-16-custom-order-design.md §3.2
//! 创建时间: 2026-06-17

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use validator::Validate;

use crate::middleware::auth_context::AuthContext;
use crate::models::custom_order_create_dto::{CancelCustomOrderDto, CreateCustomOrderDto, UpdateCustomOrderDto};
use crate::models::custom_order_response_dto::{
    CustomOrderDetail, CustomOrderListItem, PagedResponse, ProcessNodeInfo, ProcessNodeWithLogs,
    ProcessTimeline, QualityIssueInfo, AfterSalesInfo, ProcessLogInfo,
};
use crate::models::custom_order_update_dto::{
    AddProcessLogDto, AdvanceNodeDto, CreateProcessNodeDto, UpdateProcessNodeDto,
};
use crate::models::quality_issue_dto::{ReportQualityIssueDto, ResolveQualityIssueDto};
use crate::services::custom_order_aftersales_service::{
    CreateAfterSalesDto, CustomOrderAfterSalesService, UpdateAfterSalesDto,
};
use crate::services::custom_order_crud_service::CustomOrderCrudService;
use crate::services::custom_order_process_service::CustomOrderProcessService;
use crate::services::custom_order_quality_service::CustomOrderQualityService;
use crate::services::custom_order_state_service::CustomOrderStateService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ----------------------------------------------------------------------
// 公共 DTO
// ----------------------------------------------------------------------

/// 列表查询参数
#[derive(Debug, Deserialize)]
pub struct ListCustomOrdersQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub customer_id: Option<i64>,
    pub keyword: Option<String>,
}

/// 推进请求体
#[derive(Debug, Deserialize)]
pub struct AdvanceRequest {
    pub operator_id: i64,
    pub notes: Option<String>,
}

// ----------------------------------------------------------------------
// 错误转换辅助
// ----------------------------------------------------------------------

fn crud_err(e: crate::services::custom_order_crud_service::CrudError) -> AppError {
    use crate::services::custom_order_crud_service::CrudError::*;
    match e {
        NotFound => AppError::not_found("定制订单不存在"),
        InvalidState => AppError::business("当前状态不允许此操作"),
        Validation(msg) => AppError::validation(msg),
        Database(e) => AppError::database(e.to_string()),
    }
}

fn state_err(e: crate::services::custom_order_state_service::StateError) -> AppError {
    use crate::services::custom_order_state_service::StateError::*;
    match e {
        NotFound => AppError::not_found("定制订单不存在"),
        InvalidTransition(msg) => AppError::business(msg),
        Database(e) => AppError::database(e.to_string()),
        StateMachine(e) => AppError::business(e.to_string()),
    }
}

fn process_err(e: crate::services::custom_order_process_service::ProcessError) -> AppError {
    use crate::services::custom_order_process_service::ProcessError::*;
    match e {
        NotFound => AppError::not_found("工艺节点不存在"),
        InvalidState(msg) => AppError::business(msg),
        Database(e) => AppError::database(e.to_string()),
    }
}

fn quality_err(e: crate::services::custom_order_quality_service::QualityError) -> AppError {
    use crate::services::custom_order_quality_service::QualityError::*;
    match e {
        NotFound => AppError::not_found("质量异常不存在"),
        InvalidState(msg) => AppError::business(msg),
        Validation(msg) => AppError::validation(msg),
        Database(e) => AppError::database(e.to_string()),
    }
}

fn aftersales_err(e: crate::services::custom_order_aftersales_service::AfterSalesError) -> AppError {
    use crate::services::custom_order_aftersales_service::AfterSalesError::*;
    match e {
        NotFound => AppError::not_found("售后工单不存在"),
        InvalidState(msg) => AppError::business(msg),
        Validation(msg) => AppError::validation(msg),
        Database(e) => AppError::database(e.to_string()),
    }
}

// ----------------------------------------------------------------------
// CRUD：list / get / create / update / cancel（5 端点）
// ----------------------------------------------------------------------

/// GET /api/v1/erp/custom-orders - 列表
pub async fn list_custom_orders(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ListCustomOrdersQuery>,
) -> Result<Json<ApiResponse<PagedResponse<CustomOrderListItem>>>, AppError> {
    let service = CustomOrderCrudService::from_state(&state);
    let page = query.page.unwrap_or(1).max(1); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let (items, total) = service
        .list(
            page,
            page_size,
            query.status,
            query.customer_id,
            query.keyword,
        )
        .await
        .map_err(crud_err)?;

    let list: Vec<CustomOrderListItem> = items
        .into_iter()
        .map(|m| CustomOrderListItem {
            id: m.id,
            order_no: m.order_no,
            customer_id: m.customer_id,
            product_id: m.product_id,
            color_id: m.color_id,
            spec: m.spec,
            quantity: m.quantity,
            unit: m.unit,
            status: m.status,
            expected_delivery_date: m.expected_delivery_date,
            actual_delivery_date: m.actual_delivery_date,
            total_amount: m.total_amount,
            currency: m.currency,
            sales_order_id: m.sales_order_id,
            created_at: m.created_at,
            // 批次 88 PH-1 占位符实现：透传 notes 字段
            notes: m.notes,
        })
        .collect();

    Ok(Json(ApiResponse::success(PagedResponse {
        items: list,
        total,
        page,
        page_size,
    })))
}

/// POST /api/v1/erp/custom-orders - 创建草稿
pub async fn create_custom_order(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(dto): Json<CreateCustomOrderDto>,
) -> Result<Json<ApiResponse<CustomOrderListItem>>, AppError> {
    let user_id = auth.user_id as i64;
    let service = CustomOrderCrudService::from_state(&state);

    // 激活 CreateCustomOrderDto 的 Validate 注解，校验入参
    dto.validate()?;

    let created = service
        .create_draft(dto, user_id)
        .await
        .map_err(crud_err)?;

    Ok(Json(ApiResponse::success(CustomOrderListItem {
        id: created.id,
        order_no: created.order_no,
        customer_id: created.customer_id,
        product_id: created.product_id,
        color_id: created.color_id,
        spec: created.spec,
        quantity: created.quantity,
        unit: created.unit,
        status: created.status,
        expected_delivery_date: created.expected_delivery_date,
        actual_delivery_date: created.actual_delivery_date,
        total_amount: created.total_amount,
        currency: created.currency,
        sales_order_id: created.sales_order_id,
        created_at: created.created_at,
        // 批次 88 PH-1 占位符实现：透传 notes 字段
        notes: created.notes,
    })))
}

/// GET /api/v1/erp/custom-orders/:id - 详情
pub async fn get_custom_order(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<CustomOrderDetail>>, AppError> {
    let crud_svc = CustomOrderCrudService::from_state(&state);
    let quality_svc = CustomOrderQualityService::from_state(&state);
    let after_svc = CustomOrderAfterSalesService::from_state(&state);

    let order = crud_svc.get_by_id(id).await.map_err(crud_err)?;
    let nodes = crud_svc
        .list_process_nodes(id)
        .await
        .map_err(crud_err)?;
    let (issues, _) = quality_svc
        .list_by_order(id, 1, 100)
        .await
        .map_err(quality_err)?;
    let (after_sales_list, _) = after_svc
        .list_by_order(id, 1, 100)
        .await
        .map_err(aftersales_err)?;

    Ok(Json(ApiResponse::success(CustomOrderDetail {
        id: order.id,
        order_no: order.order_no,
        customer_id: order.customer_id,
        product_id: order.product_id,
        color_id: order.color_id,
        spec: order.spec,
        quantity: order.quantity,
        unit: order.unit,
        custom_requirements: order.custom_requirements,
        yarn_spec: order.yarn_spec,
        dye_method: order.dye_method,
        finishing_method: order.finishing_method,
        status: order.status,
        expected_delivery_date: order.expected_delivery_date,
        actual_delivery_date: order.actual_delivery_date,
        sales_order_id: order.sales_order_id,
        total_amount: order.total_amount,
        currency: order.currency,
        created_by: order.created_by,
        created_at: order.created_at,
        updated_at: order.updated_at,
        // 批次 88 PH-1 占位符实现：透传 notes 字段
        notes: order.notes,
        process_nodes: nodes
            .into_iter()
            .map(|n| ProcessNodeInfo {
                id: n.id,
                node_type: n.node_type,
                node_name: n.node_name,
                sequence: n.sequence,
                status: n.status,
                planned_start_date: n.planned_start_date,
                planned_end_date: n.planned_end_date,
                actual_start_date: n.actual_start_date,
                actual_end_date: n.actual_end_date,
                operator_id: n.operator_id,
                notes: n.notes,
            })
            .collect(),
        quality_issues: issues
            .into_iter()
            .map(|i| QualityIssueInfo {
                id: i.id,
                issue_type: i.issue_type,
                severity: i.severity,
                description: i.description,
                discovered_at: i.discovered_at,
                resolved_at: i.resolved_at,
                resolution: i.resolution,
                status: i.status,
            })
            .collect(),
        after_sales: after_sales_list
            .into_iter()
            .map(|a| AfterSalesInfo {
                id: a.id,
                issue_type: a.issue_type,
                description: a.description,
                status: a.status,
                opened_at: a.opened_at,
                closed_at: a.closed_at,
                resolution: a.resolution,
                refund_amount: a.refund_amount,
            })
            .collect(),
    })))
}

/// PUT /api/v1/erp/custom-orders/:id - 更新（仅草稿）
pub async fn update_custom_order(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<UpdateCustomOrderDto>,
) -> Result<Json<ApiResponse<CustomOrderListItem>>, AppError> {
    let service = CustomOrderCrudService::from_state(&state);
    let updated = service.update(id, dto).await.map_err(crud_err)?;
    Ok(Json(ApiResponse::success(CustomOrderListItem {
        id: updated.id,
        order_no: updated.order_no,
        customer_id: updated.customer_id,
        product_id: updated.product_id,
        color_id: updated.color_id,
        spec: updated.spec,
        quantity: updated.quantity,
        unit: updated.unit,
        status: updated.status,
        expected_delivery_date: updated.expected_delivery_date,
        actual_delivery_date: updated.actual_delivery_date,
        total_amount: updated.total_amount,
        currency: updated.currency,
        sales_order_id: updated.sales_order_id,
        created_at: updated.created_at,
        // 批次 88 PH-1 占位符实现：透传 notes 字段
        notes: updated.notes,
    })))
}

/// DELETE /api/v1/erp/custom-orders/:id - 取消
pub async fn cancel_custom_order(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<CancelCustomOrderDto>,
) -> Result<Json<ApiResponse<CustomOrderListItem>>, AppError> {
    let user_id = auth.user_id as i64;
    let service = CustomOrderCrudService::from_state(&state);
    let updated = service
        .cancel(id, dto, user_id)
        .await
        .map_err(crud_err)?;
    Ok(Json(ApiResponse::success(CustomOrderListItem {
        id: updated.id,
        order_no: updated.order_no,
        customer_id: updated.customer_id,
        product_id: updated.product_id,
        color_id: updated.color_id,
        spec: updated.spec,
        quantity: updated.quantity,
        unit: updated.unit,
        status: updated.status,
        expected_delivery_date: updated.expected_delivery_date,
        actual_delivery_date: updated.actual_delivery_date,
        total_amount: updated.total_amount,
        currency: updated.currency,
        sales_order_id: updated.sales_order_id,
        created_at: updated.created_at,
        // 批次 88 PH-1 占位符实现：透传 notes 字段
        notes: updated.notes,
    })))
}

// ----------------------------------------------------------------------
// 流程推进（4 端点）
// ----------------------------------------------------------------------

/// POST /api/v1/erp/custom-orders/:id/advance - 推进到下一阶段
pub async fn advance_custom_order(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<AdvanceRequest>,
) -> Result<Json<ApiResponse<CustomOrderListItem>>, AppError> {
    let service = CustomOrderStateService::from_state(&state);
    let updated = service
        .advance(id, req.operator_id, req.notes)
        .await
        .map_err(state_err)?;
    Ok(Json(ApiResponse::success(CustomOrderListItem {
        id: updated.id,
        order_no: updated.order_no,
        customer_id: updated.customer_id,
        product_id: updated.product_id,
        color_id: updated.color_id,
        spec: updated.spec,
        quantity: updated.quantity,
        unit: updated.unit,
        status: updated.status,
        expected_delivery_date: updated.expected_delivery_date,
        actual_delivery_date: updated.actual_delivery_date,
        total_amount: updated.total_amount,
        currency: updated.currency,
        sales_order_id: updated.sales_order_id,
        created_at: updated.created_at,
        // 批次 88 PH-1 占位符实现：透传 notes 字段
        notes: updated.notes,
    })))
}

/// POST /api/v1/erp/custom-orders/:id/nodes - 添加工艺节点
pub async fn add_process_node(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<CreateProcessNodeDto>,
) -> Result<Json<ApiResponse<ProcessNodeInfo>>, AppError> {
    let service = CustomOrderProcessService::from_state(&state);
    // 激活 CreateProcessNodeDto 的 Validate 注解，校验入参
    dto.validate()?;
    let node = service
        .add_node(id, dto)
        .await
        .map_err(process_err)?;
    Ok(Json(ApiResponse::success(ProcessNodeInfo {
        id: node.id,
        node_type: node.node_type,
        node_name: node.node_name,
        sequence: node.sequence,
        status: node.status,
        planned_start_date: node.planned_start_date,
        planned_end_date: node.planned_end_date,
        actual_start_date: node.actual_start_date,
        actual_end_date: node.actual_end_date,
        operator_id: node.operator_id,
        notes: node.notes,
    })))
}

/// PUT /api/v1/erp/custom-orders/:id/nodes/:nid - 更新节点
pub async fn update_process_node(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path((_oid, nid)): Path<(i64, i64)>,
    Json(dto): Json<UpdateProcessNodeDto>,
) -> Result<Json<ApiResponse<ProcessNodeInfo>>, AppError> {
    let service = CustomOrderProcessService::from_state(&state);
    let node = service
        .update_node(nid, dto)
        .await
        .map_err(process_err)?;
    Ok(Json(ApiResponse::success(ProcessNodeInfo {
        id: node.id,
        node_type: node.node_type,
        node_name: node.node_name,
        sequence: node.sequence,
        status: node.status,
        planned_start_date: node.planned_start_date,
        planned_end_date: node.planned_end_date,
        actual_start_date: node.actual_start_date,
        actual_end_date: node.actual_end_date,
        operator_id: node.operator_id,
        notes: node.notes,
    })))
}

/// POST /api/v1/erp/custom-orders/:id/nodes/:nid/advance - 推进节点
pub async fn advance_process_node(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path((_oid, nid)): Path<(i64, i64)>,
    Json(dto): Json<AdvanceNodeDto>,
) -> Result<Json<ApiResponse<ProcessNodeInfo>>, AppError> {
    let service = CustomOrderProcessService::from_state(&state);
    let node = service
        .advance_node(nid, dto)
        .await
        .map_err(process_err)?;
    Ok(Json(ApiResponse::success(ProcessNodeInfo {
        id: node.id,
        node_type: node.node_type,
        node_name: node.node_name,
        sequence: node.sequence,
        status: node.status,
        planned_start_date: node.planned_start_date,
        planned_end_date: node.planned_end_date,
        actual_start_date: node.actual_start_date,
        actual_end_date: node.actual_end_date,
        operator_id: node.operator_id,
        notes: node.notes,
    })))
}

/// GET /api/v1/erp/custom-orders/:id/timeline - 完整时间线
pub async fn get_timeline(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<ProcessTimeline>>, AppError> {
    let crud_svc = CustomOrderCrudService::from_state(&state);
    let process_svc = CustomOrderProcessService::from_state(&state);

    let order = crud_svc.get_by_id(id).await.map_err(crud_err)?;
    let timeline_data = process_svc
        .get_timeline(id)
        .await
        .map_err(process_err)?;

    let nodes: Vec<ProcessNodeWithLogs> = timeline_data
        .into_iter()
        .map(|(n, logs)| ProcessNodeWithLogs {
            id: n.id,
            node_type: n.node_type,
            node_name: n.node_name,
            sequence: n.sequence,
            status: n.status,
            planned_start_date: n.planned_start_date,
            planned_end_date: n.planned_end_date,
            actual_start_date: n.actual_start_date,
            actual_end_date: n.actual_end_date,
            logs: logs
                .into_iter()
                .map(|l| {
                    let attachments: Vec<String> = l
                        .attachments
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    ProcessLogInfo {
                        id: l.id,
                        action: l.action,
                        operator_id: l.operator_id,
                        before_status: l.before_status,
                        after_status: l.after_status,
                        log_time: l.log_time,
                        log_content: l.log_content,
                        attachments,
                    }
                })
                .collect(),
        })
        .collect();

    Ok(Json(ApiResponse::success(ProcessTimeline {
        order_id: order.id,
        order_no: order.order_no,
        current_status: order.status,
        nodes,
    })))
}

// ----------------------------------------------------------------------
// 质检（3 端点）
// ----------------------------------------------------------------------

/// POST /api/v1/erp/custom-orders/:id/issues - 上报异常
pub async fn report_quality_issue(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(_id): Path<i64>,
    Json(mut dto): Json<ReportQualityIssueDto>,
) -> Result<Json<ApiResponse<QualityIssueInfo>>, AppError> {
    // URL 中的 id 与 body 中 custom_order_id 一致时，使用 URL 的 id 作为权威
    dto.custom_order_id = _id;
    let service = CustomOrderQualityService::from_state(&state);
    let issue = service
        .report_issue(dto)
        .await
        .map_err(quality_err)?;
    Ok(Json(ApiResponse::success(QualityIssueInfo {
        id: issue.id,
        issue_type: issue.issue_type,
        severity: issue.severity,
        description: issue.description,
        discovered_at: issue.discovered_at,
        resolved_at: issue.resolved_at,
        resolution: issue.resolution,
        status: issue.status,
    })))
}

/// GET /api/v1/erp/custom-orders/:id/issues - 异常列表
#[derive(Debug, Deserialize)]
pub struct ListIssuesQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

pub async fn list_quality_issues(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(query): Query<ListIssuesQuery>,
) -> Result<Json<ApiResponse<PagedResponse<QualityIssueInfo>>>, AppError> {
    let service = CustomOrderQualityService::from_state(&state);
    let page = query.page.unwrap_or(1).max(1); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let (items, total) = service
        .list_by_order(id, page, page_size)
        .await
        .map_err(quality_err)?;

    let list: Vec<QualityIssueInfo> = items
        .into_iter()
        .map(|i| QualityIssueInfo {
            id: i.id,
            issue_type: i.issue_type,
            severity: i.severity,
            description: i.description,
            discovered_at: i.discovered_at,
            resolved_at: i.resolved_at,
            resolution: i.resolution,
            status: i.status,
        })
        .collect();

    Ok(Json(ApiResponse::success(PagedResponse {
        items: list,
        total,
        page,
        page_size,
    })))
}

/// PUT /api/v1/erp/custom-orders/issues/:id/resolve - 解决异常
pub async fn resolve_quality_issue(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<ResolveQualityIssueDto>,
) -> Result<Json<ApiResponse<QualityIssueInfo>>, AppError> {
    let service = CustomOrderQualityService::from_state(&state);
    // 批次 94 P2-15 修复：resolve_issue 返回类型改为 AppError，无需 map_err(quality_err) 转换
    let issue = service.resolve_issue(id, dto).await?;
    Ok(Json(ApiResponse::success(QualityIssueInfo {
        id: issue.id,
        issue_type: issue.issue_type,
        severity: issue.severity,
        description: issue.description,
        discovered_at: issue.discovered_at,
        resolved_at: issue.resolved_at,
        resolution: issue.resolution,
        status: issue.status,
    })))
}

// ----------------------------------------------------------------------
// 售后（3 端点）
// ----------------------------------------------------------------------

/// POST /api/v1/erp/custom-orders/:id/after-sales - 创建售后工单
pub async fn create_after_sales(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(_id): Path<i64>,
    Json(mut dto): Json<CreateAfterSalesDto>,
) -> Result<Json<ApiResponse<AfterSalesInfo>>, AppError> {
    dto.custom_order_id = _id;
    let service = CustomOrderAfterSalesService::from_state(&state);
    let after = service.create(dto).await.map_err(aftersales_err)?;
    Ok(Json(ApiResponse::success(AfterSalesInfo {
        id: after.id,
        issue_type: after.issue_type,
        description: after.description,
        status: after.status,
        opened_at: after.opened_at,
        closed_at: after.closed_at,
        resolution: after.resolution,
        refund_amount: after.refund_amount,
    })))
}

/// GET /api/v1/erp/custom-orders/:id/after-sales - 售后列表
pub async fn list_after_sales(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(query): Query<ListIssuesQuery>,
) -> Result<Json<ApiResponse<PagedResponse<AfterSalesInfo>>>, AppError> {
    let service = CustomOrderAfterSalesService::from_state(&state);
    let page = query.page.unwrap_or(1).max(1); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let (items, total) = service
        .list_by_order(id, page, page_size)
        .await
        .map_err(aftersales_err)?;

    let list: Vec<AfterSalesInfo> = items
        .into_iter()
        .map(|a| AfterSalesInfo {
            id: a.id,
            issue_type: a.issue_type,
            description: a.description,
            status: a.status,
            opened_at: a.opened_at,
            closed_at: a.closed_at,
            resolution: a.resolution,
            refund_amount: a.refund_amount,
        })
        .collect();

    Ok(Json(ApiResponse::success(PagedResponse {
        items: list,
        total,
        page,
        page_size,
    })))
}

/// PUT /api/v1/erp/custom-orders/after-sales/:id - 更新售后
pub async fn update_after_sales(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<UpdateAfterSalesDto>,
) -> Result<Json<ApiResponse<AfterSalesInfo>>, AppError> {
    let service = CustomOrderAfterSalesService::from_state(&state);
    let after = service
        .update(id, dto)
        .await
        .map_err(aftersales_err)?;
    Ok(Json(ApiResponse::success(AfterSalesInfo {
        id: after.id,
        issue_type: after.issue_type,
        description: after.description,
        status: after.status,
        opened_at: after.opened_at,
        closed_at: after.closed_at,
        resolution: after.resolution,
        refund_amount: after.refund_amount,
    })))
}

// ----------------------------------------------------------------------
// 辅助：工艺日志（无独立端点，由 advance_node 内部记录）
// ----------------------------------------------------------------------

/// POST /api/v1/erp/custom-orders/:id/nodes/:nid/logs - 添加日志
pub async fn add_node_log(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path((_oid, nid)): Path<(i64, i64)>,
    Json(dto): Json<AddProcessLogDto>,
) -> Result<Json<ApiResponse<ProcessLogInfo>>, AppError> {
    let service = CustomOrderProcessService::from_state(&state);
    let log = service.add_log(nid, dto).await.map_err(process_err)?;
    let attachments: Vec<String> = log
        .attachments
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    Ok(Json(ApiResponse::success(ProcessLogInfo {
        id: log.id,
        action: log.action,
        operator_id: log.operator_id,
        before_status: log.before_status,
        after_status: log.after_status,
        log_time: log.log_time,
        log_content: log.log_content,
        attachments,
    })))
}
