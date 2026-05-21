//! 生产排程 Handler
//!
//! 生产排程API端点，提供自动排程、甘特图数据、冲突检测和手动调整功能

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::services::scheduling_service::{
    AdjustScheduleRequest, AutoScheduleRequest, ScheduledOrderQuery, SchedulingService,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 自动排程请求体
#[derive(Debug, Deserialize)]
pub struct AutoSchedulePayload {
    pub work_center_ids: Option<Vec<i32>>,
    pub start_date: Option<NaiveDate>,
    pub strategy: Option<String>,
}

/// 甘特图数据项响应
#[derive(Debug, Serialize)]
pub struct GanttItemResponse {
    pub id: String,
    pub order_id: i32,
    pub order_no: String,
    pub product_id: i32,
    pub work_center_id: i32,
    pub work_center_name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub duration_days: i64,
    pub progress: f64,
    pub status: String,
    pub priority: i32,
    pub dependencies: Vec<String>,
}

/// 工作中心信息响应
#[derive(Debug, Serialize)]
pub struct WorkCenterInfoResponse {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub status: String,
}

/// 日期范围响应
#[derive(Debug, Serialize)]
pub struct DateRangeResponse {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

/// 甘特图数据响应
#[derive(Debug, Serialize)]
pub struct GanttDataResponse {
    pub items: Vec<GanttItemResponse>,
    pub work_centers: Vec<WorkCenterInfoResponse>,
    pub date_range: DateRangeResponse,
}

/// 排程冲突响应
#[derive(Debug, Serialize)]
pub struct ConflictResponse {
    pub conflict_type: String,
    pub order_id: i32,
    pub order_no: String,
    pub conflicting_order_id: Option<i32>,
    pub conflicting_order_no: Option<String>,
    pub work_center_id: Option<i32>,
    pub description: String,
    pub severity: String,
}

/// 排程明细响应
#[derive(Debug, Serialize)]
pub struct ScheduleDetailResponse {
    pub order_id: i32,
    pub order_no: String,
    pub work_center_id: i32,
    pub work_center_name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: String,
}

/// 自动排程结果响应
#[derive(Debug, Serialize)]
pub struct AutoScheduleResultResponse {
    pub total_orders: i32,
    pub scheduled_orders: i32,
    pub unscheduled_orders: i32,
    pub conflicts: Vec<ConflictResponse>,
    pub gantt_data: GanttDataResponse,
    pub schedule_details: Vec<ScheduleDetailResponse>,
}

/// 手动调整排程请求体
#[derive(Debug, Deserialize)]
pub struct AdjustSchedulePayload {
    pub work_center_id: Option<i32>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub priority: Option<i32>,
}

/// 排程工单响应
#[derive(Debug, Serialize)]
pub struct ScheduledOrderResponse {
    pub order_id: i32,
    pub order_no: String,
    pub product_id: i32,
    pub quantity: Decimal,
    pub work_center_id: i32,
    pub work_center_name: String,
    pub start_time: NaiveDate,
    pub end_time: NaiveDate,
    pub priority: i32,
    pub status: String,
    pub dependencies: Vec<i32>,
}

/// 甘特图查询参数
#[derive(Debug, Deserialize)]
pub struct GanttQuery {
    pub work_center_id: Option<i32>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
}

/// 排程工单列表查询参数
#[derive(Debug, Deserialize)]
pub struct ScheduledOrdersQuery {
    pub work_center_id: Option<i32>,
    pub status: Option<String>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 自动排程
pub async fn auto_schedule(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(payload): Json<AutoSchedulePayload>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SchedulingService::new(state.db.clone());

    let strategy = payload.strategy.clone().unwrap_or_else(|| "priority".to_string());
    let req = AutoScheduleRequest {
        work_center_ids: payload.work_center_ids,
        start_date: payload.start_date,
        strategy: payload.strategy,
    };

    let result = service.auto_schedule(req).await?;

    // 持久化排程结果
    let saved_result = service.save_schedule_result(
        &result,
        &strategy,
        auth.user_id,
        &auth.username,
        None,
    ).await?;

    let response = serde_json::json!({
        "id": saved_result.id,
        "batch_no": saved_result.batch_no,
        "total_orders": result.total_orders,
        "scheduled_orders": result.scheduled_orders,
        "unscheduled_orders": result.unscheduled_orders,
        "conflicts": result.conflicts,
        "gantt_data": result.gantt_data,
        "schedule_details": result.schedule_details,
    });

    Ok(Json(ApiResponse::success(response)))
}

/// 获取甘特图数据
pub async fn get_gantt_data(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<GanttQuery>,
) -> Result<Json<ApiResponse<GanttDataResponse>>, AppError> {
    let service = SchedulingService::new(state.db.clone());

    let gantt_data = service
        .get_gantt_data(query.work_center_id, query.date_from, query.date_to)
        .await?;

    let response = GanttDataResponse {
        items: gantt_data
            .items
            .into_iter()
            .map(|item| GanttItemResponse {
                id: item.id,
                order_id: item.order_id,
                order_no: item.order_no,
                product_id: item.product_id,
                work_center_id: item.work_center_id,
                work_center_name: item.work_center_name,
                start_date: item.start_date,
                end_date: item.end_date,
                duration_days: item.duration_days,
                progress: item.progress,
                status: item.status,
                priority: item.priority,
                dependencies: item.dependencies,
            })
            .collect(),
        work_centers: gantt_data
            .work_centers
            .into_iter()
            .map(|wc| WorkCenterInfoResponse {
                id: wc.id,
                code: wc.code,
                name: wc.name,
                status: wc.status,
            })
            .collect(),
        date_range: DateRangeResponse {
            start: gantt_data.date_range.start,
            end: gantt_data.date_range.end,
        },
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 检测排程冲突
pub async fn detect_conflicts(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<ConflictResponse>>>, AppError> {
    let service = SchedulingService::new(state.db.clone());

    let conflicts = service.detect_conflicts().await?;

    let response: Vec<ConflictResponse> = conflicts
        .into_iter()
        .map(|c| ConflictResponse {
            conflict_type: c.conflict_type,
            order_id: c.order_id,
            order_no: c.order_no,
            conflicting_order_id: c.conflicting_order_id,
            conflicting_order_no: c.conflicting_order_no,
            work_center_id: c.work_center_id,
            description: c.description,
            severity: c.severity,
        })
        .collect();

    Ok(Json(ApiResponse::success(response)))
}

/// 手动调整排程
pub async fn adjust_schedule(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(payload): Json<AdjustSchedulePayload>,
) -> Result<Json<ApiResponse<ScheduleDetailResponse>>, AppError> {
    let service = SchedulingService::new(state.db.clone());

    let req = AdjustScheduleRequest {
        work_center_id: payload.work_center_id,
        start_date: payload.start_date,
        end_date: payload.end_date,
        priority: payload.priority,
    };

    let detail = service.adjust_schedule(id, req).await?;

    let response = ScheduleDetailResponse {
        order_id: detail.order_id,
        order_no: detail.order_no,
        work_center_id: detail.work_center_id,
        work_center_name: detail.work_center_name,
        start_date: detail.start_date,
        end_date: detail.end_date,
        status: detail.status,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 排程工单列表
pub async fn list_scheduled_orders(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<ScheduledOrdersQuery>,
) -> Result<Json<ApiResponse<Vec<ScheduledOrderResponse>>>, AppError> {
    let service = SchedulingService::new(state.db.clone());

    let query_params = ScheduledOrderQuery {
        work_center_id: query.work_center_id,
        status: query.status,
        date_from: query.date_from,
        date_to: query.date_to,
        page: query.page.unwrap_or(1),
        page_size: query.page_size.unwrap_or(20),
    };

    let (orders, total) = service.list_scheduled_orders(query_params).await?;

    let response: Vec<ScheduledOrderResponse> = orders
        .into_iter()
        .map(|o| ScheduledOrderResponse {
            order_id: o.order_id,
            order_no: o.order_no,
            product_id: o.product_id,
            quantity: o.quantity,
            work_center_id: o.work_center_id,
            work_center_name: o.work_center_name,
            start_time: o.start_time,
            end_time: o.end_time,
            priority: o.priority,
            status: o.status,
            dependencies: o.dependencies,
        })
        .collect();

    Ok(Json(ApiResponse::success_paginated(
        response,
        total,
        query.page.unwrap_or(1),
        query.page_size.unwrap_or(20),
    )))
}

/// 排程历史查询参数
#[derive(Debug, Deserialize)]
pub struct ScheduleHistoryQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 排程历史响应
#[derive(Debug, Serialize)]
pub struct ScheduleHistoryResponse {
    pub id: i32,
    pub batch_no: String,
    pub strategy: String,
    pub status: String,
    pub total_orders: i32,
    pub scheduled_orders: i32,
    pub unscheduled_orders: i32,
    pub conflict_count: i32,
    pub schedule_start_date: NaiveDate,
    pub schedule_end_date: NaiveDate,
    pub created_by_name: Option<String>,
    pub remarks: Option<String>,
    pub created_at: String,
}

/// 获取排程历史记录
pub async fn get_schedule_history(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<ScheduleHistoryQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SchedulingService::new(state.db.clone());
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let (items, total) = service.get_schedule_history(page, page_size).await?;

    let response: Vec<ScheduleHistoryResponse> = items
        .into_iter()
        .map(|item| ScheduleHistoryResponse {
            id: item.id,
            batch_no: item.batch_no,
            strategy: item.strategy,
            status: item.status,
            total_orders: item.total_orders,
            scheduled_orders: item.scheduled_orders,
            unscheduled_orders: item.unscheduled_orders,
            conflict_count: item.conflict_count,
            schedule_start_date: item.schedule_start_date,
            schedule_end_date: item.schedule_end_date,
            created_by_name: item.created_by_name,
            remarks: item.remarks,
            created_at: item.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": response,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// 获取排程结果详情
pub async fn get_schedule_result(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SchedulingService::new(state.db.clone());

    let result = service.get_schedule_result(id).await?;
    match result {
        Some(model) => Ok(Json(ApiResponse::success(serde_json::to_value(model)?))),
        None => Err(AppError::NotFound("排程结果不存在".to_string())),
    }
}

/// 确认排程结果
pub async fn confirm_schedule_result(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SchedulingService::new(state.db.clone());

    let result = service.confirm_schedule_result(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(result)?,
        "排程结果已确认",
    )))
}
