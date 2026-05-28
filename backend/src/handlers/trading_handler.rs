//! 交易管理 Handler
//!
//! 提供采购合同、销售合同、采购价格、销售价格的管理功能

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::services::purchase_contract_service::PurchaseContractService;
use crate::services::sales_contract_service::SalesContractService;
use crate::services::purchase_price_service::PurchasePriceService;
use crate::services::sales_price_service::SalesPriceService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 合同查询参数
#[derive(Debug, Deserialize)]
pub struct ContractQueryParams {
    pub status: Option<String>,
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 价格查询参数
#[derive(Debug, Deserialize)]
pub struct PriceQueryParams {
    pub product_id: Option<i32>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 创建采购合同请求
#[derive(Debug, Deserialize)]
pub struct CreatePurchaseContractRequest {
    pub contract_no: String,
    pub contract_name: String,
    pub supplier_id: i32,
    pub total_amount: rust_decimal::Decimal,
    pub payment_terms: Option<String>,
    pub delivery_date: chrono::NaiveDate,
    pub remark: Option<String>,
}

/// 创建销售合同请求
#[derive(Debug, Deserialize)]
pub struct CreateSalesContractRequest {
    pub contract_no: String,
    pub contract_name: String,
    pub customer_id: i32,
    pub total_amount: rust_decimal::Decimal,
    pub payment_terms: Option<String>,
    pub delivery_date: chrono::NaiveDate,
    pub remark: Option<String>,
}

/// GET /api/v1/erp/trading/sales-contracts - 获取销售合同列表
pub async fn list_sales_contracts(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ContractQueryParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SalesContractService::new(state.db.clone());

    let page = params.page.unwrap_or(1) as i64;
    let page_size = params.page_size.unwrap_or(20) as i64;

    let query = crate::services::sales_contract_service::SalesContractQueryParams {
        status: params.status,
        keyword: params.keyword,
        customer_id: None,
        page,
        page_size,
    };

    let (items, total) = service.get_list(query).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// POST /api/v1/erp/trading/sales-contracts - 创建销售合同
pub async fn create_sales_contract(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateSalesContractRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SalesContractService::new(state.db.clone());

    let contract = service.create(
        crate::services::sales_contract_service::CreateSalesContractRequest {
            contract_no: req.contract_no,
            contract_name: req.contract_name,
            customer_id: req.customer_id,
            total_amount: req.total_amount,
            payment_terms: req.payment_terms,
            delivery_date: req.delivery_date,
            remark: req.remark,
        },
        auth.user_id,
    )
    .await?;

    tracing::info!("用户 {} 创建销售合同: {}", auth.username, contract.contract_no);

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(contract)?,
        "销售合同创建成功",
    )))
}

/// GET /api/v1/erp/trading/purchase-contracts - 获取采购合同列表
pub async fn list_purchase_contracts(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ContractQueryParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseContractService::new(state.db.clone());

    let page = params.page.unwrap_or(1) as i64;
    let page_size = params.page_size.unwrap_or(20) as i64;

    let query = crate::services::purchase_contract_service::ContractQueryParams {
        status: params.status,
        keyword: params.keyword,
        supplier_id: None,
        page,
        page_size,
    };

    let (items, total) = service.get_list(query).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// POST /api/v1/erp/trading/purchase-contracts - 创建采购合同
pub async fn create_purchase_contract(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreatePurchaseContractRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseContractService::new(state.db.clone());

    let contract = service.create(
        crate::services::purchase_contract_service::CreateContractRequest {
            contract_no: req.contract_no,
            contract_name: req.contract_name,
            supplier_id: req.supplier_id,
            total_amount: req.total_amount,
            payment_terms: req.payment_terms,
            delivery_date: req.delivery_date,
            remark: req.remark,
        },
        auth.user_id,
    )
    .await?;

    tracing::info!("用户 {} 创建采购合同: {}", auth.username, contract.contract_no);

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(contract)?,
        "采购合同创建成功",
    )))
}

/// GET /api/v1/erp/trading/purchase-contracts/:id - 获取采购合同详情
pub async fn get_purchase_contract(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseContractService::new(state.db.clone());

    let contract = service.get_by_id(id).await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(contract)?)))
}

/// POST /api/v1/erp/trading/purchase-contracts/:id/approve - 审批采购合同
pub async fn approve_purchase_contract(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseContractService::new(state.db.clone());

    service.approve(id, auth.user_id).await?;

    tracing::info!("用户 {} 审批采购合同: ID={}", auth.username, id);

    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({"id": id}),
        "采购合同审批成功",
    )))
}

/// POST /api/v1/erp/trading/purchase-contracts/:id/execute - 执行采购合同
pub async fn execute_purchase_contract(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseContractService::new(state.db.clone());

    let execution_type = req.get("execution_type")
        .and_then(|v| v.as_str())
        .unwrap_or("delivery")
        .to_string();

    let execution_amount = req.get("execution_amount")
        .and_then(|v| v.as_f64())
        .map(|f| rust_decimal::Decimal::from_f64_retain(f).unwrap_or_default())
        .unwrap_or_default();

    let execute_req = crate::services::purchase_contract_service::ExecuteContractRequest {
        execution_type,
        execution_amount,
        execution_date: chrono::Utc::now().date_naive(),
        related_bill_type: None,
        related_bill_id: None,
        remark: None,
    };

    service.execute(id, execute_req, auth.user_id).await?;

    tracing::info!("用户 {} 执行采购合同: ID={}", auth.username, id);

    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({"id": id}),
        "采购合同执行成功",
    )))
}

/// GET /api/v1/erp/trading/sales-contracts/:id - 获取销售合同详情
pub async fn get_sales_contract(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SalesContractService::new(state.db.clone());

    let contract = service.get_by_id(id).await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(contract)?)))
}

/// POST /api/v1/erp/trading/sales-contracts/:id/approve - 审批销售合同
pub async fn approve_sales_contract(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SalesContractService::new(state.db.clone());

    service.approve(id, auth.user_id).await?;

    tracing::info!("用户 {} 审批销售合同: ID={}", auth.username, id);

    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({"id": id}),
        "销售合同审批成功",
    )))
}

/// GET /api/v1/erp/trading/purchase-prices - 获取采购价格列表
pub async fn list_purchase_prices(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<PriceQueryParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchasePriceService::new(state.db.clone());

    let page = params.page.unwrap_or(1) as i64;
    let page_size = params.page_size.unwrap_or(20) as i64;

    let query = crate::services::purchase_price_service::PurchasePriceQueryParams {
        product_id: params.product_id,
        supplier_id: None,
        status: params.status,
        page,
        page_size,
    };

    let (items, total) = service.get_prices_list(query).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// GET /api/v1/erp/trading/sales-prices - 获取销售价格列表
pub async fn list_sales_prices(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<PriceQueryParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SalesPriceService::new(state.db.clone());

    let page = params.page.unwrap_or(1) as i64;
    let page_size = params.page_size.unwrap_or(20) as i64;

    let query = crate::services::sales_price_service::SalesPriceQueryParams {
        product_id: params.product_id,
        customer_type: None,
        status: params.status,
        page,
        page_size,
    };

    let (items, total) = service.get_prices_list(query).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}
