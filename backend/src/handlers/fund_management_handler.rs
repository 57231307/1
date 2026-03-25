use crate::utils::error::AppError;
use crate::middleware::auth_context::AuthContext;
use crate::models::fund_management;
use crate::services::fund_management_service::FundManagementService;
use crate::utils::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;
use rust_decimal::Decimal;

/// 资金账户查询参数 DTO
#[derive(Debug, Deserialize)]
pub struct FundAccountQuery {
    pub account_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 创建资金账户请求 DTO
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFundAccountRequest {
    pub account_name: String,
    pub account_no: String,
    pub account_type: String,
    pub bank_name: Option<String>,
    pub currency: String,
    pub opened_date: Option<String>,
    pub remark: Option<String>,
}

/// 存款/取款请求 DTO
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundTransactionRequest {
    pub amount: Decimal,
    pub remark: Option<String>,
}

/// 冻结资金请求 DTO
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreezeFundsRequest {
    pub amount: Decimal,
    pub reason: String,
}

/// 获取资金账户列表
pub async fn list_accounts(
    Query(params): Query<FundAccountQuery>,
    State(db): State<Arc<DatabaseConnection>>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<fund_management::Model>>>, AppError> {
    info!("用户 {} 正在查询资金账户列表", auth.username);

    let service = FundManagementService::new(db);
    let query_params = crate::services::fund_management_service::FundAccountQueryParams {
        account_type: params.account_type,
        status: params.status,
        page: params.page.unwrap_or(0),
        page_size: params.page_size.unwrap_or(10),
    };

    let (accounts, _total) = service.get_accounts_list(query_params).await?;
    info!("资金账户列表查询成功，共 {} 条记录", accounts.len());

    Ok(Json(ApiResponse::success(accounts)))
}

/// 创建资金账户
#[axum::debug_handler]
pub async fn create_account(
    State(db): State<Arc<DatabaseConnection>>,
    auth: AuthContext,
    Json(req): Json<CreateFundAccountRequest>,
) -> Result<Json<ApiResponse<fund_management::Model>>, AppError> {
    info!("用户 {} 正在创建资金账户：{}", auth.username, req.account_no);

    let service = FundManagementService::new(db);
    let account = service.create_account(
        crate::services::fund_management_service::CreateFundAccountRequest {
            account_name: req.account_name,
            account_no: req.account_no,
            account_type: req.account_type,
            bank_name: req.bank_name,
            currency: req.currency,
            opened_date: req.opened_date.and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok()),
            remark: req.remark,
        },
        auth.user_id,
    ).await?;

    info!("资金账户创建成功：{}", account.account_no);
    Ok(Json(ApiResponse::success(account)))
}

/// 获取资金账户详情
pub async fn get_account(
    Path(id): Path<i32>,
    State(db): State<Arc<DatabaseConnection>>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<fund_management::Model>>, AppError> {
    info!("用户 {} 正在查询资金账户详情：{}", auth.username, id);

    let service = FundManagementService::new(db);
    let account = service.get_account_by_id(id).await?;

    info!("资金账户详情查询成功：{}", account.account_no);
    Ok(Json(ApiResponse::success(account)))
}

/// 账户存款
#[axum::debug_handler]
pub async fn deposit(
    Path(id): Path<i32>,
    State(db): State<Arc<DatabaseConnection>>,
    auth: AuthContext,
    Json(req): Json<FundTransactionRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在向账户 {} 存款 {:.2}", auth.username, id, req.amount);

    let service = FundManagementService::new(db);
    service.deposit(id, req.amount, auth.user_id, req.remark).await?;

    info!("账户 {} 存款成功", id);
    Ok(Json(ApiResponse::success("存款成功".to_string())))
}

/// 账户取款
#[axum::debug_handler]
pub async fn withdraw(
    Path(id): Path<i32>,
    State(db): State<Arc<DatabaseConnection>>,
    auth: AuthContext,
    Json(req): Json<FundTransactionRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在从账户 {} 取款 {:.2}", auth.username, id, req.amount);

    let service = FundManagementService::new(db);
    service.withdraw(id, req.amount, auth.user_id, req.remark).await?;

    info!("账户 {} 取款成功", id);
    Ok(Json(ApiResponse::success("取款成功".to_string())))
}

/// 冻结账户资金
#[axum::debug_handler]
pub async fn freeze_funds(
    Path(id): Path<i32>,
    State(db): State<Arc<DatabaseConnection>>,
    auth: AuthContext,
    Json(req): Json<FreezeFundsRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在冻结账户 {} 资金 {:.2}，原因：{}", auth.username, id, req.amount, req.reason);

    let service = FundManagementService::new(db);
    service.freeze_funds(id, req.amount, auth.user_id, req.reason).await?;

    info!("账户 {} 资金冻结成功", id);
    Ok(Json(ApiResponse::success("冻结成功".to_string())))
}

/// 解冻账户资金
#[axum::debug_handler]
pub async fn unfreeze_funds(
    Path(id): Path<i32>,
    State(db): State<Arc<DatabaseConnection>>,
    auth: AuthContext,
    Json(req): Json<FundTransactionRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在解冻账户 {} 资金 {:.2}", auth.username, id, req.amount);

    let service = FundManagementService::new(db);
    service.unfreeze_funds(id, req.amount, auth.user_id).await?;

    info!("账户 {} 资金解冻成功", id);
    Ok(Json(ApiResponse::success("解冻成功".to_string())))
}

/// 删除资金账户
pub async fn delete_account(
    Path(id): Path<i32>,
    State(db): State<Arc<DatabaseConnection>>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在删除资金账户：{}", auth.username, id);

    let service = FundManagementService::new(db);
    service.delete_account(id, auth.user_id).await?;

    info!("资金账户 {} 删除成功", id);
    Ok(Json(ApiResponse::success("删除成功".to_string())))
}
