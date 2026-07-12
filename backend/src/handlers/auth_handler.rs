use crate::middleware::audit_context::AuditContext;
// v9 P1-G 修复：移除未使用的 AuthContext import
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::auth_service::AuthService;
use crate::services::enhanced_logger::{
    self, DeviceInfo, FailureInfo, LoginAttempt, LoginSecurityLog, SecurityInfo,
};
use crate::services::totp_service::TotpService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use super::auth_handler_session::record_login_attempt;
use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use chrono::{Duration as ChronoDuration, Utc};
use time::Duration as CookieDuration;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use validator::Validate;

const MAX_FAILED_ATTEMPTS: i32 = 5;
const LOCKOUT_DURATION_MINUTES: i64 = 30;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(length(min = 3, max = 50, message = "用户名长度必须在3到50个字符之间"))]
    pub username: String,
    // TS-S-5 安全加固（2026-06-26）：密码长度校验
    // P2 7-7 修复：登录入口仅做基本格式检查（min=6），不强制 PasswordPolicy（min_length=8）。
    // 原因：密码策略在注册/修改时强制，若登录入口也强制 min=8，历史密码长度 6-7 的用户将无法登录。
    // 密码强度验证在 change_password / reset_password / register 接口由 PasswordPolicy 强制执行。
    #[validate(length(min = 6, max = 128, message = "密码长度必须在6到128个字符之间"))]
    pub password: String,
    // 可选：如果用户开启了 TOTP，则必须在登录时传入此项
    #[validate(length(max = 10, message = "TOTP令牌长度不能超过10个字符"))]
    pub totp_token: Option<String>,
    // v11 批次 141：可选恢复码，当 totp_token 缺失时可用恢复码替代
    #[validate(length(max = 32, message = "恢复码长度不能超过32个字符"))]
    pub recovery_code: Option<String>,
}

// 安全漏洞 #14 修复：LoginResponse 的 permissions 字段改为 `Vec<String>` 资源标识符
// 格式 `"{resource}:{action}"`（如 "user.list:read"），前端可直接 `permissions.includes("user.list:read")` 判断。
// 原 `UserPermissionDto { resource, action, resource_id }` 结构体已被删除（无其他引用）。

/// 登录响应 DTO
/// - 不再返回 `token`（#10）：access_token 已在 httpOnly Cookie 写入
/// - 不再返回 `refresh_token`（#13）：refresh_token 已在 httpOnly Cookie 写入
/// - 仍返回 `csrf_token`：前端 form header 需携带，且由非 httpOnly Cookie 暴露给 JS
#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub csrf_token: String,
    pub user: UserInfo,
    /// 资源标识符列表（`"{resource}:{action}"` 格式）
    pub permissions: Vec<String>,
    /// 密码是否过期（批次 198 P0-2：true 表示超过 90 天未修改，前端引导改密）
    pub password_expired: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserInfo {
    pub id: i32,
    pub username: String,
    pub email: Option<String>,
    pub role_id: Option<i32>,
    /// 批次 24 v6 P0-2 修复：补全 role_name 字段，前端路由守卫依赖此字段判断 admin 绕过。
    /// 从 role 表 JOIN 获取，None 表示用户未分配角色或角色不存在。
    pub role_name: Option<String>,
    /// 批次 24 v6 P0-2 修复：补全 permissions 字段，前端刷新页面后从 /auth/me 获取权限列表。
    /// 与 LoginResponse 顶层 permissions 格式一致（`"{resource}:{action}"`）。
    pub permissions: Vec<String>,
    /// 批次 29 v7 P0-5 修复：补全 phone 字段（users 表已有此列），前端用户中心展示用
    pub phone: Option<String>,
    /// 批次 29 v7 P0-5 修复：补全 department_id 字段（users 表已有此列），前端用户中心展示用
    pub department_id: Option<i32>,
    /// 批次 29 v7 P0-5 修复：补全 department_name 字段，从 departments 表 JOIN 获取。
    /// None 表示用户未分配部门或部门不存在。
    pub department_name: Option<String>,
    /// 批次 29 v7 P0-4 修复：补全 is_totp_enabled 字段（users 表已有此列）。
    /// 前端 2FA 检测依赖此字段，缺失会导致前端恒判 false，已开启 2FA 的用户也被引导再次设置。
    pub is_totp_enabled: bool,
    /// 批次 29 v7 P0-5 修复：real_name 字段当前 users 表无对应列，暂返回 None。
    /// TODO(tech-debt): 后续若新增 real_name 列，需在此处补全查询。
    pub real_name: Option<String>,
    /// 批次 29 v7 P0-5 修复：avatar 字段当前 users 表无对应列，暂返回 None。
    /// TODO(tech-debt): 后续若新增 avatar 列，需在此处补全查询。
    pub avatar: Option<String>,
}

impl UserInfo {
    /// 批次 24 v6 P0-2 修复：构建包含 role_name 和 permissions 的 UserInfo。
    /// 此函数供 login 和 get_current_user 共用，确保前后端类型契约一致。
    ///
    /// - `role_name`：从 role 表 code 字段获取（None 表示未分配角色）
    /// - `permissions`：从 role_permission 表查询 allowed=true 的记录，格式 `"{resource}:{action}"`
    /// - `department_name`：批次 29 v7 P0-5 新增，从 departments 表 JOIN 获取
    pub async fn build_with_permissions(
        db: &sea_orm::DatabaseConnection,
        user: &crate::models::user::Model,
    ) -> Self {
        let role_name = if let Some(role_id) = user.role_id {
            crate::models::role::Entity::find_by_id(role_id)
                .one(db)
                .await
                .ok()
                .flatten()
                .map(|r| r.code)
        } else {
            None
        };

        let permissions: Vec<String> = if let Some(role_id) = user.role_id {
            // P2 1-12 修复：系统管理员角色（is_system=true）注入 *:* 通配权限，
            // 替代前端 role_name === 'admin' 字符串硬编码绕过，统一走权限码校验
            let role_model = crate::models::role::Entity::find_by_id(role_id)
                .one(db)
                .await
                .ok()
                .flatten();

            if let Some(ref role) = role_model {
                if role.is_system {
                    vec!["*:*".to_string()]
                } else {
                    crate::models::role_permission::Entity::find()
                        .filter(crate::models::role_permission::Column::RoleId.eq(role_id))
                        .filter(crate::models::role_permission::Column::Allowed.eq(true))
                        .all(db)
                        .await
                        .map(|perms| {
                            perms
                                .into_iter()
                                .map(|p| format!("{}:{}", p.resource_type, p.action))
                                .collect()
                        })
                        .unwrap_or_default()
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // 批次 29 v7 P0-5 修复：JOIN departments 表获取 department_name
        let department_name = if let Some(dept_id) = user.department_id {
            crate::models::department::Entity::find_by_id(dept_id)
                .one(db)
                .await
                .ok()
                .flatten()
                .map(|d| d.name)
        } else {
            None
        };

        UserInfo {
            id: user.id,
            username: user.username.clone(),
            email: user.email.clone(),
            role_id: user.role_id,
            role_name,
            permissions,
            phone: user.phone.clone(),
            department_id: user.department_id,
            department_name,
            is_totp_enabled: user.is_totp_enabled,
            // 批次 29 v7 P0-5：users 表当前无 real_name / avatar 列，暂返回 None
            real_name: None,
            avatar: None,
        }
    }
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
// 防御性 allow：
// - redundant_clone：函数内多处 `.clone()`（payload.username / client_ip / csrf_token / token
//   等）虽然当前都是必要消费，但 wave 3+ 接入新审计字段 / 多设备 session 跟踪时
//   局部 clone 形态变化可能误报，预先抑制避免 CI 抖动。
// - unused_variables：`rotated` / `csrf_ip` / `csrf_token` 在 wave 3 #7 强制轮换分支中
//   短期作为中间值使用，未来若拆分到 helper 函数时可能暂时未消费，保留标注。
// - needless_pass_by_value：axum Json 提取器要求 owned LoginRequest，无法改为引用。
// login 函数
// 批次 340 v11 复审 P1 修复：移除 `#[allow(clippy::redundant_clone)]` 抑制，
// baseline 无此警告，原标注为防御性抑制。若 CI 报 redundant_clone 则需重构 clone 为引用传递。
pub async fn login(
    State(state): State<AppState>,
    audit_ctx: Option<Extension<AuditContext>>,
    jar: axum_extra::extract::PrivateCookieJar,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<axum::response::Response, AppError> {
    if let Err(errors) = payload.validate() {
        let error_msgs: Vec<String> = errors
            .field_errors()
            .iter()
            .map(|(field, errs)| {
                let msgs: Vec<String> = errs
                    .iter()
                    .filter_map(|e| e.message.as_ref().map(|m| m.to_string()))
                    .collect();
                format!("{}: {}", field, msgs.join(", "))
            })
            .collect();

        return Err(AppError::bad_request(format!(
            "输入验证失败: {}",
            error_msgs.join("; ")
        )));
    }

    // P2-12c 修复（批次 83 v1 复审）：IP 提取统一优先级（X-Real-IP → X-Forwarded-For）
    // 原实现优先 X-Forwarded-For（可被客户端伪造），且不 split/trim，与 audit_context 不一致
    // 优先复用 audit_context 中间件已提取的 IP（已统一优先级），回退到 headers-only 提取
    let client_ip = audit_ctx
        .as_ref()
        .map(|e| e.0.ip_address.clone())
        .filter(|s| !s.is_empty() && s != "unknown")
        .unwrap_or_else(|| {
            // P3 维度 12 修复（批次 87）：headers-only 降级路径
            // 此处仅有 HeaderMap 引用（无 Request<Body>），无法直接调用 audit_context::extract_client_ip
            // 优先级与 audit_context::extract_client_ip 一致：X-Real-IP → X-Forwarded-For → "unknown"
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .or_else(|| {
                    headers
                        .get("x-forwarded-for")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.split(',').next().map(|s| s.trim().to_string()))
                        .filter(|s| !s.is_empty())
                })
                .unwrap_or_else(|| "unknown".to_string())
        });
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // Check account lockout before authentication (per username+IP to prevent DoS)
    let since = Utc::now() - ChronoDuration::minutes(LOCKOUT_DURATION_MINUTES);
    use crate::models::log_login;
    use sea_orm::PaginatorTrait;

    // Per-IP lockout: prevents an attacker from locking out a legitimate user
    let recent_ip_failures = log_login::Entity::find()
        .filter(log_login::Column::Username.eq(payload.username.as_str()))
        .filter(log_login::Column::Status.eq("FAILED"))
        .filter(log_login::Column::LoginTime.gte(since))
        .filter(log_login::Column::IpAddress.eq(client_ip.as_str()))
        .count(state.db.as_ref())
        .await
        .unwrap_or_default();

    // Per-username global lockout with higher threshold (10 attempts from any IP)
    let recent_user_failures = log_login::Entity::find()
        .filter(log_login::Column::Username.eq(payload.username.as_str()))
        .filter(log_login::Column::Status.eq("FAILED"))
        .filter(log_login::Column::LoginTime.gte(since))
        .count(state.db.as_ref())
        .await
        .unwrap_or_default();

    if recent_ip_failures >= MAX_FAILED_ATTEMPTS as u64 {
        tracing::warn!(
            "Account locked due to too many failed attempts from IP: {} for user {}",
            client_ip,
            payload.username
        );
        return Err(AppError::too_many_requests(
            "登录失败次数过多，请30分钟后再试",
        ));
    }

    if recent_user_failures >= (MAX_FAILED_ATTEMPTS * 2) as u64 {
        tracing::warn!(
            "Account globally locked due to too many failed attempts: {}",
            payload.username
        );
        return Err(AppError::too_many_requests("账号已被锁定，请30分钟后再试"));
    }

    let auth_service = AuthService::new(state.db.clone(), state.jwt_secret.clone());

    match auth_service
        .authenticate(&payload.username, &payload.password)
        .await
    {
        Ok((token, user)) => {
            // 验证 TOTP 逻辑 (如已开启)
            if user.is_totp_enabled {
                let totp_service = TotpService::new(state.db.clone());

                // v11 批次 141：优先验证 totp_token，缺失时尝试 recovery_code
                if let Some(ref totp_token) = payload.totp_token {
                    // 路径 A：TOTP 令牌验证
                    match totp_service.verify_login_totp(user.id, totp_token).await {
                        Ok(true) => {} // 验证通过
                        _ => {
                            record_login_attempt(
                                &state,
                                &payload.username,
                                user.id,
                                &client_ip,
                                &user_agent,
                                "FAILED",
                                Some("TOTP verification failed"),
                            )
                            .await;
                            return Err(AppError::unauthorized("两步验证码错误"));
                        }
                    }
                } else if let Some(ref recovery_code) = payload.recovery_code {
                    // 路径 B：恢复码验证（v11 批次 141 新增）
                    match totp_service
                        .verify_recovery_code(user.id, recovery_code)
                        .await
                    {
                        Ok(true) => {
                            tracing::info!(
                                user_id = user.id,
                                username = %user.username,
                                "用户通过恢复码登录（恢复码已消耗）"
                            );
                        }
                        _ => {
                            record_login_attempt(
                                &state,
                                &payload.username,
                                user.id,
                                &client_ip,
                                &user_agent,
                                "FAILED",
                                Some("Recovery code verification failed"),
                            )
                            .await;
                            return Err(AppError::unauthorized("恢复码无效或已使用"));
                        }
                    }
                } else {
                    // 两种验证方式都未提供
                    record_login_attempt(
                        &state,
                        &payload.username,
                        user.id,
                        &client_ip,
                        &user_agent,
                        "FAILED",
                        Some("TOTP token and recovery code both missing"),
                    )
                    .await;
                    return Err(AppError::unauthorized("需要提供两步验证码或恢复码"));
                }
            }

            // Record successful login
            record_login_attempt(
                &state,
                &payload.username,
                user.id,
                &client_ip,
                &user_agent,
                "SUCCESS",
                None,
            )
            .await;

            // 记录增强登录安全日志
            let security_log = LoginSecurityLog {
                event: "LOGIN_SUCCESS".to_string(),
                attempt: LoginAttempt {
                    username: payload.username.clone(),
                    ip_address: client_ip.clone(),
                    user_agent: user_agent.clone(),
                    timestamp: Utc::now().to_rfc3339(),
                    method: "password".to_string(),
                    login_type: "web".to_string(),
                },
                failure_info: None,
                security_info: SecurityInfo {
                    risk_level: "LOW".to_string(),
                    risk_factors: Vec::new(),
                    blocked: false,
                    block_reason: None,
                    require_captcha: false,
                    notify_user: false,
                },
                geo_info: None,
                device_info: DeviceInfo {
                    os: None,
                    browser: None,
                    device_type: "unknown".to_string(),
                    is_mobile: false,
                },
            };
            enhanced_logger::EnhancedLogger::log_login_security(&security_log);

            // 异步记录审计日志：登录成功（P13 批 1 P3-2）
            let login_event = AuditEvent {
                user_id: Some(user.id),
                username: Some(payload.username.clone()),
                operation_type: OperationType::Login,
                severity: Severity::Info,
                resource_type: Some("auth".to_string()),
                resource_id: Some(user.id.to_string()),
                resource_name: Some(user.username.clone()),
                description: Some("用户登录成功".to_string()),
                request_method: Some("POST".to_string()),
                request_path: Some("/api/v1/erp/auth/login".to_string()),
                before_snapshot: None,
                after_snapshot: None,
            };
            let svc = Arc::new(AuditLogService::new(state.db.clone()));
            svc.record_async(login_event, audit_ctx.clone().map(|e| e.0));

            // Update last login timestamp
            // 批次 114 P1-6：登录后更新最后登录时间失败改为 warn 日志（原 `let _ =` 静默吞错）
            let user_svc = crate::services::user_service::UserService::new(state.db.clone());
            if let Err(e) = user_svc.update_last_login(user.id).await {
                tracing::warn!(error = %e, user_id = user.id, "更新最后登录时间失败（不影响登录主流程）");
            }

            // 批次 24 v6 P0-2 修复：使用统一的 build_with_permissions 构建 UserInfo，
            // 确保登录响应与 /auth/me 响应字段一致（均含 role_name 和 permissions）。
            // 安全漏洞 #14 修复：权限列表转换为 `Vec<String>` 资源标识符
            // 格式 `"{resource}:{action}"`，避免暴露内部 `resource_id` 主键
            let user_info = UserInfo::build_with_permissions(state.db.as_ref(), &user).await;
            let permissions = user_info.permissions.clone();

            // 生成 CSRF Token，使用 JWT claims 中的 session_id 作为会话标识
            let claims =
                AuthService::validate_token_static(&token, &state.jwt_secret).map_err(|e| {
                    tracing::error!("Failed to decode JWT token: {}", e);
                    AppError::unauthorized("无效的认证令牌")
                })?;
            let session_id = claims.session_id;

            // 提取客户端 IP（Wave 3 安全漏洞 #7：IP 绑定到 CSRF Token）
            // 优先从 AuditContext 取（已处理 X-Real-IP / X-Forwarded-For 多级降级），
            // 缺失时回退到 local 提取（双保险，与 audit_log 一致）。
            let csrf_ip = audit_ctx
                .as_ref()
                .map(|e| e.0.ip_address.clone())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| client_ip.clone());

            // 强制轮换：登录前清除该 user_id 关联的旧 CSRF Token（Wave 3 #7）
            let rotated = state.cache.clear_old_csrf_token_for_user(user.id);
            if rotated {
                tracing::info!(
                    user_id = user.id,
                    username = %payload.username,
                    "已清除该用户的旧 CSRF Token（强制轮换）"
                );
            }

            // 生成随机 CSRF Token 并存储到缓存中（Wave 3 #7）：
            // - 缓存值 = (session_id, ip_address) 元组，IP 用于消费时校验
            // - 反向索引 user_id → csrf_token 支持强制轮换
            // - TTL = CSRF_TOKEN_DEFAULT_TTL_SECS (1800s = 30min)，与 access_token Cookie 对齐
            let csrf_token = uuid::Uuid::new_v4().to_string();
            state.cache.set_csrf_token(
                csrf_token.clone(),
                session_id.clone(),
                csrf_ip,
                user.id,
                None, // 使用默认 TTL (1800s)
            );

            // 生成 refresh_token：JWT 形式（P1 7-1 修复）
            // 修复背景：原用 uuid::Uuid::new_v4().to_string() 纯 UUID，但 refresh_token 接口
            // 用 validate_token_static（JWT 验证）校验，纯 UUID 必然验证失败，
            // 导致 access_token 30 分钟过期后用户永远无法刷新。
            // 修复方案：refresh_token 改用 JWT 形式，session_id 与 access_token 共享，
            // 便于 refresh 时统一吊销旧会话；exp = refresh_exp = 7 天。
            // 复用函数开头已创建的 auth_service（authenticate 调用所用）
            let refresh_token = auth_service
                .generate_refresh_token(user.id, &user.username, user.role_id, &session_id)
                .map_err(|e| AppError::internal(format!("生成刷新令牌失败：{}", e)))?;

            // 安全漏洞 #10 + #13 修复：LoginResponse 不再返回 token / refresh_token
            // - access_token 已在 httpOnly Cookie 写入
            // - refresh_token 已在 httpOnly Cookie 写入
            // 仅保留 csrf_token（前端 form header 需要）+ user + permissions
            // 批次 198 P0-2：检查密码是否过期（password_changed_at 为 None 时不强制过期，兼容存量用户）
            let policy_svc =
                crate::services::auth::password_policy_service::PasswordPolicyService::new();
            let password_expired = user
                .password_changed_at
                .map(|t| policy_svc.is_expired(t))
                .unwrap_or(false);
            if password_expired {
                tracing::info!(
                    user_id = user.id,
                    username = %payload.username,
                    "[SECURITY] 用户密码已过期（超过 {} 天未修改），前端将引导改密",
                    policy_svc.max_age_days.unwrap_or(0)
                );
            }
            let response = LoginResponse {
                csrf_token: csrf_token.clone(),
                user: user_info,
                permissions,
                password_expired,
            };

            // 创建 HttpOnly Cookie
            // 开发环境下关闭 secure 标志，允许 HTTP 传输；生产环境必须开启 HTTPS
            // 漏洞 #12 修复：统一从 `crate::utils::config::is_production()` 读取 APP_ENV
            let is_production = crate::utils::config::is_production();

            // access_token: httpOnly（防 XSS 窃取），SameSite=Strict 防止跨站请求携带
            let access_cookie = Cookie::build(("access_token", token.clone()))
                .path("/")
                .http_only(true)
                .secure(is_production)
                .same_site(SameSite::Strict)
                .max_age(CookieDuration::minutes(30))
                .build();

            // refresh_token: httpOnly，2 天有效期（用于续签 access_token）
            // P2 7-9 修复：原 7 天有效期无 IP 绑定，被窃取后可任意 IP 使用 7 天。
            // 缩短至 2 天降低被盗用窗口；IP 绑定 + user_agent 绑定作为后续技术债
            // （需 AppClaims 增加字段 + refresh handler 验证，涉及 token 结构变更）。
            let refresh_cookie = Cookie::build(("refresh_token", refresh_token))
                .path("/")
                .http_only(true)
                .secure(is_production)
                .same_site(SameSite::Strict)
                .max_age(CookieDuration::days(2))
                .build();

            // csrf_token: 必须可被前端 JS 读取以注入 X-CSRF-Token 头，
            // 故 http_only=false；CSRF 防护依赖"攻击者无法读取跨域 Cookie"的同源策略
            let csrf_cookie = Cookie::build(("csrf_token", response.csrf_token.clone()))
                .path("/")
                .http_only(false)
                .secure(is_production)
                .same_site(SameSite::Strict)
                .max_age(CookieDuration::days(7))
                .build();

            // 兼容旧版客户端：保留 jwt Cookie（httpOnly）。新代码优先读取 access_token。
            // L-2 修复：legacy_jwt 也使用 SameSite::Strict，防止 CSRF 攻击
            let legacy_jwt_cookie = Cookie::build(("jwt", token.clone()))
                .path("/")
                .http_only(true)
                .secure(is_production)
                .same_site(SameSite::Strict)
                .max_age(CookieDuration::minutes(30))
                .build();

            let jar = jar
                .add(access_cookie)
                .add(refresh_cookie)
                .add(csrf_cookie)
                .add(legacy_jwt_cookie);

            Ok((jar, Json(ApiResponse::success(response))).into_response())
        }
        Err(e) => {
            // Record failed login attempt
            record_login_attempt(
                &state,
                &payload.username,
                0,
                &client_ip,
                &user_agent,
                "FAILED",
                Some(&e.to_string()),
            )
            .await;

            // 记录增强登录安全日志
            let security_log = LoginSecurityLog {
                event: "LOGIN_FAILURE".to_string(),
                attempt: LoginAttempt {
                    username: payload.username.clone(),
                    ip_address: client_ip.clone(),
                    user_agent: user_agent.clone(),
                    timestamp: Utc::now().to_rfc3339(),
                    method: "password".to_string(),
                    login_type: "web".to_string(),
                },
                failure_info: Some(FailureInfo {
                    reason: e.to_string(),
                    attempts_today: recent_user_failures as i32 + 1,
                    attempts_total: 0,
                    last_success: None,
                    last_failure: Some(Utc::now().to_rfc3339()),
                }),
                security_info: SecurityInfo {
                    risk_level: if recent_user_failures >= 3 {
                        "HIGH".to_string()
                    } else {
                        "MEDIUM".to_string()
                    },
                    risk_factors: {
                        let mut factors = Vec::new();
                        if recent_user_failures >= 3 {
                            factors.push("多次失败".to_string());
                        }
                        factors
                    },
                    blocked: recent_ip_failures >= MAX_FAILED_ATTEMPTS as u64,
                    block_reason: if recent_ip_failures >= MAX_FAILED_ATTEMPTS as u64 {
                        Some("登录失败次数过多".to_string())
                    } else {
                        None
                    },
                    require_captcha: recent_user_failures >= 2,
                    notify_user: false,
                },
                geo_info: None,
                device_info: DeviceInfo {
                    os: None,
                    browser: None,
                    device_type: "unknown".to_string(),
                    is_mobile: false,
                },
            };
            enhanced_logger::EnhancedLogger::log_login_security(&security_log);

            // 异步记录审计日志：登录失败（P13 批 1 P3-2）
            let failure_event = AuditEvent {
                user_id: None,
                username: Some(payload.username.clone()),
                operation_type: OperationType::Login,
                severity: Severity::Warn,
                resource_type: Some("auth".to_string()),
                resource_id: None,
                resource_name: None,
                description: Some(format!("用户登录失败：{}", e)),
                request_method: Some("POST".to_string()),
                request_path: Some("/api/v1/erp/auth/login".to_string()),
                before_snapshot: None,
                after_snapshot: None,
            };
            let svc = Arc::new(AuditLogService::new(state.db.clone()));
            svc.record_async(failure_event, audit_ctx.map(|e| e.0));

            Err(AppError::unauthorized(e.to_string()))
        }
    }
}

// =================================================================
// 安全漏洞 #10 + #13 + #14 修复的单测
// 验证 LoginResponse 序列化后：
//   - 不含 `token` 字段（access_token 已在 httpOnly Cookie 写入）
//   - 不含 `refresh_token` 字段（refresh_token 已在 httpOnly Cookie 写入）
//   - `permissions` 字段类型为 `Vec<String>` 资源标识符（`"{resource}:{action}"` 格式）
// =================================================================
#[cfg(test)]
mod tests {
    use super::*;

    /// 构造测试用的 LoginResponse 实例
    fn build_test_login_response() -> LoginResponse {
        LoginResponse {
            csrf_token: "csrf-token-uuid".to_string(),
            user: UserInfo {
                id: 42,
                username: "test_user".to_string(),
                email: Some("test@example.com".to_string()),
                role_id: Some(1),
                role_name: Some("admin".to_string()),
                permissions: vec![
                    "user.list:read".to_string(),
                    "user.list:write".to_string(),
                    "order:read".to_string(),
                ],
                // 批次 29 v7 P0-4+5：补全新增的 6 个字段（与生产构造保持一致）
                phone: Some("13800000000".to_string()),
                department_id: Some(1),
                department_name: Some("研发部".to_string()),
                is_totp_enabled: false,
                real_name: Some("测试用户".to_string()),
                avatar: None,
            },
            permissions: vec![
                "user.list:read".to_string(),
                "user.list:write".to_string(),
                "order:read".to_string(),
            ],
            password_expired: false,
        }
    }

    /// 测试 #10：LoginResponse JSON 序列化结果不含 `token` 字段
    /// 原因：access_token 已通过 httpOnly Cookie 写入响应，响应体再含 token 字段会增加
    ///       XSS/中间人/前端日志泄露的攻击面
    #[test]
    fn test_login_response_omits_token_field() {
        let response = build_test_login_response();
        let json = serde_json::to_value(&response).expect("LoginResponse 序列化失败");

        // 响应体不应包含 `token` 字段
        assert!(
            json.get("token").is_none(),
            "LoginResponse 序列化结果不应包含 `token` 字段，实际 JSON = {}",
            json
        );
    }

    /// 测试 #13：LoginResponse JSON 序列化结果不含 `refresh_token` 字段
    /// 原因：refresh_token 已通过 httpOnly Cookie 写入响应，响应体再含 refresh_token 字段
    ///       同样会增加泄露风险
    #[test]
    fn test_login_response_omits_refresh_token_field() {
        let response = build_test_login_response();
        let json = serde_json::to_value(&response).expect("LoginResponse 序列化失败");

        // 响应体不应包含 `refresh_token` 字段
        assert!(
            json.get("refresh_token").is_none(),
            "LoginResponse 序列化结果不应包含 `refresh_token` 字段，实际 JSON = {}",
            json
        );
    }

    /// 测试 #14：LoginResponse 的 `permissions` 字段是 `Vec<String>` 类型
    /// 验证资源标识符格式 `"{resource}:{action}"`，且不暴露内部 `resource_id` 主键
    #[test]
    fn test_login_response_permissions_is_string_array() {
        let response = build_test_login_response();
        let json = serde_json::to_value(&response).expect("LoginResponse 序列化失败");

        // 验证 permissions 字段存在
        let permissions = json
            .get("permissions")
            .expect("LoginResponse 应包含 `permissions` 字段")
            .as_array()
            .expect("`permissions` 字段类型应为 JSON 数组");

        // 验证数组元素全部为字符串（不是对象）
        assert_eq!(permissions.len(), 3, "测试数据应包含 3 个权限项");
        for (i, perm) in permissions.iter().enumerate() {
            assert!(
                perm.is_string(),
                "`permissions[{}]` 必须是字符串，实际类型 = {:?}",
                i,
                perm
            );
        }

        // 验证资源标识符格式 `"{resource}:{action}"`
        assert_eq!(permissions[0].as_str(), Some("user.list:read"));
        assert_eq!(permissions[1].as_str(), Some("user.list:write"));
        assert_eq!(permissions[2].as_str(), Some("order:read"));

        // 验证 permissions 元素是对象时不存在（防止回归到 `Vec<UserPermissionDto>` 形态）
        assert!(
            permissions[0].as_object().is_none(),
            "`permissions` 元素不应为对象，回归到 `Vec<UserPermissionDto>` 形态"
        );
    }

    /// 综合测试：LoginResponse 序列化结果的字段白名单
    /// 只允许包含 `csrf_token` / `user` / `permissions` 三个字段
    #[test]
    fn test_login_response_field_whitelist() {
        let response = build_test_login_response();
        let json = serde_json::to_value(&response).expect("LoginResponse 序列化失败");
        let obj = json.as_object().expect("LoginResponse 应序列化为 JSON 对象");

        let actual_fields: std::collections::HashSet<&String> = obj.keys().collect();
        let expected_fields: std::collections::HashSet<&str> =
            ["csrf_token", "user", "permissions"].into_iter().collect();

        let extra: Vec<&&String> = actual_fields
            .iter()
            .filter(|f| !expected_fields.contains(f.as_str()))
            .collect();
        assert!(
            extra.is_empty(),
            "LoginResponse 应仅包含白名单字段，发现额外字段: {:?}",
            extra
        );
    }
}
