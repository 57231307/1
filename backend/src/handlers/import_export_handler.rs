use axum::{Json, extract::State, http::StatusCode};
use serde_json::json;
use crate::utils::app_state::AppState;

pub async fn import_csv(
    State(_state): State<AppState>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": {"imported": 0, "failed": 0}})))
}

pub async fn import_excel(
    State(_state): State<AppState>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": {"imported": 0, "failed": 0}})))
}

pub async fn download_template(
    State(_state): State<AppState>,
    axum::extract::Path(_import_type): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": null})))
}

pub async fn export_csv(
    State(_state): State<AppState>,
    axum::extract::Path(_export_type): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": null})))
}

pub async fn export_excel_type(
    State(_state): State<AppState>,
    axum::extract::Path(_export_type): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": null})))
}
