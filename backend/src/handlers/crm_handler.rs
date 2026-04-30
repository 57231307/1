use crate::models::dto::crm_dto::{
    CreateLeadRequest, CreateOpportunityRequest, LeadQuery, OpportunityQuery,
};
use crate::models::dto::ApiResponse;
use crate::services::crm_service::CrmService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use axum::{
    extract::{Path, Query, State},
    Json,
};

pub async fn create_lead(
    State(state): State<AppState>,
    Json(req): Json<CreateLeadRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let res = service.create_lead(req, 1).await?; // TODO: Auth extraction
    Ok(Json(ApiResponse::success(
        serde_json::to_value(res).unwrap(),
    )))
}

pub async fn list_leads(
    State(state): State<AppState>,
    Query(query): Query<LeadQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let res = service.list_leads(query).await?;
    Ok(Json(ApiResponse::success(
        serde_json::to_value(res).unwrap(),
    )))
}

pub async fn update_lead_status(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let status = payload
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("NEW");
    let service = CrmService::new(state.db.clone());
    service.update_lead_status(id, status).await?;
    Ok(Json(ApiResponse::success("Status updated".to_string())))
}

pub async fn create_opportunity(
    State(state): State<AppState>,
    Json(req): Json<CreateOpportunityRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let res = service.create_opportunity(req, 1).await?;
    Ok(Json(ApiResponse::success(
        serde_json::to_value(res).unwrap(),
    )))
}

pub async fn list_opportunities(
    State(state): State<AppState>,
    Query(query): Query<OpportunityQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let res = service.list_opportunities(query).await?;
    Ok(Json(ApiResponse::success(
        serde_json::to_value(res).unwrap(),
    )))
}
