//! 多租户行级数据隔离中间件
//!
//! 基于 SeaORM QueryFilter trait 实现自动租户过滤，
//! 在所有数据库查询中注入 tenant_id 条件，确保数据隔离。
#![allow(dead_code)]

use std::collections::HashSet;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use sea_orm::QueryFilter;
use tokio::sync::RwLock;

use crate::middleware::auth_context::AuthContext;
use crate::middleware::tenant::TenantContext;
use crate::utils::app_state::AppState;

/// 租户隔离配置
#[derive(Debug, Clone)]
pub struct TenantIsolationConfig {
    /// 不需要租户隔离的表名白名单
    pub whitelist: HashSet<String>,
    /// 是否启用严格模式（缺少租户ID时拒绝请求）
    pub strict_mode: bool,
}

impl Default for TenantIsolationConfig {
    fn default() -> Self {
        Self {
            whitelist: HashSet::from([
                "tenants".to_string(),
                "tenant_plans".to_string(),
                "tenant_configs".to_string(),
                "tenant_users".to_string(),
                "users".to_string(),
                "roles".to_string(),
                "role_permissions".to_string(),
                "data_permissions".to_string(),
                "migrations".to_string(),
                "log_api_access".to_string(),
                "log_login".to_string(),
                "log_system".to_string(),
                "audit_log".to_string(),
                "omni_audit_log".to_string(),
                "audit_alert_rule".to_string(),
            ]),
            strict_mode: true,
        }
    }
}

/// 租户隔离运行时状态
#[derive(Debug, Clone)]
pub struct TenantIsolationState {
    pub config: TenantIsolationConfig,
    /// 管理员角色 ID 列表（可跨租户查询）
    pub admin_role_ids: Arc<RwLock<Vec<i32>>>,
}

impl TenantIsolationState {
    pub fn new(config: TenantIsolationConfig) -> Self {
        Self {
            config,
            admin_role_ids: Arc::new(RwLock::new(vec![])),
        }
    }

    /// 检查表是否需要租户隔离
    pub fn requires_isolation(&self, table_name: &str) -> bool {
        !self.config.whitelist.contains(table_name)
    }

    /// 检查用户是否具有管理员权限（可跨租户查询）
    pub async fn is_admin(&self, auth_context: &AuthContext) -> bool {
        if let Some(role_id) = auth_context.role_id {
            let admin_roles = self.admin_role_ids.read().await;
            admin_roles.contains(&role_id)
        } else {
            false
        }
    }

    /// 添加管理员角色 ID
    pub async fn add_admin_role(&self, role_id: i32) {
        let mut admin_roles = self.admin_role_ids.write().await;
        if !admin_roles.contains(&role_id) {
            admin_roles.push(role_id);
        }
    }

    /// 移除管理员角色 ID
    pub async fn remove_admin_role(&self, role_id: i32) {
        let mut admin_roles = self.admin_role_ids.write().await;
        admin_roles.retain(|&id| id != role_id);
    }
}

impl AppState {
    /// 获取租户隔离状态（如果存在）
    pub fn tenant_isolation_state(&self) -> Option<Arc<TenantIsolationState>> {
        self.get_service::<TenantIsolationState>()
    }
}

/// 从请求中提取租户 ID
/// 优先级：TenantContext > AuthContext.tenant_id > Header X-Tenant-ID
fn extract_tenant_id(request: &Request<Body>) -> Option<i32> {
    // 1. 从 TenantContext 提取
    if let Some(tenant_ctx) = request.extensions().get::<TenantContext>() {
        return Some(tenant_ctx.tenant_id);
    }

    // 2. 从 AuthContext 提取
    if let Some(auth_ctx) = request.extensions().get::<AuthContext>() {
        if let Some(tid) = auth_ctx.tenant_id {
            return Some(tid);
        }
    }

    // 3. 从 Header 提取
    request
        .headers()
        .get("X-Tenant-ID")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<i32>().ok())
}

/// 租户隔离中间件
///
/// 验证租户有效性，并在请求扩展中注入隔离标记，
/// 供后续 Service 层使用 SeaORM QueryFilter 应用过滤。
pub async fn tenant_isolation_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path().to_string();

    // 公共路由跳过隔离检查
    if is_public_isolation_path(&path) {
        return Ok(next.run(request).await);
    }

    // 获取隔离配置
    let isolation_state = match state.tenant_isolation_state() {
        Some(s) => s,
        None => {
            // 未配置隔离状态，使用默认配置继续
            let default_state = Arc::new(TenantIsolationState::new(TenantIsolationConfig::default()));
            default_state
        }
    };

    // 提取租户 ID
    let tenant_id = extract_tenant_id(&request);

    // 严格模式下，缺少租户ID则拒绝
    if isolation_state.config.strict_mode && tenant_id.is_none() {
        // 检查是否是管理员（管理员可以没有租户ID）
        if let Some(auth_ctx) = request.extensions().get::<AuthContext>() {
            if !isolation_state.is_admin(auth_ctx).await {
                tracing::warn!(
                    path = %path,
                    "租户隔离: 缺少租户ID且非管理员"
                );
                return Err(StatusCode::BAD_REQUEST);
            }
        } else {
            tracing::warn!(
                path = %path,
                "租户隔离: 缺少租户ID且未认证"
            );
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    // 注入租户隔离标记到请求扩展
    if let Some(tid) = tenant_id {
        request.extensions_mut().insert(TenantIsolationMarker {
            tenant_id: tid,
            skip_isolation: false,
        });
    } else if let Some(auth_ctx) = request.extensions().get::<AuthContext>() {
        // 管理员跨租户查询
        if isolation_state.is_admin(auth_ctx).await {
            request.extensions_mut().insert(TenantIsolationMarker {
                tenant_id: 0,
                skip_isolation: true,
            });
        }
    }

    // 注入隔离状态供后续使用
    request.extensions_mut().insert(isolation_state);

    Ok(next.run(request).await)
}

/// 租户隔离标记，注入到请求扩展中
#[derive(Debug, Clone)]
pub struct TenantIsolationMarker {
    pub tenant_id: i32,
    pub skip_isolation: bool,
}

impl TenantIsolationMarker {
    /// 从请求扩展中获取标记
    pub fn from_request(request: &Request<Body>) -> Option<&Self> {
        request.extensions().get::<Self>()
    }

    /// 检查是否应该跳过隔离
    pub fn should_skip(&self) -> bool {
        self.skip_isolation
    }

    /// 获取租户 ID
    pub fn tenant_id(&self) -> i32 {
        self.tenant_id
    }
}

/// 检查路径是否跳过租户隔离
fn is_public_isolation_path(path: &str) -> bool {
    let public_prefixes = [
        "/api/health",
        "/api/swagger",
        "/api/openapi",
        "/api/auth/login",
        "/api/auth/register",
        "/api/public",
        "/grpc",
    ];

    public_prefixes.iter().any(|prefix| path.starts_with(prefix))
}

/// SeaORM QueryFilter 辅助 trait
/// 为 Select 语句添加租户过滤
pub trait TenantScopedQuery {
    fn apply_tenant_filter<C>(self, tenant_id: i32, table_name: &str, state: &TenantIsolationState) -> Self
    where
        C: sea_orm::ColumnTrait,
        Self: QueryFilter + Sized;
}

impl<S> TenantScopedQuery for sea_orm::Select<S>
where
    S: sea_orm::EntityTrait,
{
    fn apply_tenant_filter<C>(self, tenant_id: i32, table_name: &str, state: &TenantIsolationState) -> Self
    where
        C: sea_orm::ColumnTrait,
        Self: QueryFilter + Sized,
    {
        if !state.requires_isolation(table_name) {
            return self;
        }

        self.filter(sea_orm::Condition::all().add(
            sea_orm::Condition::any().add(
                C::belongs_to(tenant_id)
            )
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_whitelist_contains_system_tables() {
        let config = TenantIsolationConfig::default();
        assert!(config.whitelist.contains("tenants"));
        assert!(config.whitelist.contains("users"));
        assert!(config.whitelist.contains("roles"));
    }

    #[test]
    fn test_requires_isolation() {
        let state = TenantIsolationState::new(TenantIsolationConfig::default());
        assert!(!state.requires_isolation("tenants"));
        assert!(state.requires_isolation("products"));
        assert!(state.requires_isolation("orders"));
    }

    #[test]
    fn test_tenant_isolation_marker() {
        let marker = TenantIsolationMarker {
            tenant_id: 42,
            skip_isolation: false,
        };
        assert_eq!(marker.tenant_id(), 42);
        assert!(!marker.should_skip());

        let admin_marker = TenantIsolationMarker {
            tenant_id: 0,
            skip_isolation: true,
        };
        assert!(admin_marker.should_skip());
    }
}
