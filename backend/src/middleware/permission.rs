use crate::middleware::auth_context::AuthContext;
use crate::middleware::public_routes::is_public_path;
use crate::models::audit_log::{OperationType, Severity};
use crate::models::role_permission;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::utils::admin_checker;
use crate::utils::app_state::AppState;
use crate::utils::path_utils::{
    is_known_resource_segment, is_module_prefix, resolve_module_prefixed_resource,
};
use crate::utils::request_ext::PublicPathCache;
use crate::utils::response::{forbidden_response, unauthorized_response};
use axum::{
    body::Body,
    extract::State,
    http::{Method, Request},
    middleware::Next,
    response::Response,
};
use chrono::{DateTime, Duration, Utc};
use futures::StreamExt;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::sync::Arc;
use tracing::warn;

/// 检查请求是否命中公共路径（优先使用缓存）
fn is_public_path_request(request: &Request<Body>, path: &str) -> bool {
    request
        .extensions()
        .get::<PublicPathCache>()
        .map(|cache| cache.is_public)
        .unwrap_or_else(|| is_public_path(path))
}

/// 提取认证上下文，缺失时返回 401
#[allow(clippy::result_large_err)]
fn extract_auth_context(request: &Request<Body>) -> Result<AuthContext, Response> {
    match request.extensions().get::<AuthContext>().cloned() {
        Some(auth) => Ok(auth),
        None => {
            warn!("缺少认证上下文");
            Err(unauthorized_response("缺少认证上下文"))
        }
    }
}

/// 从认证上下文提取 role_id，缺失时返回 403
#[allow(clippy::result_large_err)]
fn extract_role_id(auth: &AuthContext) -> Result<i32, Response> {
    match auth.role_id {
        Some(id) => Ok(id),
        None => {
            warn!("用户没有关联角色，拒绝访问");
            Err(forbidden_response("没有关联角色，无法访问"))
        }
    }
}

/// V15 P0-S21：校验 segment3 是否在已知资源白名单中
#[allow(clippy::result_large_err)]
fn validate_route_whitelist(path: &str) -> Result<(), Response> {
    if let Some(segment3) = extract_segment3(path) {
        if !is_known_resource_segment(segment3) {
            warn!(
                "拒绝未知路由: path={}, segment3={} 不在白名单中",
                path, segment3
            );
            return Err(forbidden_response("未知的资源路径"));
        }
    }
    Ok(())
}

/// 综合提取路由信息：resource_type + resource_id + action（query > path > method）
fn extract_route_info(
    path: &str,
    uri: &axum::http::Uri,
    method: &Method,
) -> (String, Option<i32>, String) {
    let (resource_type, resource_id) = extract_resource_info(path);
    let action = extract_action_from_query(uri)
        .or_else(|| extract_action_from_path(path))
        .unwrap_or_else(|| method_to_action(method));
    (resource_type, resource_id, action)
}

/// V15 P1-5-3：认证豁免 RBAC 的路径清单（仅跳过权限码校验，仍需 JWT 认证）
// 端点：/audit-logs/record-print（前端打印审计埋点，任何已认证用户均可上报，无副作用）
const AUTH_ONLY_PATHS: &[&str] = &["/api/v1/erp/audit-logs/record-print"];

/// V15 P1-5-3：检查路径是否仅需认证（跳过 RBAC 权限码校验）
fn is_auth_only_path(path: &str) -> bool {
    let clean_path = path.split(['?', '#']).next().unwrap_or(path);
    AUTH_ONLY_PATHS.contains(&clean_path)
}

/// 权限校验中间件：公共路径放行，非公共路径校验 role_id + 资源/动作权限
pub async fn permission_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let method = request.method();
    let uri = request.uri();
    let path = uri.path();

    if is_public_path_request(&request, path) {
        return Ok(next.run(request).await);
    }

    let auth = extract_auth_context(&request)?;
    tracing::debug!("权限检查: user_id={}, path={}", auth.user_id, path);

    let role_id = extract_role_id(&auth)?;

    // V15 P1-5-3：认证豁免 RBAC 路径（如前端打印审计埋点），仅需认证 + role_id
    if is_auth_only_path(path) {
        tracing::debug!(
            "认证豁免 RBAC 路径放行: path={}, user_id={}",
            path,
            auth.user_id
        );
        return Ok(next.run(request).await);
    }

    validate_route_whitelist(path)?;
    let (resource_type, resource_id, action) = extract_route_info(path, uri, method);

    let has_permission =
        check_permission(&state.db, role_id, &resource_type, resource_id, &action).await;
    tracing::debug!(
        "权限检查结果: path={}, resource={}, action={}, has_perm={}",
        path,
        resource_type,
        action,
        has_permission
    );

    if has_permission {
        Ok(next.run(request).await)
    } else {
        warn!("权限不足: path={} {}", method, path);
        // V15 P1 12.5：权限拒绝日志落库（resource_type=permission_denied）
        // 安全原因：权限拒绝是安全事件，必须落库审计以便追溯越权尝试。
        record_permission_denial(
            &state.audit_log,
            &auth,
            method,
            path,
            &resource_type,
            resource_id,
            &action,
        );
        Err(forbidden_response("权限不足，无法访问该资源"))
    }
}

/// V15 P1 12.5：异步落库权限拒绝审计事件（best-effort，不阻塞业务响应）。
/// 字段说明：user_id/path/method/ip/user_agent/required_permission（resource_type:action）
fn record_permission_denial(
    audit_log: &Arc<AuditLogService>,
    auth: &AuthContext,
    method: &Method,
    path: &str,
    resource_type: &str,
    resource_id: Option<i32>,
    action: &str,
) {
    let required_permission = if let Some(rid) = resource_id {
        format!("{}:{}:{}", resource_type, action, rid)
    } else {
        format!("{}:{}", resource_type, action)
    };
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Other,
        severity: Severity::Warn,
        resource_type: Some("permission_denied".to_string()),
        resource_id: None,
        resource_name: Some(format!("权限拒绝: {}", required_permission)),
        description: Some(format!(
            "用户 {}（user_id={}，role_id={:?}）尝试访问资源 {} 但权限不足",
            auth.username, auth.user_id, auth.role_id, required_permission
        )),
        request_method: Some(method.as_str().to_string()),
        request_path: Some(path.to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "required_permission": required_permission,
            "resource_type": resource_type,
            "resource_id": resource_id,
            "action": action,
        })),
    };
    audit_log.clone().record_async(event, None);
}

/// V15 P0-S21：提取 URL segment3（/api/v1/erp/{segment3}/...），用于白名单校验
fn extract_segment3(path: &str) -> Option<&str> {
    let path_parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    if path_parts.len() >= 4
        && path_parts[0] == "api"
        && path_parts[1] == "v1"
        && path_parts[2] == "erp"
    {
        Some(path_parts[3])
    } else {
        None
    }
}

/// V15 P0-S20：路径动作关键字集合（出现在 URL 末段时优先作为 action）
const PATH_ACTION_KEYWORDS: &[&str] = &[
    "print", "export", "import", "audit", "approve", "reject", "cancel", "close", "confirm",
    "submit", "release",
];

/// V15 P0-S20：从路径末段提取动作关键字，非关键字返回 None
fn extract_action_from_path(path: &str) -> Option<String> {
    // V15 clippy 修复：使用 rfind 从后向前查找第一个非空段，等价于 filter().next_back() 但更简洁
    let last_segment = path.split('/').rfind(|p| !p.is_empty())?;
    if PATH_ACTION_KEYWORDS.contains(&last_segment) {
        Some(last_segment.to_string())
    } else {
        None
    }
}

/// V15 P0-S10：查询参数 action 关键字白名单（print/export/download）
const QUERY_ACTION_KEYWORDS: &[&str] = &["print", "export", "download"];

/// V15 P0-S10：从 `?action=xxx` 提取动作，仅识别白名单内动作以防绕过权限
fn extract_action_from_query(uri: &axum::http::Uri) -> Option<String> {
    let query = uri.query()?;
    // 解析 query string，查找 action 参数
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        if parts.next()? == "action" {
            let value = parts.next()?;
            // URL 解码（处理 %20 等编码）
            let decoded = percent_encoding::percent_decode_str(value)
                .decode_utf8()
                .ok()?;
            if QUERY_ACTION_KEYWORDS.contains(&&*decoded) {
                return Some(decoded.into_owned());
            }
            // 不在白名单中，返回 None 让调用方回退
            return None;
        }
    }
    None
}

fn extract_resource_info(path: &str) -> (String, Option<i32>) {
    // 解析API路径，提取资源类型和ID
    let path_parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();

    if path_parts.len() >= 4
        && path_parts[0] == "api"
        && path_parts[1] == "v1"
        && path_parts[2] == "erp"
    {
        // 处理嵌套路径，如 /api/v1/erp/sales/orders/:id/approve
        // 资源类型由第4段决定，如果第4段是资源类型（如users, products），直接使用
        // 如果第4段是模块名（如sales, purchase），则使用第5段作为资源类型
        // V15 P1 4.2：处理双层嵌套模块前缀（如 /advanced/ai/recipe-optimization）
        // 当 segment3 与 segment4 均为模块前缀时，使用 segment5 作为资源类型
        // V15 P1-14.4-C：对模块前缀下的资源进行消歧（如 purchase/orders → purchase-orders）
        let resource_type = if path_parts.len() >= 5 && is_module_prefix(path_parts[3]) {
            if is_module_prefix(path_parts[4]) && path_parts.len() >= 6 {
                resolve_module_prefixed_resource(path_parts[3], path_parts[5])
            } else {
                resolve_module_prefixed_resource(path_parts[3], path_parts[4])
            }
        } else {
            path_parts[3].to_string()
        };

        // 尝试提取资源ID（跳过模块前缀）
        // V15 P0-S20 修复：跳过路径中的动作段（如 approve/export/print），
        // 避免动作关键字被误认为资源ID
        let start_idx = if path_parts.len() >= 5 && is_module_prefix(path_parts[3]) {
            if is_module_prefix(path_parts[4]) && path_parts.len() >= 6 {
                6
            } else {
                5
            }
        } else {
            4
        };
        for part in path_parts.iter().skip(start_idx) {
            // 跳过动作关键字，避免误判
            if PATH_ACTION_KEYWORDS.contains(part) {
                continue;
            }
            if let Ok(id) = part.parse::<i32>() {
                return (resource_type, Some(id));
            }
        }

        (resource_type, None)
    } else {
        ("unknown".to_string(), None)
    }
}

fn method_to_action(method: &Method) -> String {
    match *method {
        Method::GET => "read",
        Method::POST => "create",
        Method::PUT => "update",
        Method::PATCH => "update",
        Method::DELETE => "delete",
        _ => "read",
    }
    .to_string()
}

use dashmap::DashMap;
use std::sync::LazyLock;

/// 缓存项，包含数据和过期时间
#[derive(Clone)]
struct CacheEntry<T: Clone> {
    data: T,
    expires_at: DateTime<Utc>,
}

impl<T: Clone> CacheEntry<T> {
    fn new(data: T, ttl: Duration) -> Self {
        Self {
            data,
            expires_at: Utc::now() + ttl,
        }
    }

    fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

// Cache: role_id -> CacheEntry<Arc<Vec<role_permission::Model>>>
// 使用 Arc 包装，克隆时只增加引用计数，不复制数据
static PERMISSION_CACHE: LazyLock<DashMap<i32, CacheEntry<Arc<Vec<role_permission::Model>>>>> =
    LazyLock::new(DashMap::new);

/// 权限缓存TTL（5分钟）
const PERMISSION_CACHE_TTL: i64 = 5;

/// V15 P0-S07：失效指定角色的权限缓存（P1-14.9-C 同步发布 Redis pub/sub，多实例失效）
pub fn invalidate_permission_cache(role_id: i32) {
    PERMISSION_CACHE.remove(&role_id);
    tracing::info!(role_id, "权限缓存已失效");
    // V15 P1-14.9-C：发布 Redis pub/sub 通知（异步，不阻塞调用方）
    let channel = PERMISSION_CACHE_INVALIDATION_CHANNEL;
    let message = format!("{}", role_id);
    tokio::spawn(async move {
        crate::utils::redis_cache::publish_to_channel(channel, &message).await;
    });
}

/// V15 P0-S07：失效全部权限缓存（P1-14.9-C 同步发布 Redis pub/sub "ALL"，多实例清空）
pub fn invalidate_all_permission_cache() {
    PERMISSION_CACHE.clear();
    tracing::info!("全部权限缓存已失效");
    let channel = PERMISSION_CACHE_INVALIDATION_CHANNEL;
    tokio::spawn(async move {
        crate::utils::redis_cache::publish_to_channel(channel, "ALL").await;
    });
}

/// V15 P1-14.9-C：权限缓存失效 Redis pub/sub 频道名
const PERMISSION_CACHE_INVALIDATION_CHANNEL: &str = "permission_cache_invalidation";

/// V15 P1-14.9-C：启动权限缓存 Redis pub/sub 订阅器（应用启动时调用）
// 行为：订阅频道，"ALL"→清空本地缓存，"<role_id>"→失效指定角色缓存；
// 无 Redis 时 no-op；Redis 连接失败仅 warn 不阻塞启动
pub async fn start_permission_cache_pubsub_subscriber() {
    let Some(url) = crate::utils::redis_cache::get_redis_url() else {
        tracing::info!("未配置 Redis，权限缓存多实例广播降级为单实例本地失效");
        return;
    };
    let client = match redis::Client::open(url.as_str()) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "Redis Client 创建失败，权限缓存 Pub/Sub 订阅器未启动");
            return;
        }
    };
    // Pub/Sub 需要独立连接（不能复用 ConnectionManager，订阅会阻塞连接）
    let mut pubsub = match client.get_async_pubsub().await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "Redis PubSub 连接失败，权限缓存多实例广播未启用");
            return;
        }
    };
    if let Err(e) = pubsub
        .subscribe(PERMISSION_CACHE_INVALIDATION_CHANNEL)
        .await
    {
        tracing::warn!(error = %e, "Redis PubSub 订阅失败，权限缓存多实例广播未启用");
        return;
    }
    tracing::info!(
        "权限缓存 Redis Pub/Sub 订阅器已启动（频道: {}）",
        PERMISSION_CACHE_INVALIDATION_CHANNEL
    );
    let mut stream = pubsub.on_message();
    while let Some(msg) = stream.next().await {
        let payload = match msg.get_payload::<String>() {
            Ok(s) => s,
            Err(_) => {
                tracing::warn!("权限缓存 Pub/Sub 消息 payload 非 String，跳过");
                continue;
            }
        };
        if payload == "ALL" {
            PERMISSION_CACHE.clear();
            tracing::info!("接收到权限缓存失效通知：清空全部本地权限缓存");
        } else if let Ok(role_id) = payload.parse::<i32>() {
            PERMISSION_CACHE.remove(&role_id);
            tracing::info!(role_id, "接收到权限缓存失效通知：失效本地权限缓存");
        } else {
            tracing::warn!(payload = %payload, "权限缓存 Pub/Sub 消息格式无效（期望 ALL 或 role_id），跳过");
        }
    }
    tracing::warn!("权限缓存 Redis Pub/Sub 订阅器流结束（Redis 连接断开或服务器关闭）");
}

/// V15 P1-2-4：禁止打印操作的角色清单（customer/temporary 即使持有 print 权限码也拒绝）
const PRINT_DENIED_ROLE_CODES: &[&str] = &["customer", "temporary"];

/// V15 P1-2-4：禁止导出操作的角色清单（customer 外部用户/temporary 临时账号）
const EXPORT_DENIED_ROLE_CODES: &[&str] = &["customer", "temporary"];

/// V15 P1-2-4：染色配方导出额外禁止的角色清单（仅 dye_recipe_master 可导出）
const DYE_RECIPE_EXPORT_DENIED_ROLE_CODES: &[&str] = &[
    "customer",
    "temporary",
    "manager",
    "operator",
    "sales",
    "purchase",
    "warehouse",
];

/// V15 P1-2-4：检查角色是否被禁止执行指定动作（print/export/dye_recipe export 三类规则）
// 规则：action="print"→PRINT_DENIED；action="export"→EXPORT_DENIED；dye_recipe+export→DYE_RECIPE_EXPORT_DENIED
async fn is_action_denied_for_role(
    db: &sea_orm::DatabaseConnection,
    role_id: i32,
    resource_type: &str,
    action: &str,
) -> bool {
    // admin 短路（admin 不在禁止清单内，避免重复 DB 查询）
    if admin_checker::is_admin_role(db, role_id).await {
        return false;
    }

    let role_code = match admin_checker::get_role_code(db, role_id).await {
        Some(code) => code,
        None => {
            // 查询失败 fail-closed：禁止（避免查询失败时放行敏感操作）
            tracing::warn!(
                role_id,
                action,
                resource_type,
                "[P1-2-4] 角色 code 查询失败，fail-closed 拒绝 {} 操作",
                action
            );
            return true;
        }
    };

    // 染色配方导出特殊清单（更严格）
    if resource_type == "dye_recipe" && action == "export" {
        return DYE_RECIPE_EXPORT_DENIED_ROLE_CODES.contains(&role_code.as_str());
    }

    if action == "print" {
        return PRINT_DENIED_ROLE_CODES.contains(&role_code.as_str());
    }

    if action == "export" {
        return EXPORT_DENIED_ROLE_CODES.contains(&role_code.as_str());
    }

    false
}

async fn check_permission(
    db: &sea_orm::DatabaseConnection,
    role_id: i32,
    resource_type: &str,
    resource_id: Option<i32>,
    action: &str,
) -> bool {
    // 检查是否是管理员角色（带缓存）
    if admin_checker::is_admin_role(db, role_id).await {
        return true;
    }

    // V15 P1-2-4：print/export 动作的角色黑名单校验（在权限码校验之前）
    // 安全原因：customer/temporary 等外部角色即使误配 print/export 权限码也必须拒绝。
    // fail-closed：查询失败时拒绝（避免放行敏感操作）。
    if is_action_denied_for_role(db, role_id, resource_type, action).await {
        tracing::warn!(
            role_id,
            action,
            resource_type,
            "[P1-2-4] 角色 {} 被禁止执行 {} 操作（黑名单命中）",
            role_id,
            action
        );
        return false;
    }

    // 尝试从缓存加载，检查是否过期
    let permissions = if let Some(cached) = PERMISSION_CACHE.get(&role_id) {
        if cached.is_expired() {
            // 缓存已过期，移除并重新加载
            PERMISSION_CACHE.remove(&role_id);
            None
        } else {
            Some(cached.data.clone())
        }
    } else {
        None
    };

    let permissions = match permissions {
        Some(perms) => perms,
        None => {
            // 从数据库加载
            let db_perms = role_permission::Entity::find()
                .filter(
                    <role_permission::Entity as sea_orm::EntityTrait>::Column::RoleId.eq(role_id),
                )
                .filter(<role_permission::Entity as sea_orm::EntityTrait>::Column::Allowed.eq(true))
                .all(db)
                .await
                .unwrap_or_default();

            // 使用 Arc 包装，插入缓存，设置TTL
            let arc_perms = Arc::new(db_perms);
            let ttl = Duration::minutes(PERMISSION_CACHE_TTL);
            PERMISSION_CACHE.insert(role_id, CacheEntry::new(arc_perms.clone(), ttl));
            arc_perms
        }
    };

    // 检查是否有匹配的权限
    // M-6 修复：resource_id 精确匹配，action 支持 "*" 通配符
    permissions
        .iter()
        .any(|p| matches_permission(p, resource_type, resource_id, action))
}

/// 权限匹配纯函数：resource_type 精确匹配，action 支持 "*"，resource_id 精确匹配防越权
fn matches_permission(
    p: &role_permission::Model,
    resource_type: &str,
    resource_id: Option<i32>,
    action: &str,
) -> bool {
    p.resource_type == resource_type
        && (p.action == action || p.action == "*")
        && match (p.resource_id, resource_id) {
            (None, None) => true,
            (Some(pid), Some(rid)) => pid == rid,
            _ => false,
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造测试用权限模型
    fn make_permission(
        resource_type: &str,
        resource_id: Option<i32>,
        action: &str,
    ) -> role_permission::Model {
        role_permission::Model {
            id: 1,
            role_id: 1,
            resource_type: resource_type.to_string(),
            resource_id,
            action: action.to_string(),
            allowed: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // ===== extract_resource_info 测试 =====

    #[test]
    fn test_extract_resource_info_标准路径无ID() {
        let (rt, rid) = extract_resource_info("/api/v1/erp/users");
        assert_eq!(rt, "users");
        assert_eq!(rid, None);
    }

    #[test]
    fn test_extract_resource_info_标准路径带ID() {
        let (rt, rid) = extract_resource_info("/api/v1/erp/users/123");
        assert_eq!(rt, "users");
        assert_eq!(rid, Some(123));
    }

    #[test]
    fn test_extract_resource_info_模块前缀路径无ID() {
        let (rt, rid) = extract_resource_info("/api/v1/erp/sales/orders");
        assert_eq!(rt, "orders");
        assert_eq!(rid, None);
    }

    #[test]
    fn test_extract_resource_info_模块前缀路径带ID() {
        let (rt, rid) = extract_resource_info("/api/v1/erp/sales/orders/456");
        assert_eq!(rt, "orders");
        assert_eq!(rid, Some(456));
    }

    #[test]
    fn test_extract_resource_info_嵌套路径带ID和动作() {
        let (rt, rid) = extract_resource_info("/api/v1/erp/sales/orders/123/approve");
        assert_eq!(rt, "orders");
        assert_eq!(rid, Some(123));
    }

    #[test]
    fn test_extract_resource_info_非API路径() {
        let (rt, rid) = extract_resource_info("/health");
        assert_eq!(rt, "unknown");
        assert_eq!(rid, None);
    }

    #[test]
    fn test_extract_resource_info_短路径() {
        let (rt, rid) = extract_resource_info("/api/v1");
        assert_eq!(rt, "unknown");
        assert_eq!(rid, None);
    }

    #[test]
    fn test_extract_resource_info_空路径() {
        let (rt, rid) = extract_resource_info("/");
        assert_eq!(rt, "unknown");
        assert_eq!(rid, None);
    }

    #[test]
    fn test_extract_resource_info_动作段不误判为ID() {
        // V15 P0-S20 新增：动作关键字不应被误认为资源ID
        let (rt, rid) = extract_resource_info("/api/v1/erp/sales/orders/approve");
        assert_eq!(rt, "orders");
        assert_eq!(rid, None);
    }

    #[test]
    fn test_extract_resource_info_生产域模块前缀() {
        // V15 P0-S21 新增：production 模块前缀应正确提取资源
        let (rt, rid) = extract_resource_info("/api/v1/erp/production/dye-batches/789");
        assert_eq!(rt, "dye-batches");
        assert_eq!(rid, Some(789));
    }

    #[test]
    fn test_extract_resource_info_采购域修正拼写() {
        // V15 P0-S21 修正：purchase（单数）应正确识别为模块前缀
        // V15 P1-14.4-C：purchase/orders 消歧为 purchase-orders（与权限定义对齐）
        let (rt, rid) = extract_resource_info("/api/v1/erp/purchase/orders");
        assert_eq!(rt, "purchase-orders");
        assert_eq!(rid, None);
    }

    #[test]
    fn test_extract_resource_info_采购域消歧全量() {
        // V15 P1-14.4-C：采购域资源消歧映射
        let (rt, _) = extract_resource_info("/api/v1/erp/purchase/orders/123/approve");
        assert_eq!(rt, "purchase-orders");
        let (rt, _) = extract_resource_info("/api/v1/erp/purchase/returns/1");
        assert_eq!(rt, "purchase-returns");
        let (rt, _) = extract_resource_info("/api/v1/erp/purchase/receipts/1");
        assert_eq!(rt, "purchase-receipts");
        let (rt, _) = extract_resource_info("/api/v1/erp/purchase/contracts/1");
        assert_eq!(rt, "purchase-contracts");
        let (rt, _) = extract_resource_info("/api/v1/erp/purchase/prices/1");
        assert_eq!(rt, "purchase-prices");
    }

    #[test]
    fn test_extract_resource_info_销售域消歧全量() {
        // V15 P1-14.4-C：销售域资源消歧映射（orders 保留原名）
        let (rt, _) = extract_resource_info("/api/v1/erp/sales/orders/123/approve");
        assert_eq!(rt, "orders");
        let (rt, _) = extract_resource_info("/api/v1/erp/sales/returns/1");
        assert_eq!(rt, "sales-returns");
        let (rt, _) = extract_resource_info("/api/v1/erp/sales/contracts/1");
        assert_eq!(rt, "sales-contracts");
        let (rt, _) = extract_resource_info("/api/v1/erp/sales/prices/1");
        assert_eq!(rt, "sales-prices");
    }

    // ===== extract_segment3 测试 =====

    #[test]
    fn test_extract_segment3_标准路径() {
        assert_eq!(extract_segment3("/api/v1/erp/users"), Some("users"));
        assert_eq!(extract_segment3("/api/v1/erp/sales/orders"), Some("sales"));
        assert_eq!(
            extract_segment3("/api/v1/erp/production/dye-batches"),
            Some("production")
        );
    }

    #[test]
    fn test_extract_segment3_非API路径返回None() {
        assert_eq!(extract_segment3("/health"), None);
        assert_eq!(extract_segment3("/api/v1"), None);
        assert_eq!(extract_segment3("/"), None);
    }

    // ===== extract_action_from_path 测试 =====

    #[test]
    fn test_extract_action_from_path_approve动作() {
        assert_eq!(
            extract_action_from_path("/api/v1/erp/sales/orders/123/approve"),
            Some("approve".to_string())
        );
    }

    #[test]
    fn test_extract_action_from_path_export动作() {
        assert_eq!(
            extract_action_from_path("/api/v1/erp/users/export"),
            Some("export".to_string())
        );
    }

    #[test]
    fn test_extract_action_from_path_print动作() {
        assert_eq!(
            extract_action_from_path("/api/v1/erp/orders/456/print"),
            Some("print".to_string())
        );
    }

    #[test]
    fn test_extract_action_from_path_reject动作() {
        assert_eq!(
            extract_action_from_path("/api/v1/erp/purchase/orders/789/reject"),
            Some("reject".to_string())
        );
    }

    #[test]
    fn test_extract_action_from_path_无动作返回None() {
        assert_eq!(extract_action_from_path("/api/v1/erp/users"), None);
        assert_eq!(extract_action_from_path("/api/v1/erp/users/123"), None);
    }

    #[test]
    fn test_extract_action_from_path_非动作关键字返回None() {
        // 非动作关键字不应被识别为动作
        assert_eq!(extract_action_from_path("/api/v1/erp/users/profile"), None);
    }

    // ===== method_to_action 测试 =====

    #[test]
    fn test_method_to_action_GET映射read() {
        assert_eq!(method_to_action(&Method::GET), "read");
    }

    #[test]
    fn test_method_to_action_POST映射create() {
        assert_eq!(method_to_action(&Method::POST), "create");
    }

    #[test]
    fn test_method_to_action_PUT映射update() {
        assert_eq!(method_to_action(&Method::PUT), "update");
    }

    #[test]
    fn test_method_to_action_PATCH映射update() {
        assert_eq!(method_to_action(&Method::PATCH), "update");
    }

    #[test]
    fn test_method_to_action_DELETE映射delete() {
        assert_eq!(method_to_action(&Method::DELETE), "delete");
    }

    #[test]
    fn test_method_to_action_未知方法映射read() {
        // OPTIONS 等未明确映射的方法默认为 read
        assert_eq!(method_to_action(&Method::OPTIONS), "read");
    }

    // ===== CacheEntry 测试 =====

    #[test]
    fn test_cache_entry_新建未过期() {
        let entry = CacheEntry::new(true, Duration::minutes(5));
        assert!(!entry.is_expired());
        assert!(entry.data);
    }

    #[test]
    fn test_cache_entry_已过期() {
        // 构造一个已过期的缓存项（过期时间为当前时间减 1 分钟）
        let entry = CacheEntry {
            data: false,
            expires_at: Utc::now() - Duration::minutes(1),
        };
        assert!(entry.is_expired());
    }

    // ===== invalidate_permission_cache 测试 =====

    #[test]
    fn test_invalidate_permission_cache_移除指定角色() {
        // 插入缓存条目
        PERMISSION_CACHE.insert(
            9991,
            CacheEntry {
                data: Arc::new(vec![]),
                expires_at: Utc::now() + Duration::minutes(5),
            },
        );
        assert!(PERMISSION_CACHE.contains_key(&9991));

        // 失效指定角色缓存
        invalidate_permission_cache(9991);
        assert!(!PERMISSION_CACHE.contains_key(&9991));
    }

    #[test]
    fn test_invalidate_permission_cache_不存在角色不报错() {
        // 失效不存在的角色缓存不应 panic
        invalidate_permission_cache(99999);
    }

    #[test]
    fn test_invalidate_all_permission_cache_清空全部() {
        // 插入多个缓存条目
        PERMISSION_CACHE.insert(
            9992,
            CacheEntry {
                data: Arc::new(vec![]),
                expires_at: Utc::now() + Duration::minutes(5),
            },
        );
        PERMISSION_CACHE.insert(
            9993,
            CacheEntry {
                data: Arc::new(vec![]),
                expires_at: Utc::now() + Duration::minutes(5),
            },
        );
        assert!(PERMISSION_CACHE.contains_key(&9992));
        assert!(PERMISSION_CACHE.contains_key(&9993));

        // 清空全部
        invalidate_all_permission_cache();
        assert!(PERMISSION_CACHE.is_empty());
    }

    // ===== V15 P1-14.11-C：缓存失效生命周期测试（insert→invalidate→reload→expiry 完整链路）=====

    /// 构造带权限数据的缓存条目，用于生命周期测试
    fn make_cache_entry(
        permissions: Vec<role_permission::Model>,
        ttl_minutes: i64,
    ) -> CacheEntry<Arc<Vec<role_permission::Model>>> {
        CacheEntry {
            data: Arc::new(permissions),
            expires_at: Utc::now() + Duration::minutes(ttl_minutes),
        }
    }

    /// 构造已过期的缓存条目
    fn make_expired_cache_entry(
        permissions: Vec<role_permission::Model>,
    ) -> CacheEntry<Arc<Vec<role_permission::Model>>> {
        CacheEntry {
            data: Arc::new(permissions),
            expires_at: Utc::now() - Duration::minutes(1),
        }
    }

    /// 生命周期场景 1：insert → invalidate → reload 完整链路
    #[test]
    fn test_lifecycle_insert_invalidate_reload() {
        let role_id = 88001;
        // 清理可能的残留
        PERMISSION_CACHE.remove(&role_id);

        // 1. insert：插入权限缓存
        let perms_v1 = vec![make_permission("users", None, "read")];
        PERMISSION_CACHE.insert(role_id, make_cache_entry(perms_v1.clone(), 5));
        assert!(
            PERMISSION_CACHE.contains_key(&role_id),
            "insert 后缓存应存在"
        );
        assert_eq!(
            PERMISSION_CACHE.get(&role_id).unwrap().data.len(),
            1,
            "缓存应含 1 条权限"
        );

        // 2. invalidate：失效缓存
        invalidate_permission_cache(role_id);
        assert!(
            !PERMISSION_CACHE.contains_key(&role_id),
            "invalidate 后缓存应被移除"
        );

        // 3. reload：重新加载（模拟 check_permission 重新查询 DB 后回填缓存）
        let perms_v2 = vec![
            make_permission("users", None, "read"),
            make_permission("users", None, "create"),
        ];
        PERMISSION_CACHE.insert(role_id, make_cache_entry(perms_v2.clone(), 5));
        assert!(
            PERMISSION_CACHE.contains_key(&role_id),
            "reload 后缓存应重新存在"
        );
        assert_eq!(
            PERMISSION_CACHE.get(&role_id).unwrap().data.len(),
            2,
            "reload 后应含 2 条权限（数据已更新）"
        );

        // 清理
        invalidate_permission_cache(role_id);
    }

    /// 生命周期场景 2：insert → expiry → reload 过期触发重新加载链路
    #[test]
    fn test_lifecycle_insert_expiry_reload() {
        let role_id = 88002;
        PERMISSION_CACHE.remove(&role_id);

        // 1. insert：插入已过期的缓存条目（模拟 TTL 到期）
        let perms_v1 = vec![make_permission("orders", None, "read")];
        PERMISSION_CACHE.insert(role_id, make_expired_cache_entry(perms_v1.clone()));
        assert!(PERMISSION_CACHE.contains_key(&role_id), "缓存条目存在");
        assert!(
            PERMISSION_CACHE.get(&role_id).unwrap().is_expired(),
            "缓存条目应已过期"
        );

        // 2. expiry：过期后应被 is_expired 识别（模拟 check_permission 中的过期清理逻辑）
        let cached = PERMISSION_CACHE.get(&role_id).unwrap();
        if cached.is_expired() {
            drop(cached);
            PERMISSION_CACHE.remove(&role_id);
        }
        assert!(!PERMISSION_CACHE.contains_key(&role_id), "过期条目应被移除");

        // 3. reload：重新加载新数据
        let perms_v2 = vec![
            make_permission("orders", None, "read"),
            make_permission("orders", None, "export"),
        ];
        PERMISSION_CACHE.insert(role_id, make_cache_entry(perms_v2.clone(), 5));
        assert!(
            !PERMISSION_CACHE.get(&role_id).unwrap().is_expired(),
            "新条目不应过期"
        );
        assert_eq!(
            PERMISSION_CACHE.get(&role_id).unwrap().data.len(),
            2,
            "reload 后应含 2 条权限"
        );

        // 清理
        invalidate_permission_cache(role_id);
    }

    /// 生命周期场景 3：完整链路 insert → read(hit) → invalidate → read(miss) → reload → read(hit) → expiry → read(miss)
    #[test]
    fn test_lifecycle_complete_chain() {
        let role_id = 88003;
        PERMISSION_CACHE.remove(&role_id);

        // 1. insert
        let perms = vec![make_permission("products", None, "read")];
        PERMISSION_CACHE.insert(role_id, make_cache_entry(perms.clone(), 5));

        // 2. read(hit)：缓存命中
        assert!(PERMISSION_CACHE.contains_key(&role_id), "缓存应命中");
        let hit_data = PERMISSION_CACHE.get(&role_id).unwrap().data.clone();
        assert_eq!(hit_data.len(), 1, "命中数据应含 1 条权限");
        drop(hit_data);

        // 3. invalidate：失效缓存
        invalidate_permission_cache(role_id);

        // 4. read(miss)：缓存未命中
        assert!(!PERMISSION_CACHE.contains_key(&role_id), "缓存应未命中");

        // 5. reload：重新加载（含新权限）
        let perms_v2 = vec![
            make_permission("products", None, "read"),
            make_permission("products", None, "create"),
            make_permission("products", None, "update"),
        ];
        PERMISSION_CACHE.insert(role_id, make_cache_entry(perms_v2.clone(), 5));

        // 6. read(hit)：重新命中，数据已更新
        assert!(PERMISSION_CACHE.contains_key(&role_id), "reload 后应命中");
        assert_eq!(
            PERMISSION_CACHE.get(&role_id).unwrap().data.len(),
            3,
            "应含 3 条权限（数据已更新）"
        );

        // 7. expiry：模拟 TTL 到期（替换为已过期条目）
        PERMISSION_CACHE.insert(role_id, make_expired_cache_entry(perms_v2.clone()));
        assert!(
            PERMISSION_CACHE.get(&role_id).unwrap().is_expired(),
            "条目应已过期"
        );

        // 8. read(miss)：过期后视为未命中（模拟 check_permission 的过期检测逻辑）
        let is_miss = match PERMISSION_CACHE.get(&role_id) {
            Some(entry) => entry.is_expired(),
            None => true,
        };
        assert!(is_miss, "过期条目应视为未命中");

        // 清理
        invalidate_permission_cache(role_id);
    }

    /// 生命周期场景 4：多角色并发缓存生命周期隔离
    #[test]
    fn test_lifecycle_multi_role_isolation() {
        let role_a = 88004;
        let role_b = 88005;
        PERMISSION_CACHE.remove(&role_a);
        PERMISSION_CACHE.remove(&role_b);

        // 两个角色同时缓存
        PERMISSION_CACHE.insert(
            role_a,
            make_cache_entry(vec![make_permission("a", None, "read")], 5),
        );
        PERMISSION_CACHE.insert(
            role_b,
            make_cache_entry(vec![make_permission("b", None, "read")], 5),
        );

        // 失效 role_a，role_b 不受影响
        invalidate_permission_cache(role_a);
        assert!(!PERMISSION_CACHE.contains_key(&role_a), "role_a 应被失效");
        assert!(PERMISSION_CACHE.contains_key(&role_b), "role_b 不应受影响");

        // reload role_a
        PERMISSION_CACHE.insert(
            role_a,
            make_cache_entry(vec![make_permission("a", None, "read")], 5),
        );
        assert!(
            PERMISSION_CACHE.contains_key(&role_a),
            "role_a reload 后应存在"
        );

        // 清理
        invalidate_permission_cache(role_a);
        invalidate_permission_cache(role_b);
    }

    /// 生命周期场景 5：invalidate_all 后所有角色缓存全清空，可重新加载
    #[test]
    fn test_lifecycle_invalidate_all_then_reload() {
        let role_ids = [88006, 88007, 88008];
        for &rid in &role_ids {
            PERMISSION_CACHE.remove(&rid);
        }

        // 插入多个角色缓存
        for &rid in &role_ids {
            PERMISSION_CACHE.insert(
                rid,
                make_cache_entry(vec![make_permission("x", None, "read")], 5),
            );
        }

        // invalidate_all 清空全部
        invalidate_all_permission_cache();
        for &rid in &role_ids {
            assert!(
                !PERMISSION_CACHE.contains_key(&rid),
                "角色 {} 应被清空",
                rid
            );
        }

        // 重新加载单个角色
        PERMISSION_CACHE.insert(
            role_ids[0],
            make_cache_entry(vec![make_permission("y", None, "read")], 5),
        );
        assert!(
            PERMISSION_CACHE.contains_key(&role_ids[0]),
            "reload 后应存在"
        );
        assert!(
            !PERMISSION_CACHE.contains_key(&role_ids[1]),
            "其他角色仍应被清空"
        );

        // 清理
        invalidate_all_permission_cache();
    }

    // ===== extract_action_from_query 测试（V15 P0-S10）=====

    #[test]
    fn test_extract_action_from_query_print动作() {
        let uri: axum::http::Uri = "/api/v1/erp/sales/orders?action=print".parse().unwrap();
        assert_eq!(extract_action_from_query(&uri), Some("print".to_string()));
    }

    #[test]
    fn test_extract_action_from_query_export动作() {
        let uri: axum::http::Uri = "/api/v1/erp/inventory/stocks?action=export"
            .parse()
            .unwrap();
        assert_eq!(extract_action_from_query(&uri), Some("export".to_string()));
    }

    #[test]
    fn test_extract_action_from_query_download动作() {
        let uri: axum::http::Uri = "/api/v1/erp/reports/finance?action=download"
            .parse()
            .unwrap();
        assert_eq!(
            extract_action_from_query(&uri),
            Some("download".to_string())
        );
    }

    #[test]
    fn test_extract_action_from_query_无action参数返回None() {
        let uri: axum::http::Uri = "/api/v1/erp/sales/orders?page=1".parse().unwrap();
        assert_eq!(extract_action_from_query(&uri), None);
    }

    #[test]
    fn test_extract_action_from_query_白名单外动作返回None() {
        // action=read 不在白名单中，防止客户端绕过权限
        let uri: axum::http::Uri = "/api/v1/erp/sales/orders?action=read".parse().unwrap();
        assert_eq!(extract_action_from_query(&uri), None);
    }

    #[test]
    fn test_extract_action_from_query_无查询字符串返回None() {
        let uri: axum::http::Uri = "/api/v1/erp/sales/orders".parse().unwrap();
        assert_eq!(extract_action_from_query(&uri), None);
    }

    #[test]
    fn test_extract_action_from_query_多参数识别action() {
        let uri: axum::http::Uri = "/api/v1/erp/sales/orders?page=1&action=print&format=pdf"
            .parse()
            .unwrap();
        assert_eq!(extract_action_from_query(&uri), Some("print".to_string()));
    }

    #[test]
    fn test_extract_action_from_query_url编码解码() {
        // %70%72%69%6e%74 = "print"
        let uri: axum::http::Uri = "/api/v1/erp/sales/orders?action=%70%72%69%6e%74"
            .parse()
            .unwrap();
        assert_eq!(extract_action_from_query(&uri), Some("print".to_string()));
    }

    // ===== matches_permission 测试（安全核心）=====

    #[test]
    fn test_matches_permission_类型不匹配返回false() {
        let p = make_permission("users", None, "read");
        assert!(!matches_permission(&p, "orders", None, "read"));
    }

    #[test]
    fn test_matches_permission_全部匹配无ID() {
        let p = make_permission("users", None, "read");
        assert!(matches_permission(&p, "users", None, "read"));
    }

    #[test]
    fn test_matches_permission_action通配符匹配() {
        let p = make_permission("users", None, "*");
        assert!(matches_permission(&p, "users", None, "read"));
        assert!(matches_permission(&p, "users", None, "create"));
        assert!(matches_permission(&p, "users", None, "delete"));
    }

    #[test]
    fn test_matches_permission_ID精确匹配相等() {
        let p = make_permission("users", Some(100), "read");
        assert!(matches_permission(&p, "users", Some(100), "read"));
    }

    #[test]
    fn test_matches_permission_ID精确匹配不等返回false() {
        // 垂直越权防护：权限 ID=100 不能访问 ID=200
        let p = make_permission("users", Some(100), "read");
        assert!(!matches_permission(&p, "users", Some(200), "read"));
    }

    #[test]
    fn test_matches_permission_权限无ID请求有ID返回false() {
        // M-6 修复点：权限 resource_id=None 不能匹配请求 resource_id=Some
        // 防止拥有全局权限的用户操作特定资源（应通过 action="*" 明确授予）
        let p = make_permission("users", None, "read");
        assert!(!matches_permission(&p, "users", Some(100), "read"));
    }

    #[test]
    fn test_matches_permission_权限有ID请求无ID返回false() {
        let p = make_permission("users", Some(100), "read");
        assert!(!matches_permission(&p, "users", None, "read"));
    }

    #[test]
    fn test_matches_permission_action不匹配且非通配符返回false() {
        let p = make_permission("users", None, "read");
        assert!(!matches_permission(&p, "users", None, "delete"));
    }

    #[test]
    fn test_matches_permission_通配符加ID精确匹配() {
        // action="*" + resource_id 精确匹配的组合
        let p = make_permission("users", Some(100), "*");
        assert!(matches_permission(&p, "users", Some(100), "update"));
        assert!(!matches_permission(&p, "users", Some(200), "update"));
    }
}
