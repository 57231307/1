use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::customer_credit_service::{
    CreditEvaluationResult, CreditLimitAdjustmentRequest, CreditQueryParams, CreditRatingRequest,
    CustomerCreditService, CustomerCreditView,
};
use crate::utils::error::AppError;
use crate::utils::{ApiResponse, PaginatedResponse};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::info;
use validator::Validate;

// 批次 98 P2-B 修复（v5 复审）：本地 validate_amount_range 已抽取到 utils::validator 模块，
// 统一追加 round_dp(2) 精度校验。#[validate(custom)] 引用改为 crate::utils::validator::validate_amount_range。

/// 客户信用查询参数 DTO
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CreditQuery {
    pub customer_id: Option<i32>,
    pub credit_level: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 创建/更新信用评级请求 DTO；P1-2b 修复（批次 81 v1 复审）：添加 Validate + 字段校验，用于 create_credit
/// 批次 414 技术债务修复：credit_limit 改为 Option<Decimal>，区分"未提供"与"显式置 0"
#[allow(dead_code, reason = "序列化/反序列化字段")]
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct CreditRatingRequestDto {
    pub customer_id: i32,
    #[validate(length(max = 20, message = "信用等级长度不能超过20字符"))]
    pub credit_level: Option<String>,
    pub credit_score: Option<i32>,
    /// 信用额度。None 表示更新时保持原值；Some(v) 表示显式设置（含 Some(0)）。 创建场景下 None 默认为 0。 validator
    /// 框架对 Option<T> 自动解包：None 跳过校验，Some(v) 调用 validate_credit_limit_range。
    #[validate(custom(function = "crate::utils::validator::validate_credit_limit_range"))]
    pub credit_limit: Option<Decimal>,
    pub credit_days: Option<i32>,
    #[validate(length(max = 500, message = "备注长度不能超过500字符"))]
    pub remark: Option<String>,
}

/// P1-2b 修复（批次 81 v1 复审）：更新客户信用请求 DTO
/// 用于 update_credit，所有字段可选（仅更新提交的字段）
#[allow(dead_code, reason = "序列化/反序列化字段")]
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
#[allow(dead_code, reason = "反序列化输入字段")]
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
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct CreditAmountRequest {
    #[validate(custom(function = "crate::utils::validator::validate_amount_range"))]
    pub amount: Decimal,
}

/// 写后回读：按客户自然键取富化视图，使创建/更新/设级三个端点的出参与列表、详情
/// 同一条 JOIN 链路（customer_name 只有一个来源）。
///
/// 刚写入的行回读不到属库内不一致，如实报内部错误并点名客户，绝不返回 None、
/// 也不降级成"未富化"的另一套出参。
async fn read_back_view(
    service: &CustomerCreditService,
    customer_id: i32,
) -> Result<CustomerCreditView, AppError> {
    service
        .get_view_by_customer_id(customer_id)
        .await?
        .ok_or_else(|| AppError::internal(format!("客户 {} 的信用评级写入后回读缺失", customer_id)))
}

/// 获取客户信用列表
///
/// 调用方为前端客户信用列表页，入参为查询条件与分页键 page 与 page_size；
/// 出参为分页信封 PaginatedResponse，字段 items 与 total 与 page 与 page_size 四键齐全，
/// 供前端分页器读取总数。items 的行载体与详情同一条 LEFT JOIN 富化链路
/// （CustomerCreditView，customer_name 唯一来源是 customers 真值），信封不改变行形状。
pub async fn list_credits(
    Query(params): Query<CreditQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<PaginatedResponse<CustomerCreditView>>>, AppError> {
    info!("用户 {} 正在查询客户信用列表", auth.user_id);

    let page = CustomerCreditService::clamp_page(params.page.unwrap_or(1));
    let page_size = params.page_size.unwrap_or(10).clamp(1, 100);

    let service = CustomerCreditService::new(state.db.clone());
    let query_params = CreditQueryParams {
        customer_id: params.customer_id,
        credit_level: params.credit_level,
        status: params.status,
        page,
        page_size,
    };

    let (credits, total) = service.get_list(query_params).await?;
    info!(
        "客户信用列表查询成功，共 {} 条记录，当前页 {} 条",
        total,
        credits.len()
    );

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        credits,
        total,
        page as u64,
        page_size as u64,
    ))))
}

/// 获取客户信用详情
///
/// 路径参数 {id} 是本资源的**客户自然键**，即 customer_credit_ratings.customer_id
/// （= customers.id；该列由迁移 m0082 施加全表唯一 + 外键，读写契约为每客户恒一行），
/// **不是** customer_credit_ratings.id 行主键。正因取客户自然键，本 handler 才能同时
/// 挂在 GET /customers/{id}/credit 与 GET /customer-credits/{id} 两个前缀下且两处
/// {id} 同义；GET /customer-credits/{id}/print 的 {id} 是行主键（print_service
/// 按 find_by_id 取单行），与本端点不同义，勿按本端点口径互改。
pub async fn get_credit(
    Path(customer_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<CustomerCreditView>>, AppError> {
    info!(
        "用户 {} 正在查询客户 {} 的信用详情",
        auth.user_id, customer_id
    );

    let service = CustomerCreditService::new(state.db.clone());
    let credit = service
        .get_view_by_customer_id(customer_id)
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
///
/// 客户由请求体 customer_id 指定；路径 {id} 只是路由占位，本 handler 不读取它。
/// 出参 customer_name 与列表/详情同一条富化链路（写后按 customer_id 回读），
/// 不在构造点填 None 或拼装名。
#[axum::debug_handler]
pub async fn set_credit_rating(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreditRatingRequestDto>,
) -> Result<Json<ApiResponse<CustomerCreditView>>, AppError> {
    info!(
        "用户 {} 正在设置客户 {} 的信用评级",
        auth.user_id, req.customer_id
    );

    let service = CustomerCreditService::new(state.db.clone());
    let rating_req = CreditRatingRequest {
        customer_id: req.customer_id,
        credit_level: req.credit_level,
        credit_score: req.credit_score,
        // credit_limit 直接透传 Option<Decimal>：None=保持原值，Some(v)=显式设置
        credit_limit: req.credit_limit,
        credit_days: req.credit_days,
        remark: req.remark,
    };

    let credit = service.set_credit_rating(rating_req, auth.user_id).await?;
    info!("客户 {} 信用评级设置成功", req.customer_id);

    Ok(Json(ApiResponse::success(
        read_back_view(&service, credit.customer_id).await?,
    )))
}

/// 占用信用额度
///
/// 路径参数 {id} 是**客户自然键** customer_credit_ratings.customer_id
/// （= customers.id），**不是** customer_credit_ratings.id 行主键（与 [get_credit] 同口径）。
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
///
/// 路径参数 {id} 是**客户自然键** customer_credit_ratings.customer_id
/// （= customers.id），**不是** customer_credit_ratings.id 行主键（与 [get_credit] 同口径）。
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
///
/// 路径参数 {id} 是**客户自然键** customer_credit_ratings.customer_id
/// （= customers.id），**不是** customer_credit_ratings.id 行主键（与 [get_credit] 同口径）。
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
///
/// 路径参数 {id} 是**客户自然键** customer_credit_ratings.customer_id
/// （= customers.id），**不是** customer_credit_ratings.id 行主键（与 [get_credit] 同口径）。
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

/// POST /api/v1/erp/crm/customer-credits - 创建客户信用
///
/// 客户由请求体 customer_id 指定。出参 customer_name 与列表/详情同一条富化链路
/// （写后按 customer_id 回读），不落在实体冗余列上。
pub async fn create_credit(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreditRatingRequestDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 创建客户信用", auth.username);

    // 强类型 DTO + validator 校验入参（不接受 Json<Value> 松类型）
    req.validate().map_err(AppError::from)?;

    let service = CustomerCreditService::new(state.db.clone());

    let rating_req = crate::services::customer_credit_service::CreditRatingRequest {
        customer_id: req.customer_id,
        credit_level: req.credit_level,
        credit_score: req.credit_score,
        // credit_limit 直接透传 Option<Decimal>
        credit_limit: req.credit_limit,
        credit_days: req.credit_days,
        remark: req.remark,
    };

    let credit = service.set_credit_rating(rating_req, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(read_back_view(&service, credit.customer_id).await?)?,
        "客户信用创建成功",
    )))
}

/// PUT /api/v1/erp/customer-credits/{id} - 更新客户信用
///
/// 路径参数 {id} 是**客户自然键** customer_credit_ratings.customer_id
/// （= customers.id，本表每客户恒一行），**不是** customer_credit_ratings.id
/// 行主键——与 [get_credit] 同口径；写侧一律按客户键定位唯一评级行。
/// 出参 customer_name 与列表/详情同一条富化链路（写后回读）。
pub async fn update_credit(
    Path(customer_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateCreditDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 更新客户 {} 的信用额度", auth.username, customer_id);

    // 强类型 DTO + validator 校验入参（不接受 Json<Value> 松类型）
    req.validate().map_err(AppError::from)?;

    let service = CustomerCreditService::new(state.db.clone());

    let rating_req = crate::services::customer_credit_service::CreditRatingRequest {
        customer_id,
        credit_level: req.credit_level,
        credit_score: req.credit_score,
        // credit_limit 直接透传 Option<Decimal>，
        // service 层区分 None（保持原值）与 Some(v)（显式设置，含 Some(0)）
        credit_limit: req.credit_limit,
        credit_days: req.credit_days,
        remark: req.remark,
    };

    let credit = service.set_credit_rating(rating_req, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(read_back_view(&service, credit.customer_id).await?)?,
        "客户信用更新成功",
    )))
}

/// DELETE /api/v1/erp/customer-credits/{id} - 删除客户信用
///
/// 路径参数 {id} 是**客户自然键** customer_credit_ratings.customer_id，
/// **不是** customer_credit_ratings.id 行主键（与 [get_credit] 同口径）。
/// 实际动作是软停用（status → inactive、保留原行），故本端点与
/// POST /customer-credits/{id}/deactivate 等价。
pub async fn delete_credit(
    Path(customer_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 删除客户 {} 的信用", auth.username, customer_id);

    let service = CustomerCreditService::new(state.db.clone());
    service.deactivate(customer_id, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        (),
        "客户信用已删除",
    )))
}

/// 信用评估模型请求
#[allow(dead_code, reason = "反序列化输入字段")]
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
