use crate::middleware::auth_context::AuthContext;
use crate::models::sales_contract;
use crate::services::sales_contract_service::{
    CreateSalesContractRequest, ExecuteSalesContractRequest, SalesContractService,
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
use validator::Validate;

/// 销售合同查询参数 DTO
#[derive(Debug, Deserialize)]
pub struct SalesContractQuery {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 创建销售合同请求 DTO
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateSalesContractRequestDto {
    pub contract_no: String,
    pub contract_name: String,
    pub customer_id: i32,
    pub total_amount: rust_decimal::Decimal,
    pub payment_terms: Option<String>,
    pub delivery_date: chrono::NaiveDate,
    pub remark: Option<String>,
}

/// P1-2o 修复（批次 81 v1 复审）：更新销售合同请求 DTO
/// 替代 update_contract 中的 Json<serde_json::Value>，提供强类型校验
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct UpdateSalesContractDto {
    /// 合同名称：可选
    #[validate(length(max = 200, message = "合同名称长度不能超过200字符"))]
    pub contract_name: Option<String>,
    /// 付款条款：可选
    pub payment_terms: Option<String>,
}

/// 合同执行请求 DTO
#[derive(Debug, Deserialize)]
pub struct ExecuteSalesContractRequestDto {
    pub execution_type: String,
    pub execution_amount: rust_decimal::Decimal,
    pub related_bill_type: Option<String>,
    pub related_bill_id: Option<i32>,
    pub remark: Option<String>,
}

/// 取消合同请求 DTO
#[derive(Debug, Deserialize)]
pub struct CancelSalesContractRequest {
    pub reason: String,
}

/// 获取销售合同列表
pub async fn list_contracts(
    Query(params): Query<SalesContractQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<sales_contract::Model>>>, AppError> {
    info!("用户 {} 正在查询销售合同列表", auth.user_id);

    let service = SalesContractService::new(state.db.clone());
    let query_params = crate::services::sales_contract_service::SalesContractQueryParams {
        keyword: params.keyword,
        status: params.status,
        customer_id: params.customer_id,
        page: params.page.unwrap_or(1).max(1),
        page_size: params.page_size.unwrap_or(10).clamp(1, 100),
    };

    let (contracts, _total) = service.get_list(query_params).await?;
    info!("销售合同列表查询成功，共 {} 条记录", contracts.len());

    Ok(Json(ApiResponse::success(contracts)))
}

/// 获取销售合同详情
pub async fn get_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<sales_contract::Model>>, AppError> {
    info!("用户 {} 正在查询销售合同详情：{}", auth.user_id, id);

    let service = SalesContractService::new(state.db.clone());
    let contract = service.get_by_id(id).await?;
    info!("销售合同详情查询成功：{}", contract.contract_no);

    Ok(Json(ApiResponse::success(contract)))
}

/// 创建销售合同
#[axum::debug_handler]
pub async fn create_contract(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateSalesContractRequestDto>,
) -> Result<Json<ApiResponse<sales_contract::Model>>, AppError> {
    info!(
        "用户 {} 正在创建销售合同：{}",
        auth.user_id, req.contract_no
    );

    let service = SalesContractService::new(state.db.clone());
    let create_req = CreateSalesContractRequest {
        contract_no: req.contract_no,
        contract_name: req.contract_name,
        customer_id: req.customer_id,
        total_amount: req.total_amount,
        payment_terms: req.payment_terms,
        delivery_date: req.delivery_date,
        remark: req.remark,
    };

    let contract = service.create(create_req, auth.user_id).await?;
    info!("销售合同创建成功：{}", contract.contract_no);

    Ok(Json(ApiResponse::success(contract)))
}

/// 审核销售合同
pub async fn approve_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在审核销售合同 {}", auth.user_id, id);

    let service = SalesContractService::new(state.db.clone());
    service.approve(id, auth.user_id).await?;

    let message = format!("合同 {} 审核成功", id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}

/// 执行销售合同
#[axum::debug_handler]
pub async fn execute_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<ExecuteSalesContractRequestDto>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在执行销售合同 {}", auth.user_id, id);

    let service = SalesContractService::new(state.db.clone());
    let execute_req = ExecuteSalesContractRequest {
        execution_type: req.execution_type,
        execution_amount: req.execution_amount,
        related_bill_type: req.related_bill_type,
        related_bill_id: req.related_bill_id,
        remark: req.remark,
    };

    service.execute(id, execute_req, auth.user_id).await?;

    let message = format!("合同 {} 执行成功", id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}

/// 取消销售合同
#[axum::debug_handler]
pub async fn cancel_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CancelSalesContractRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在取消销售合同 {}", auth.user_id, id);

    let service = SalesContractService::new(state.db.clone());
    service.cancel(id, auth.user_id, req.reason).await?;

    let message = format!("合同 {} 取消成功", id);
    info!("{}", message);

    Ok(Json(ApiResponse::success(message)))
}

/// PUT /api/v1/erp/sales-contracts/:id - 更新销售合同
pub async fn update_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateSalesContractDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 更新销售合同: ID={}", auth.username, id);

    // P1-2o 修复（批次 81 v1 复审）：强类型 DTO + validator 替代 Json<Value>
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let service = SalesContractService::new(state.db.clone());

    // 获取现有合同
    let mut contract = service.get_by_id(id).await?;

    // 检查状态
    if contract.status != "draft" {
        return Err(AppError::validation(
            "只有草稿状态的合同才能修改".to_string(),
        ));
    }

    // 更新字段
    if let Some(name) = req.contract_name {
        contract.contract_name = name;
    }
    if let Some(terms) = req.payment_terms {
        contract.payment_terms = Some(terms);
    }

    // 保存更新
    use sea_orm::ActiveModelTrait;
    let mut active_model: crate::models::sales_contract::ActiveModel = contract.into();
    active_model.updated_at = sea_orm::Set(chrono::Utc::now());

    let updated = active_model.update(&*state.db).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(updated)?,
        "销售合同更新成功",
    )))
}

/// DELETE /api/v1/erp/sales-contracts/:id - 删除销售合同
pub async fn delete_contract(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 删除销售合同: ID={}", auth.username, id);

    let service = SalesContractService::new(state.db.clone());

    // 获取现有合同
    let contract = service.get_by_id(id).await?;

    // 检查状态
    if contract.status != "draft" {
        return Err(AppError::validation(
            "只有草稿状态的合同才能删除".to_string(),
        ));
    }

    // 软删除
    use sea_orm::ActiveModelTrait;
    let mut active_model: crate::models::sales_contract::ActiveModel = contract.into();
    active_model.status = sea_orm::Set("cancelled".to_string());
    active_model.updated_at = sea_orm::Set(chrono::Utc::now());

    active_model.update(&*state.db).await?;

    Ok(Json(ApiResponse::success_with_message(
        (),
        "销售合同已删除",
    )))
}
