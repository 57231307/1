use crate::services::auth_service::AuthService;
use crate::services::totp_service::TotpService;
use crate::utils::app_state::AppState;
use crate::utils::cache::Cache;
use crate::utils::response::ApiResponse;
use crate::middleware::auth_context::AuthContext;
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};
use axum::{
    extract::{State, Extension},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use serde::{Deserialize, Serialize};
use validator::Validate;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(length(min = 3, max = 50, message = "用户名长度必须在3到50个字符之间"))]
    pub username: String,
    pub password: String,
    // 可选：如果用户开启了 TOTP，则必须在登录时传入此项
    pub totp_token: Option<String>,
}


#[derive(Debug, Serialize, ToSchema)]
pub struct UserPermissionDto {
    pub resource: String,
    pub action: String,
    pub resource_id: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
    pub permissions: Vec<UserPermissionDto>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserInfo {
    pub id: i32,
    pub username: String,
    pub email: Option<String>,
    pub role_id: Option<i32>,
}

#[utoipa::path(
    post,
    path = "/api/v1/erp/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "登录成功", body = ApiResponse<LoginResponse>),
        (status = 400, description = "请求参数错误"),
        (status = 401, description = "未授权或密码错误")
    ),
    tags = ["Auth"]
)]
pub async fn login(
    State(state): State<AppState>,
    jar: axum_extra::extract::PrivateCookieJar,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ApiResponse<()>>)> {
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
            // 验证 TOTP 逻辑 (如已开启)
            if user.is_totp_enabled {
                let totp_token = match payload.totp_token {
                    Some(ref t) => t,
                    None => return Err((StatusCode::UNAUTHORIZED, Json(ApiResponse::error("需要提供两步验证码".to_string())))),
                };
                
                let totp_service = TotpService::new(state.db.clone());
                match totp_service.verify_login_totp(user.id, totp_token).await {
                    Ok(true) => {}, // 验证通过
                    _ => return Err((StatusCode::UNAUTHORIZED, Json(ApiResponse::error("两步验证码错误".to_string())))),
                }
            }

            let mut permissions = vec![];
            if let Some(role_id) = user.role_id {
                let role_perms = crate::models::role_permission::Entity::find()
                    .filter(crate::models::role_permission::Column::RoleId.eq(role_id))
                    .filter(crate::models::role_permission::Column::Allowed.eq(true))
                    .all(state.db.as_ref())
                    .await
                    .unwrap_or_default();
                    
                permissions = role_perms.into_iter().map(|p| UserPermissionDto {
                    resource: p.resource_type,
                    action: p.action,
                    resource_id: p.resource_id,
                }).collect();
            }

            let user_info = UserInfo {
                id: user.id,
                username: user.username.clone(),
                email: user.email.clone(),
                role_id: user.role_id,
            };

            let response = LoginResponse {
                token: token.clone(),
                user: user_info,
                permissions,
            };

            // 创建 HttpOnly Cookie
            let cookie = Cookie::build(("jwt", token))
                .path("/")
                .http_only(true)
                .secure(true) // 生产环境应开启 HTTPS
                .same_site(SameSite::Strict)
                .max_age(time::Duration::hours(24))
                .build();

            let jar = jar.add(cookie);

            Ok((jar, Json(ApiResponse::success(response))).into_response())
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
    jar: axum_extra::extract::PrivateCookieJar,
    headers: HeaderMap,
) -> Result<axum::response::Response, (StatusCode, Json<ApiResponse<()>>)> {
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
                let now = chrono::Utc::now().timestamp() as usize;
                let exp = claims.exp.timestamp() as usize;
                
                if exp > now {
                    let ttl = std::time::Duration::from_secs((exp - now) as u64);
                    // 将 Token 加入黑名单
                    state.cache.get_token_blacklist().set(token.to_string(), true, Some(ttl));
                    tracing::info!("Token blacklisted for user {}", claims.username);
                }
            }
            Err(e) => {
                tracing::warn!("Logout attempted with invalid token: {:?}", e);
            }
        }
    }

    let removal_cookie = axum_extra::extract::cookie::Cookie::build(("jwt", ""))
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(axum_extra::extract::cookie::SameSite::Strict)
        .max_age(time::Duration::ZERO)
        .build();
        
    let jar = jar.add(removal_cookie);

    Ok((jar, axum::Json(ApiResponse::success(LogoutResponse { success: true }))).into_response())
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

#[derive(Debug, Serialize)]
pub struct TotpSetupResponse {
    pub secret: String,
    pub qr_code: String,
}

/// 1. 获取 TOTP 绑定信息 (需登录)
pub async fn setup_totp(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<ApiResponse<TotpSetupResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let totp_service = TotpService::new(state.db.clone());
    
    match totp_service.generate_totp_secret(auth.user_id, &auth.username).await {
        Ok((secret, qr_code)) => Ok(Json(ApiResponse::success(TotpSetupResponse { secret, qr_code }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string())))),
    }
}

#[derive(Debug, Deserialize)]
pub struct TotpVerifyRequest {
    pub token: String,
}

/// 2. 验证并正式启用 TOTP (需登录)
pub async fn enable_totp(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(payload): Json<TotpVerifyRequest>,
) -> Result<Json<ApiResponse<bool>>, (StatusCode, Json<ApiResponse<()>>)> {
    let totp_service = TotpService::new(state.db.clone());
    
    match totp_service.verify_and_enable(auth.user_id, &payload.token).await {
        Ok(true) => Ok(Json(ApiResponse::success_with_msg(true, "双因素认证已成功开启"))),
        Ok(false) => Err((StatusCode::BAD_REQUEST, Json(ApiResponse::error("验证码不正确".to_string())))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(e.to_string())))),
    }
}
