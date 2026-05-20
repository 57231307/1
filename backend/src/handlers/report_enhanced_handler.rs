use axum::{Json, extract::State, http::StatusCode};
use serde_json::json;
use crate::utils::app_state::AppState;

pub async fn create_report_template(
    State(_state): State<AppState>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": null})))
}

pub async fn list_report_templates(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": {"items": [], "total": 0}})))
}

pub async fn export_pdf(
    State(_state): State<AppState>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": null})))
}

pub async fn export_excel(
    State(_state): State<AppState>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": null})))
}

pub async fn create_subscription(
    State(_state): State<AppState>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": null})))
}

pub async fn list_subscriptions(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": {"items": [], "total": 0}})))
}
