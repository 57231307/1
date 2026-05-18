use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::middleware::auth_context::AuthContext;
use crate::services::ar_reconciliation_service::{
    ArReconciliationService, CreateReconciliationRequest, ReconciliationQuery,
};
use crate::utils::app_state::AppState;
use crate::utils::response::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct CreateReconciliationApiRequest {
    pub reconciliation_no: String,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub opening_balance: Decimal,
    pub total_invoices: Decimal,
    pub total_collections: Decimal,
}

#[derive(Debug, Serialize)]
pub struct ReconciliationResponse {
    pub id: i32,
    pub reconciliation_no: String,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub period_start: String,
    pub period_end: String,
    pub opening_balance: String,
    pub total_invoices: String,
    pub total_collections: String,
    pub closing_balance: String,
    pub reconciliation_status: Option<String>,
    pub created_at: String,
}

impl From<crate::models::ar_reconciliation::Model> for ReconciliationResponse {
    fn from(model: crate::models::ar_reconciliation::Model) -> Self {
        Self {
            id: model.id,
            reconciliation_no: model.reconciliation_no,
            customer_id: model.customer_id,
            customer_name: model.customer_name,
            period_start: model.period_start.to_string(),
            period_end: model.period_end.to_string(),
            opening_balance: model.opening_balance.to_string(),
            total_invoices: model.total_invoices.to_string(),
            total_collections: model.total_collections.to_string(),
            closing_balance: model.closing_balance.to_string(),
            reconciliation_status: model.reconciliation_status,
            created_at: model.created_at.to_rfc3339(),
        }
    }
}

pub async fn create_reconciliation(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateReconciliationApiRequest>,
) -> Result<Json<ApiResponse<ReconciliationResponse>>, StatusCode> {
    let service = ArReconciliationService::new(state.db);
    let create_req = CreateReconciliationRequest {
        reconciliation_no: req.reconciliation_no,
        customer_id: req.customer_id,
        customer_name: req.customer_name,
        period_start: req.period_start,
        period_end: req.period_end,
        opening_balance: req.opening_balance,
        total_invoices: req.total_invoices,
        total_collections: req.total_collections,
        notes: None,
    };

    match service.create(create_req).await {
        Ok(model) => Ok(Json(ApiResponse::success(ReconciliationResponse::from(model)))),
        Err(e) => {
            tracing::error!("创建对账单失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListReconciliationsQuery {
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

pub async fn list_reconciliations(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<ListReconciliationsQuery>,
) -> Result<Json<ApiResponse<Vec<ReconciliationResponse>>>, StatusCode> {
    let service = ArReconciliationService::new(state.db);
    let req = ReconciliationQuery {
        status: query.status,
        customer_id: query.customer_id,
        page: query.page.unwrap_or(1),
        page_size: query.page_size.unwrap_or(20),
    };

    match service.list(req).await {
        Ok((models, _total)) => {
            let responses: Vec<ReconciliationResponse> = models.into_iter().map(ReconciliationResponse::from).collect();
            Ok(Json(ApiResponse::success(responses)))
        }
        Err(e) => {
            tracing::error!("获取对账单列表失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_reconciliation(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<ReconciliationResponse>>, StatusCode> {
    let service = ArReconciliationService::new(state.db);

    match service.get_by_id(id).await {
        Ok(Some(model)) => Ok(Json(ApiResponse::success(ReconciliationResponse::from(model)))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("获取对账单失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
}

pub async fn update_reconciliation_status(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateStatusRequest>,
) -> Result<Json<ApiResponse<ReconciliationResponse>>, StatusCode> {
    let service = ArReconciliationService::new(state.db);

    match service.update_status(id, &req.status).await {
        Ok(model) => Ok(Json(ApiResponse::success(ReconciliationResponse::from(model)))),
        Err(e) => {
            tracing::error!("更新对账单状态失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
