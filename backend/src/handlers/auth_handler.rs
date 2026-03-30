use crate::services::auth_service::AuthService;
use crate::services::user_service::UserService;
use crate::utils::app_state::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};

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
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    let auth_service = AuthService::new(state.db.clone(), state.jwt_secret.clone());

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

#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub success: bool,
    pub message: String,
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<LogoutResponse>, (StatusCode, String)> {
    let token = headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "缺少认证令牌".to_string()))?;

    let claims =
        AuthService::validate_token_static(token, &state.jwt_secret).map_err(|_| (StatusCode::UNAUTHORIZED, "无效的令牌".to_string()))?;

    let _user_service = UserService::new(state.db.clone());
    let _user_id = claims.sub;

    Ok(Json(LogoutResponse {
        success: true,
        message: "注销成功".to_string(),
    }))
}

#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub token: String,
    pub expires_in: u64,
}

pub async fn refresh_token(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<RefreshTokenResponse>, (StatusCode, String)> {
    let token = headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "缺少认证令牌".to_string()))?;

    let claims =
        AuthService::validate_token_static(token, &state.jwt_secret).map_err(|_| (StatusCode::UNAUTHORIZED, "无效的令牌".to_string()))?;

    let auth_service = AuthService::new(state.db.clone(), state.jwt_secret.clone());
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
        expires_in: 86400,
    }))
}
