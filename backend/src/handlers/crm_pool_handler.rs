use axum::{Json, extract::State, http::StatusCode};
use serde_json::json;
use crate::utils::app_state::AppState;

pub async fn list_pool(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": {"items": [], "total": 0}})))
}

pub async fn claim_from_pool(
    State(_state): State<AppState>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": null})))
}

pub async fn recycle_to_pool(
    State(_state): State<AppState>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": null})))
}
