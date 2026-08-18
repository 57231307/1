use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::middleware::public_routes::is_public_path;
use crate::models::audit_log::{OperationType, Severity};
use crate::models::role_permission;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::utils::admin_checker;
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
use sea_orm::{ColumnTrait, DatabaseConnectionType, EntityTrait, QueryFilter};
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
fn extract_auth_context(request: &Request<Body>) -> Result<AuthContext, Box<Response>> {
    match request.extensions().get::<AuthContext>().cloned() {
        Some(auth) => Ok(auth),
        None => {
            warn!("缺少认证上下文");
            Err(Box::new(unauthorized_response("缺少认证上下文")))
        }
    }
}

/// 从认证上下文提取 role_id，缺失时返回 403
fn extract_role_id(auth: &AuthContext) -> Result<i32, Box<Response>> {
    match auth.role_id {
        Some(id) => Ok(id),
        None => {
            warn!("用户没有关联角色，拒绝访问");
            Err(Box::new(forbidden_response("没有关联角色，无法访问")))
        }
    }
}

/// V15 P0-S21：校验 segment3 是否在已知资源白名单中
fn validate_route_whitelist(path: &str) -> Result<(), Box<Response>> {
    if let Some(segment3) = extract_segment3(path) {
        if !is_known_resource_segment(segment3) {
            warn!(
                "拒绝未知路由: path={}, segment3={} 不在白名单中",
                path, segment3
            );
            return Err(Box::new(forbidden_response("未知的资源路径")));
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

    let auth = extract_auth_context(&request).map_err(|e| *e)?;
    tracing::debug!("权限检查: user_id={}, path={}", auth.user_id, path);

    let role_id = extract_role_id(&auth).map_err(|e| *e)?;

    // V15 P1-5-3：认证豁免 RBAC 路径（如前端打印审计埋点），仅需认证 + role_id
    if is_auth_only_path(path) {
        tracing::debug!(
            "认证豁免 RBAC 路径放行: path={}, user_id={}",
            path,
            auth.user_id
        );
        return Ok(next.run(request).await);
    }

    validate_route_whitelist(path).map_err(|e| *e)?;
    let (resource_type, resource_id, action) = extract_route_info(path, uri, method);

    // V15 P2 B12-P2-13：resource_type="unknown" 时 fail-closed，直接拒绝
    // extract_resource_info 对不符合 /api/v1/erp/... 前缀的路径返回 "unknown"，
    // 此时不应继续权限匹配（可能误放行），直接拒绝并记录审计日志。
    if resource_type == "unknown" {
        warn!("未知资源类型，拒绝访问: path={} {}", method, path);
        record_permission_denial(
            &state.audit_log,
            &auth,
            method,
            path,
            "unknown",
            resource_id,
            &action,
        );
        return Err(forbidden_response("权限不足，无法访问该资源"));
    }

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
pub fn extract_segment3(path: &str) -> Option<&str> {
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
pub fn extract_action_from_path(path: &str) -> Option<String> {
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
pub fn extract_action_from_query(uri: &axum::http::Uri) -> Option<String> {
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

pub fn extract_resource_info(path: &str) -> (String, Option<i32>) {
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
        // V15 P2 B12-P2-13：路径不符合 /api/v1/erp/... 前缀，记录 warn 便于发现配置错误
        tracing::warn!(
            resource = %path,
            "extract_resource_info 返回 unknown，可能存在配置错误"
        );
        ("unknown".to_string(), None)
    }
}

pub fn method_to_action(method: &Method) -> String {
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
pub struct CacheEntry<T: Clone> {
    pub payload: T,
    pub expires_at: DateTime<Utc>,
}

impl<T: Clone> CacheEntry<T> {
    pub fn new(payload: T, ttl: Duration) -> Self {
        Self {
            payload,
            expires_at: Utc::now() + ttl,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

// Cache: role_id -> CacheEntry<Arc<Vec<role_permission::Model>>>
// 使用 Arc 包装，克隆时只增加引用计数，不复制数据
pub static PERMISSION_CACHE: LazyLock<DashMap<i32, CacheEntry<Arc<Vec<role_permission::Model>>>>> =
    LazyLock::new(DashMap::new);

/// 权限缓存 TTL（分钟），可通过环境变量 PERMISSION_CACHE_TTL_MINS 配置，默认 5 分钟。
// B03-P2-3 修复：原硬编码 const 5 分钟，现改为启动时读取环境变量，便于按部署规模调优；
// 非法值（非数字/<=0）回退为默认 5 分钟，避免配置错误导致缓存失效或永驻。
static PERMISSION_CACHE_TTL_MINS: LazyLock<i64> = LazyLock::new(|| {
    let raw = std::env::var("PERMISSION_CACHE_TTL_MINS").unwrap_or_else(|_| "5".to_string());
    let mins = raw.parse::<i64>().unwrap_or(5).max(1);
    if std::env::var("PERMISSION_CACHE_TTL_MINS").is_err() {
        tracing::info!("PERMISSION_CACHE_TTL_MINS 未设置，使用默认值 5 分钟");
    } else {
        tracing::info!(value = %raw, mins, "PERMISSION_CACHE_TTL_MINS 已设置");
    }
    mins
});

/// V15 P0-S07：失效指定角色的权限缓存（P1-14.9-C 同步发布 Redis pub/sub，多实例失效）
pub fn invalidate_permission_cache(role_id: i32) {
    PERMISSION_CACHE.remove(&role_id);
    tracing::info!(role_id, "权限缓存已失效");
    // V15 P1-14.9-C：发布 Redis pub/sub 通知（异步，不阻塞调用方）
    // 无 Tokio runtime（如同步测试）时安全跳过 spawn
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        let channel = PERMISSION_CACHE_INVALIDATION_CHANNEL;
        let message = format!("{}", role_id);
        handle.spawn(async move {
            crate::utils::redis_cache::publish_to_channel(channel, &message).await;
        });
    }
}

/// V15 P0-S07：失效全部权限缓存（P1-14.9-C 同步发布 Redis pub/sub "ALL"，多实例清空）
#[allow(dead_code)]
pub fn invalidate_all_permission_cache() {
    PERMISSION_CACHE.clear();
    tracing::info!("全部权限缓存已失效");
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        let channel = PERMISSION_CACHE_INVALIDATION_CHANNEL;
        handle.spawn(async move {
            crate::utils::redis_cache::publish_to_channel(channel, "ALL").await;
        });
    }
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
    // Mock/Disconnected 连接用于测试环境，fail-closed 返回 false
    if matches!(db.inner, DatabaseConnectionType::Disconnected) {
        return false;
    }
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
            Some(cached.payload.clone())
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
            let ttl = Duration::minutes(*PERMISSION_CACHE_TTL_MINS);
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
/// V15 P2 14.11-F：resource_type 支持 "*" 通配（超级权限码 "resource:*" 或 "*:*"）
pub fn matches_permission(
    p: &role_permission::Model,
    resource_type: &str,
    resource_id: Option<i32>,
    action: &str,
) -> bool {
    let resource_match =
        p.resource_type == resource_type || p.resource_type == "*" || resource_type == "*";
    // 超级通配（resource_type="*"）豁免 resource_id 垂直越权防护
    let id_match = p.resource_type == "*"
        || match (p.resource_id, resource_id) {
            (None, None) => true,
            (Some(pid), Some(rid)) => pid == rid,
            _ => false,
        };
    resource_match && (p.action == action || p.action == "*") && id_match
}
