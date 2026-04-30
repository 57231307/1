use crate::middleware::auth_context::AuthContext;
use crate::models::purchase_contract;
use crate::services::purchase_contract_service::{
    CreateContractRequest, ExecuteContractRequest, PurchaseContractService,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::info;

/// 合同查询参数 DTO
#[derive(Debug, Deserialize)]
pub struct ContractQuery {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 创建采购合同请求 DTO
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateContractRequestDto {
    pub contract_no: String,
    pub contract_name: String,
    pub supplier_id: i32,
    pub total_amount: rust_decimal::Decimal,
    pub payment_terms: Option<String>,
    pub delivery_date: chrono::NaiveDate,
    pub remark: Option<String>,
}

/// 合同执行请求 DTO
#[derive(Debug, Deserialize)]
pub struct ExecuteContractRequestDto {
    pub execution_type: String,
    pub execution_amount: rust_decimal::Decimal,
    pub execution_date: chrono::NaiveDate,
    pub related_bill_type: Option<String>,
    pub related_bill_id: Option<i32>,
    pub remark: Option<String>,
}

/// 取消合同请求 DTO
#[derive(Debug, Deserialize)]
pub struct CancelContractRequest {
    pub reason: String,
}

/// 获取合同列表
pub async fn list_contracts(
    Query(params): Query<ContractQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<purchase_contract::Model>>>, AppError> {
    info!("用户 {} 正在查询采购合同列表", auth.user_id);

    let service = PurchaseContractService::new(state.db.clone());
    let query_params = crate::services::purchase_contract_service::ContractQueryParams {
        keyword: params.keyword,
        status: params.status,
        supplier_id: params.supplier_id,
        page: params.page.unwrap_or(0),
        page_size: params.page_size.unwrap_or(10),
    };

    let (contracts, _total) = service.get_list(query_params).await?;
    info!("采购合同列表查询成功，共 {} 条记录", contracts.len());

    Ok(Json(ApiResponse::success(contracts)))
}

/// 获取合同详情
pub async fn get_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<purchase_contract::Model>>, AppError> {
    info!("用户 {} 正在查询采购合同详情：{}", auth.user_id, id);

    let service = PurchaseContractService::new(state.db.clone());
    let contract = service.get_by_id(id).await?;
    info!("采购合同详情查询成功：{}", contract.contract_no);

    Ok(Json(ApiResponse::success(contract)))
}

/// 创建合同
#[axum::debug_handler]
pub async fn create_contract(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateContractRequestDto>,
) -> Result<Json<ApiResponse<purchase_contract::Model>>, AppError> {
    info!(
        "用户 {} 正在创建采购合同：{}",
        auth.user_id, req.contract_no
    );

    let service = PurchaseContractService::new(state.db.clone());
    let create_req = CreateContractRequest {
        contract_no: req.contract_no,
        contract_name: req.contract_name,
        supplier_id: req.supplier_id,
        total_amount: req.total_amount,
        payment_terms: req.payment_terms,
        delivery_date: req.delivery_date,
        remark: req.remark,
    };

    let contract = service.create(create_req, auth.user_id).await?;
    info!("采购合同创建成功：{}", contract.contract_no);

    Ok(Json(ApiResponse::success(contract)))
}

/// 审核合同
pub async fn approve_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在审核采购合同 {}", auth.user_id, id);

    let service = PurchaseContractService::new(state.db.clone());
    service.approve(id, auth.user_id).await?;

    let message = format!("合同 {} 审核成功", id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}

/// 执行合同
#[axum::debug_handler]
pub async fn execute_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<ExecuteContractRequestDto>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在执行采购合同 {}", auth.user_id, id);

    let service = PurchaseContractService::new(state.db.clone());
    let execute_req = ExecuteContractRequest {
        execution_type: req.execution_type,
        execution_amount: req.execution_amount,
        execution_date: req.execution_date,
        related_bill_type: req.related_bill_type,
        related_bill_id: req.related_bill_id,
        remark: req.remark,
    };

    service.execute(id, execute_req, auth.user_id).await?;

    let message = format!("合同 {} 执行成功", id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}

/// 取消合同
#[axum::debug_handler]
pub async fn cancel_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CancelContractRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在取消采购合同 {}", auth.user_id, id);

    let service = PurchaseContractService::new(state.db.clone());
    service.cancel(id, auth.user_id, req.reason).await?;

    let message = format!("合同 {} 取消成功", id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}

/// 更新合同
pub async fn update_contract(
    Path(_id): Path<i32>,
    State(_state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在更新采购合同", auth.user_id);
    Err(AppError::ValidationError(
        "合同更新功能尚未实现".to_string(),
    ))
}

/// 删除合同
pub async fn delete_contract(
    Path(_id): Path<i32>,
    State(_state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在删除采购合同", auth.user_id);
    Err(AppError::ValidationError(
        "合同删除功能尚未实现".to_string(),
    ))
}
