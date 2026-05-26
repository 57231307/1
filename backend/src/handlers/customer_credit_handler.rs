use crate::middleware::auth_context::AuthContext;
use crate::models::customer_credit;
use crate::services::customer_credit_service::{
    CreditEvaluationResult, CreditLimitAdjustmentRequest, CreditQueryParams, CreditRatingRequest, CustomerCreditService,
};
use crate::utils::error::AppError;
use crate::utils::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use crate::utils::app_state::AppState;
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use tracing::info;

/// 客户信用查询参数 DTO
#[derive(Debug, Deserialize)]
pub struct CreditQuery {
    pub customer_id: Option<i32>,
    pub credit_level: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 创建/更新信用评级请求 DTO
#[derive(Debug, Deserialize, Serialize)]
pub struct CreditRatingRequestDto {
    pub customer_id: i32,
    pub credit_level: Option<String>,
    pub credit_score: Option<i32>,
    pub credit_limit: BigDecimal,
    pub credit_days: Option<i32>,
    pub remark: Option<String>,
}

/// 信用额度调整请求 DTO
#[derive(Debug, Deserialize)]
pub struct CreditLimitAdjustmentRequestDto {
    pub adjustment_type: String,
    pub amount: BigDecimal,
    pub reason: String,
}

/// 占用/释放信用额度请求 DTO
#[derive(Debug, Deserialize)]
pub struct CreditAmountRequest {
    pub amount: BigDecimal,
}

/// 获取客户信用列表
pub async fn list_credits(
    Query(params): Query<CreditQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<customer_credit::Model>>>, AppError> {
    info!("用户 {} 正在查询客户信用列表", auth.user_id);

    let service = CustomerCreditService::new(state.db.clone());
    let query_params = CreditQueryParams {
        customer_id: params.customer_id,
        credit_level: params.credit_level,
        status: params.status,
        page: params.page.unwrap_or(0),
        page_size: params.page_size.unwrap_or(10),
    };

    let (credits, _total) = service.get_list(query_params).await?;
    info!("客户信用列表查询成功，共 {} 条记录", credits.len());

    Ok(Json(ApiResponse::success(credits)))
}

/// 获取客户信用详情
pub async fn get_credit(
    Path(customer_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<customer_credit::Model>>, AppError> {
    info!(
        "用户 {} 正在查询客户 {} 的信用详情",
        auth.user_id, customer_id
    );

    let service = CustomerCreditService::new(state.db.clone());
    let credit = service
        .get_by_customer_id(customer_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("客户 {} 的信用评级不存在", customer_id)))?;
    info!(
        "客户 {} 信用详情查询成功，等级：{}",
        customer_id,
        credit.credit_level.as_deref().unwrap_or("N/A")
    );

    Ok(Json(ApiResponse::success(credit)))
}

/// 设置客户信用评级
#[axum::debug_handler]
pub async fn set_credit_rating(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreditRatingRequestDto>,
) -> Result<Json<ApiResponse<customer_credit::Model>>, AppError> {
    info!(
        "用户 {} 正在设置客户 {} 的信用评级",
        auth.user_id, req.customer_id
    );

    let service = CustomerCreditService::new(state.db.clone());
    let rating_req = CreditRatingRequest {
        customer_id: req.customer_id,
        credit_level: req.credit_level,
        credit_score: req.credit_score,
        credit_limit: req.credit_limit,
        credit_days: req.credit_days,
        remark: req.remark,
    };

    let credit = service.set_credit_rating(rating_req, auth.user_id).await?;
    info!("客户 {} 信用评级设置成功", req.customer_id);

    Ok(Json(ApiResponse::success(credit)))
}

/// 占用信用额度
#[axum::debug_handler]
pub async fn occupy_credit(
    Path(customer_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreditAmountRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!(
        "用户 {} 正在占用客户 {} 的信用额度 {:.2}",
        auth.user_id, customer_id, req.amount
    );

    let service = CustomerCreditService::new(state.db.clone());
    service
        .occupy_credit(customer_id, req.amount, auth.user_id)
        .await?;

    let message = format!("客户 {} 信用额度占用成功", customer_id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}

/// 释放信用额度
#[axum::debug_handler]
pub async fn release_credit(
    Path(customer_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreditAmountRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!(
        "用户 {} 正在释放客户 {} 的信用额度 {:.2}",
        auth.user_id, customer_id, req.amount
    );

    let service = CustomerCreditService::new(state.db.clone());
    service
        .release_credit(customer_id, req.amount, auth.user_id)
        .await?;

    let message = format!("客户 {} 信用额度释放成功", customer_id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}

/// 调整信用额度
#[axum::debug_handler]
pub async fn adjust_credit_limit(
    Path(customer_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreditLimitAdjustmentRequestDto>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!(
        "用户 {} 正在调整客户 {} 的信用额度",
        auth.user_id, customer_id
    );

    let service = CustomerCreditService::new(state.db.clone());
    let adjust_req = CreditLimitAdjustmentRequest {
        customer_id,
        adjustment_type: req.adjustment_type,
        amount: req.amount,
        reason: req.reason,
    };

    service
        .adjust_credit_limit(adjust_req, auth.user_id)
        .await?;

    let message = format!("客户 {} 信用额度调整成功", customer_id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}

/// 停用客户信用
pub async fn deactivate_credit(
    Path(customer_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在停用客户 {} 的信用", auth.user_id, customer_id);

    let service = CustomerCreditService::new(state.db.clone());
    service.deactivate(customer_id, auth.user_id).await?;

    let message = format!("客户 {} 信用停用成功", customer_id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}


/// POST /api/v1/erp/customer-credits - 创建客户信用
pub async fn create_credit(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 创建客户信用", auth.username);

    let service = CustomerCreditService::new(state.db.clone());

    let customer_id = req.get("customer_id")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    let credit_limit = req.get("credit_limit")
        .and_then(|v| v.as_f64())
        .map(|f| BigDecimal::try_from(f).unwrap_or_default())
        .unwrap_or_default();

    let credit_level = req.get("credit_level")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let credit_score = req.get("credit_score")
        .and_then(|v| v.as_i64())
        .map(|s| s as i32);

    let credit_days = req.get("credit_days")
        .and_then(|v| v.as_i64())
        .map(|d| d as i32);

    let rating_req = crate::services::customer_credit_service::CreditRatingRequest {
        customer_id,
        credit_level,
        credit_score,
        credit_limit,
        credit_days,
        remark: None,
    };

    let credit = service.set_credit_rating(rating_req, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(credit)?,
        "客户信用创建成功",
    )))
}

/// PUT /api/v1/erp/customer-credits/:id - 更新客户信用
pub async fn update_credit(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 更新客户信用: ID={}", auth.username, id);

    let service = CustomerCreditService::new(state.db.clone());

    let credit_limit = req.get("credit_limit")
        .and_then(|v| v.as_f64())
        .map(|f| BigDecimal::try_from(f).unwrap_or_default());

    let credit_level = req.get("credit_level")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let credit_score = req.get("credit_score")
        .and_then(|v| v.as_i64())
        .map(|i| i as i32);

    let credit_days = req.get("credit_days")
        .and_then(|v| v.as_i64())
        .map(|i| i as i32);

    let rating_req = crate::services::customer_credit_service::CreditRatingRequest {
        customer_id: id,
        credit_level,
        credit_score,
        credit_limit: credit_limit.unwrap_or_default(),
        credit_days,
        remark: None,
    };

    let credit = service.set_credit_rating(rating_req, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(credit)?,
        "客户信用更新成功",
    )))
}

/// DELETE /api/v1/erp/customer-credits/:id - 删除客户信用
pub async fn delete_credit(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 删除客户信用: ID={}", auth.username, id);

    let service = CustomerCreditService::new(state.db.clone());
    service.deactivate(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message((), "客户信用已删除")))
}

/// 信用评估模型请求
#[derive(Debug, Deserialize)]
pub struct CreditEvaluationRequest {
    pub customer_id: i32,
    pub evaluation_date: String,
}

/// 信用评估
#[axum::debug_handler]
pub async fn evaluate_credit(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreditEvaluationRequest>,
) -> Result<Json<ApiResponse<CreditEvaluationResult>>, AppError> {
    info!("用户 {} 正在评估客户 {} 的信用", auth.username, req.customer_id);
    
    let service = CustomerCreditService::new(state.db.clone());
    let result = service.evaluate_credit(req.customer_id, req.evaluation_date, auth.user_id).await?;
    
    Ok(Json(ApiResponse::success(result)))
}
