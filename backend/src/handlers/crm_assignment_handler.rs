use axum::{Json, extract::State, http::StatusCode};
use serde_json::json;
use crate::utils::app_state::AppState;

pub async fn assign_customer(
    State(_state): State<AppState>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": null})))
}

pub async fn batch_assign(
    State(_state): State<AppState>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": {"assigned": 0}})))
}

pub async fn list_assignment_history(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": {"items": [], "total": 0}})))
}
