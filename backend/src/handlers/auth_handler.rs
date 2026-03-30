use crate::services::auth_service::AuthService;
use crate::services::user_service::UserService;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use once_cell::sync::OnceCell;

// 全局 JWT secret，在 main 函数中初始化
static JWT_SECRET: OnceCell<String> = OnceCell::new();

pub fn set_jwt_secret(secret: String) {
    let _ = JWT_SECRET.set(secret);
}

pub fn get_jwt_secret() -> String {
    JWT_SECRET.get().cloned().unwrap_or_else(|| "secret".to_string())
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: i32,
    pub username: String,
    pub email: Option<String>,
    pub role_id: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

pub async fn login(
    State(db): State<Arc<DatabaseConnection>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    let auth_service = AuthService::new(db.clone(), get_jwt_secret());

    match auth_service
        .authenticate(&payload.username, &payload.password)
        .await
    {
        Ok((token, user)) => {
            let user_info = UserInfo {
                id: user.id,
                username: user.username.clone(),
                email: user.email.clone(),
                role_id: user.role_id,
            };

            Ok(Json(LoginResponse {
                token,
                user: user_info,
            }))
        }
        Err(e) => {
            let error_response = ErrorResponse {
                error: "authentication_failed".to_string(),
                message: e.to_string(),
            };
            Err((StatusCode::UNAUTHORIZED, Json(error_response)))
        }
    }
}

/// 用户注销
#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub success: bool,
    pub message: String,
}

pub async fn logout(
    State(db): State<Arc<DatabaseConnection>>,
    headers: HeaderMap,
) -> Result<Json<LogoutResponse>, (StatusCode, String)> {
    // 从 Token 中获取用户 ID
    let token = headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "缺少认证令牌".to_string()))?;

    // 验证 Token 并获取 Claims
    let claims =
        AuthService::validate_token_static(token, &get_jwt_secret()).map_err(|_| (StatusCode::UNAUTHORIZED, "无效的令牌".to_string()))?;

    // 更新用户最后登录时间（设置为 None 表示注销）
    let _user_service = UserService::new(db.clone());
    let _user_id = claims.sub;

    // 可选：将 Token 加入黑名单（如果需要立即失效）
    // 这里暂不实现 Token 黑名单，依赖 Token 自然过期

    Ok(Json(LogoutResponse {
        success: true,
        message: "注销成功".to_string(),
    }))
}

/// 刷新 Token
#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub token: String,
    pub expires_in: u64,
}

pub async fn refresh_token(
    State(db): State<Arc<DatabaseConnection>>,
    headers: HeaderMap,
) -> Result<Json<RefreshTokenResponse>, (StatusCode, String)> {
    // 从 Token 中获取用户信息
    let token = headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "缺少认证令牌".to_string()))?;

    // 验证旧 Token 并获取 Claims
    let claims =
        AuthService::validate_token_static(token, &get_jwt_secret()).map_err(|_| (StatusCode::UNAUTHORIZED, "无效的令牌".to_string()))?;

    // 使用 Claims 中的信息生成新 Token
    let auth_service = AuthService::new(db.clone(), get_jwt_secret());
    let new_token = auth_service
        .generate_token(claims.sub, &claims.username, claims.role_id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("生成令牌失败：{}", e),
            )
        })?;

    Ok(Json(RefreshTokenResponse {
        token: new_token,
        expires_in: 86400, // 24 小时
    }))
}
