use crate::services::auth_service::AuthService;
use crate::services::user_service::UserService;
use crate::utils::app_state::AppState;
use crate::utils::cache::Cache;
use crate::utils::response::ApiResponse;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 3, max = 50, message = "用户名长度必须在3到50个字符之间"))]
    pub username: String,
    #[validate(length(min = 6, message = "密码长度不能少于6个字符"))]
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

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    if let Err(errors) = payload.validate() {
        let error_msgs: Vec<String> = errors
            .field_errors()
            .iter()
            .map(|(field, errs)| {
                let msgs: Vec<String> = errs.iter().filter_map(|e| e.message.as_ref().map(|m| m.to_string())).collect();
                format!("{}: {}", field, msgs.join(", "))
            })
            .collect();
            
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!("输入验证失败: {}", error_msgs.join("; ")))),
        ));
    }

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

            let response = LoginResponse {
                token,
                user: user_info,
            };

            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let error_response = ApiResponse::<()>::error(e.to_string());
            Err((StatusCode::UNAUTHORIZED, Json(error_response)))
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub success: bool,
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<LogoutResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // 提取 Token
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "));

    if let Some(auth_header) = auth_header {
        let token = &auth_header[7..];

        // 验证 Token 是否有效
        match AuthService::validate_token_static(token, &state.jwt_secret) {
            Ok(claims) => {
                let now = chrono::Utc::now();
                let exp = claims.exp;
                
                if exp > now {
                    let ttl = match (exp - now).to_std() {
                        Ok(ttl) => ttl,
                        Err(_) => {
                            return Ok(Json(ApiResponse::success(LogoutResponse { success: true })));
                        }
                    };
                    // 将 Token 加入黑名单
                    state
                        .cache
                        .get_token_blacklist()
                        .set(token.to_string(), true, Some(ttl));
                    tracing::info!("Token blacklisted for user {}", claims.username);
                }
            }
            Err(e) => {
                tracing::warn!("Logout attempted with invalid token: {:?}", e);
            }
        }
    }

    Ok(Json(ApiResponse::success(LogoutResponse { success: true })))
}

#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub token: String,
    pub expires_in: u64,
}

pub async fn refresh_token(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<RefreshTokenResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let token = headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("缺少认证令牌")),
        ))?;

    if state
        .cache
        .get_token_blacklist()
        .get(&token.to_string())
        .is_some()
    {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("令牌已被撤销")),
        ));
    }

    let claims = AuthService::validate_token_static(token, &state.jwt_secret)
        .map_err(|_| (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("无效的令牌")),
        ))?;

    let auth_service = AuthService::new(state.db.clone(), state.jwt_secret.clone());
    let new_token = auth_service
        .generate_token(claims.sub, &claims.username, claims.role_id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("生成令牌失败：{}", e))),
            )
        })?;

    Ok(Json(ApiResponse::success(RefreshTokenResponse {
        token: new_token,
        expires_in: 86400,
    })))
}
