use crate::services::auth_service::AuthService;
use crate::services::user_service::UserService;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sea_orm::DatabaseConnection;
use crate::utils::app_state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub role_id: Option<i32>,
    pub department_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub role_id: Option<i32>,
    pub department_id: Option<i32>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i32,
    pub username: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub role_id: Option<i32>,
    pub department_id: Option<i32>,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct UserListResponse {
    pub users: Vec<UserResponse>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Serialize)]
pub struct DeleteUserResponse {
    pub success: bool,
}

pub async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<UserResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_service = UserService::new(state.db.clone());

    match user_service.find_by_id(id).await {
        Ok(user) => Ok(Json(ApiResponse::success(UserResponse {
            id: user.id,
            username: user.username,
            email: user.email,
            phone: user.phone,
            role_id: user.role_id,
            department_id: user.department_id,
            is_active: user.is_active,
            created_at: user.created_at,
        }))),
        Err(e) => Err((StatusCode::NOT_FOUND, Json(ApiResponse::error(e.to_string())))),
    }
}

pub async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<ApiResponse<UserResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_service = UserService::new(state.db.clone());

    let password_hash = AuthService::hash_password(&payload.password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    match user_service
        .create_user(
            payload.username,
            password_hash,
            payload.email,
            payload.phone,
            payload.role_id,
            payload.department_id,
        )
        .await
    {
        Ok(user) => Ok(Json(ApiResponse::success(UserResponse {
            id: user.id,
            username: user.username,
            email: user.email,
            phone: user.phone,
            role_id: user.role_id,
            department_id: user.department_id,
            is_active: user.is_active,
            created_at: user.created_at,
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string())))),
    }
}

pub async fn list_users(
    State(state): State<AppState>,
    Query(params): Query<ListUsersParams>,
) -> Result<Json<ApiResponse<UserListResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_service = UserService::new(state.db.clone());

    match user_service
        .list_users(params.page.unwrap_or(0), params.page_size.unwrap_or(20))
        .await
    {
        Ok((users, total)) => {
            let user_responses: Vec<UserResponse> = users
                .into_iter()
                .map(|user| UserResponse {
                    id: user.id,
                    username: user.username,
                    email: user.email,
                    phone: user.phone,
                    role_id: user.role_id,
                    department_id: user.department_id,
                    is_active: user.is_active,
                    created_at: user.created_at,
                })
                .collect();

            Ok(Json(ApiResponse::success(UserListResponse {
                users: user_responses,
                total,
                page: params.page.unwrap_or(0),
                page_size: params.page_size.unwrap_or(20),
            })))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string())))),
    }
}

#[derive(Debug, Deserialize)]
pub struct ListUsersParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

use axum::extract::Query;

/// 更新用户信息
pub async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<ApiResponse<UserResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_service = UserService::new(state.db.clone());

    match user_service
        .update_user(
            id,
            req.email,
            req.phone,
            req.role_id,
            req.department_id,
            req.status,
        )
        .await
    {
        Ok(user) => Ok(Json(ApiResponse::success(UserResponse {
            id: user.id,
            username: user.username,
            email: user.email,
            phone: user.phone,
            role_id: user.role_id,
            department_id: user.department_id,
            is_active: user.is_active,
            created_at: user.created_at,
        }))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(ApiResponse::error(e.to_string())))),
    }
}

/// 删除用户（软删除）
pub async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<DeleteUserResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_service = UserService::new(state.db.clone());

    match user_service.delete_user(id).await {
        Ok(_) => Ok(Json(ApiResponse::success(DeleteUserResponse { success: true }))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(ApiResponse::error(e.to_string())))),
    }
}
