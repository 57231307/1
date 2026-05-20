use axum::{Json, extract::State, http::StatusCode};
use serde_json::json;
use crate::utils::app_state::AppState;

pub async fn create_customer(
    State(_state): State<AppState>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": null})))
}

pub async fn list_customers(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": {"items": [], "total": 0}})))
}

pub async fn get_customer(
    State(_state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": {"id": id}})))
}

pub async fn update_customer(
    State(_state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": {"id": id}})))
}

pub async fn delete_customer(
    State(_state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": null})))
}

pub async fn add_tags(
    State(_state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": null})))
}

pub async fn list_contacts(
    State(_state): State<AppState>,
    axum::extract::Path(customer_id): axum::extract::Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({"success": true, "data": {"items": [], "total": 0}})))
}
