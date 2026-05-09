use crate::services::auth_service::AuthService;
use crate::services::user_service::UserService;
use crate::services::role_permission_service::RolePermissionService;
use crate::utils::response::ApiResponse;
use crate::utils::password_validator::{validate_password, get_password_feedback};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use crate::utils::app_state::AppState;
use serde::{Deserialize, Serialize};
use crate::middleware::auth_context::AuthContext;
use validator::{Validate, ValidationError};

fn validate_password_strength(password: &str) -> Result<(), ValidationError> {
    let result = validate_password(password);
    if result.is_valid {
        Ok(())
    } else {
        let msg = get_password_feedback(&result);
        let mut err = ValidationError::new("password_strength");
        err.message = Some(std::borrow::Cow::Owned(msg));
        Err(err)
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 3, max = 50, message = "用户名长度必须在3-50之间"))]
    pub username: String,
    #[validate(custom(function = "validate_password_strength"))]
    pub password: String,
    #[validate(email(message = "邮箱格式不正确"))]
    pub email: Option<String>,
    #[validate(length(min = 1, message = "电话号码不能为空"))]
    pub phone: Option<String>,
    pub role_id: Option<i32>,
    pub department_id: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserRequest {
    #[validate(email(message = "邮箱格式不正确"))]
    pub email: Option<String>,
    #[validate(length(min = 1, message = "电话号码不能为空"))]
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
    if let Err(e) = payload.validate() {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::error(e.to_string()))));
    }

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
    if let Err(e) = req.validate() {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::error(e.to_string()))));
    }

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
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<DeleteUserResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // 检查是否是删除自己的账户
    if auth.user_id != id {
        // 非自己账户需要权限检查
        let role_permission_service = RolePermissionService::new(state.db.clone());
        
        // 检查是否有权限删除用户
        let has_permission = role_permission_service
            .check_permission(
                auth.role_id.unwrap_or(0),
                "user",
                "delete",
                Some(id)
            )
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;
        
        if !has_permission {
            return Err((StatusCode::FORBIDDEN, Json(ApiResponse::error("没有删除用户的权限".to_string()))));
        }
    }

    let user_service = UserService::new(state.db.clone());

    // 检查用户是否存在
    user_service.find_by_id(id).await
        .map_err(|e| (StatusCode::NOT_FOUND, Json(ApiResponse::error(e.to_string()))))?;

    // 这里可以添加更多禁止删除的逻辑，例如：
    // 1. 系统管理员不允许删除
    // 2. 有特殊权限的用户不允许删除
    // 3. 正在使用中的用户不允许删除

    match user_service.delete_user(id).await {
        Ok(_) => Ok(Json(ApiResponse::success(DeleteUserResponse { success: true }))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(ApiResponse::error(e.to_string())))),
    }
}

/// 修改密码请求
#[derive(Debug, Deserialize, Validate)]
pub struct ChangePasswordRequest {
    #[validate(length(min = 1, message = "原密码不能为空"))]
    pub old_password: String,
    #[validate(custom(function = "validate_password_strength"))]
    pub new_password: String,
}

/// 修改密码响应
#[derive(Debug, Serialize)]
pub struct ChangePasswordResponse {
    pub success: bool,
    pub message: String,
}

/// 修改当前用户密码
pub async fn change_password(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<ApiResponse<ChangePasswordResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    if let Err(e) = req.validate() {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::error(e.to_string()))));
    }

    let user_service = UserService::new(state.db.clone());

    // 获取当前用户信息
    let user = user_service.find_by_id(auth.user_id).await
        .map_err(|e| (StatusCode::NOT_FOUND, Json(ApiResponse::error(e.to_string()))))?;

    // 验证原密码
    let is_valid = AuthService::verify_password(&req.old_password, &user.password_hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    if !is_valid {
        return Err((StatusCode::UNAUTHORIZED, Json(ApiResponse::error("原密码不正确".to_string()))));
    }

    // 检查新密码不能与原密码相同
    let is_same = AuthService::verify_password(&req.new_password, &user.password_hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    if is_same {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::error("新密码不能与原密码相同".to_string()))));
    }

    // 哈希新密码
    let new_password_hash = AuthService::hash_password(&req.new_password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    // 更新密码
    use sea_orm::ActiveModelTrait;
    let mut user_model: crate::models::user::ActiveModel = user.into();
    user_model.password_hash = sea_orm::Set(new_password_hash);
    user_model.updated_at = sea_orm::Set(chrono::Utc::now());

    user_model.update(state.db.as_ref()).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string()))))?;

    Ok(Json(ApiResponse::success_with_message(
        ChangePasswordResponse {
            success: true,
            message: "密码修改成功".to_string(),
        },
        "密码修改成功，请使用新密码重新登录",
    )))
}
