use crate::middleware::auth_context::AuthContext;
use crate::models::customer_credit;
use crate::services::customer_credit_service::{
    CreditEvaluationResult, CreditLimitAdjustmentRequest, CreditQueryParams, CreditRatingRequest,
    CustomerCreditService,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;
use tracing::info;

// 批次 98 P2-B 修复（v5 复审）：本地 validate_amount_range 已抽取到 utils::validator 模块，
// 统一追加 round_dp(2) 精度校验。#[validate(custom)] 引用改为 crate::utils::validator::validate_amount_range。

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
///
/// P1-2b 修复（批次 81 v1 复审）：添加 Validate + 字段校验，用于 create_credit
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct CreditRatingRequestDto {
    pub customer_id: i32,
    #[validate(length(max = 20, message = "信用等级长度不能超过20字符"))]
    pub credit_level: Option<String>,
    pub credit_score: Option<i32>,
    #[validate(custom(function = "crate::utils::validator::validate_amount_range"))]
    pub credit_limit: Decimal,
    pub credit_days: Option<i32>,
    #[validate(length(max = 500, message = "备注长度不能超过500字符"))]
    pub remark: Option<String>,
}

/// P1-2b 修复（批次 81 v1 复审）：更新客户信用请求 DTO
/// 用于 update_credit，所有字段可选（仅更新提交的字段）
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct UpdateCreditDto {
    #[validate(length(max = 20, message = "信用等级长度不能超过20字符"))]
    pub credit_level: Option<String>,
    pub credit_score: Option<i32>,
    pub credit_limit: Option<Decimal>,
    pub credit_days: Option<i32>,
    #[validate(length(max = 500, message = "备注长度不能超过500字符"))]
    pub remark: Option<String>,
}

/// 信用额度调整请求 DTO
/// 批次 31 v7 P1-6 修复：添加 Validate + 字段验证
#[derive(Debug, Deserialize, Validate)]
pub struct CreditLimitAdjustmentRequestDto {
    #[validate(length(min = 1, max = 20, message = "调整类型长度必须在1到20字符之间"))]
    pub adjustment_type: String,
    #[validate(custom(function = "crate::utils::validator::validate_amount_range"))]
    pub amount: Decimal,
    #[validate(length(min = 1, max = 500, message = "调整原因不能为空且不超过500字符"))]
    pub reason: String,
}

/// 占用/释放信用额度请求 DTO
/// 批次 31 v7 P1-6 修复：添加 Validate + 字段验证
#[derive(Debug, Deserialize, Validate)]
pub struct CreditAmountRequest {
    #[validate(custom(function = "crate::utils::validator::validate_amount_range"))]
    pub amount: Decimal,
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
        page: params.page.unwrap_or(1).clamp(1, 1000),
        page_size: params.page_size.unwrap_or(10).clamp(1, 100),
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
        .ok_or_else(|| AppError::not_found(format!("客户 {} 的信用评级不存在", customer_id)))?;
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
    // v8 P1-C 修复：调用 DTO 验证，激活 Validate 注解
    req.validate()?;
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
    // v8 P1-C 修复：调用 DTO 验证，激活 Validate 注解
    req.validate()?;
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
    // v8 P1-C 修复：调用 DTO 验证，激活 Validate 注解
    req.validate()?;
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
    Json(req): Json<CreditRatingRequestDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 创建客户信用", auth.username);

    // P1-2b 修复（批次 81 v1 复审）：强类型 DTO + validator 替代 Json<Value>
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let service = CustomerCreditService::new(state.db.clone());

    let rating_req = crate::services::customer_credit_service::CreditRatingRequest {
        customer_id: req.customer_id,
        credit_level: req.credit_level,
        credit_score: req.credit_score,
        credit_limit: req.credit_limit,
        credit_days: req.credit_days,
        remark: req.remark,
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
    Json(req): Json<UpdateCreditDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 更新客户信用: ID={}", auth.username, id);

    // P1-2b 修复（批次 81 v1 复审）：强类型 DTO + validator 替代 Json<Value>
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let service = CustomerCreditService::new(state.db.clone());

    let rating_req = crate::services::customer_credit_service::CreditRatingRequest {
        customer_id: id,
        credit_level: req.credit_level,
        credit_score: req.credit_score,
        // 批次 407 标注：credit_limit 缺失时默认为 0 有风险（service 层无法区分"未提供"与"显式置 0"）
        // TODO(tech-debt): 将 CreditRatingRequest.credit_limit 改为 Option<Decimal>，由 service 层区分语义
        credit_limit: req.credit_limit.unwrap_or_default(),
        credit_days: req.credit_days,
        remark: req.remark,
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

    Ok(Json(ApiResponse::success_with_message(
        (),
        "客户信用已删除",
    )))
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
    info!(
        "用户 {} 正在评估客户 {} 的信用",
        auth.username, req.customer_id
    );

    let service = CustomerCreditService::new(state.db.clone());
    let result = service
        .evaluate_credit(req.customer_id, req.evaluation_date, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success(result)))
}
