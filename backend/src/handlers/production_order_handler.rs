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
use crate::utils::response::{ApiResponse, PaginatedResponse};

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
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<ProductionOrderResponse>>, AppError> {
    let service = ProductionOrderService::new(state.db.clone());

    let model = service
        .get_by_id(id)
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
    _auth: AuthContext,
    Query(query): Query<ListProductionOrdersQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<ProductionOrderResponse>>>, AppError> {
    let service = ProductionOrderService::new(state.db.clone());

    let query_params = ProductionOrderQuery {
        status: query.status,
        product_id: query.product_id,
        page: query.page.unwrap_or(1),
        page_size: query.page_size.unwrap_or(20).clamp(1, 100),
    };

    let (models, total) = service.list(query_params).await?;

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
        query.page.unwrap_or(1),
        query.page_size.unwrap_or(20).clamp(1, 100),
    )))
}

/// 更新生产订单
pub async fn update_production_order(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateProductionOrderPayload>,
) -> Result<Json<ApiResponse<ProductionOrderResponse>>, AppError> {
    let service = ProductionOrderService::new(state.db.clone());

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
        "审批通过"
    } else {
        "审批拒绝"
    };
    Ok(Json(ApiResponse::success_with_message(response, message)))
}

/// 删除生产订单（软删除）
pub async fn delete_production_order(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let service = ProductionOrderService::new(state.db.clone());
    service.delete(id).await?;
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
        .get_by_id(id)
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
pub async fn get_production_order_logs(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let _service = ProductionOrderService::new(state.db.clone());
    // 返回空日志列表，后续可扩展为从审计日志表查询
    Ok(Json(ApiResponse::success(serde_json::json!({
        "order_id": id,
        "logs": []
    }))))
}

/// 更新生产订单状态
pub async fn update_production_order_status(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<ProductionOrderResponse>>, AppError> {
    let service = ProductionOrderService::new(state.db.clone());

    let status = payload
        .get("status")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::validation("状态不能为空"))?;

    let actual_quantity = payload
        .get("actual_quantity")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<Decimal>().ok());

    let model = service
        .update_status(id, status.to_string(), actual_quantity)
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
