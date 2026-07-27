
use crate::middleware::auth_context::AuthContext;
use crate::middleware::public_routes::is_public_path;
use crate::models::audit_log::{OperationType, Severity};
use crate::models::role_permission;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::utils::admin_checker;
use crate::utils::app_state::AppState;
use crate::utils::path_utils::{is_known_resource_segment, is_module_prefix};
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
    "print",
    "export",
    "import",
    "audit",
    "approve",
    "reject",
    "cancel",
    "close",
    "confirm",
    "submit",
    "release",
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
        let resource_type = if path_parts.len() >= 5 && is_module_prefix(path_parts[3]) {
            path_parts[4].to_string()
        } else {
            path_parts[3].to_string()
        };

        // 尝试提取资源ID（跳过模块前缀）
        // V15 P0-S20 修复：跳过路径中的动作段（如 approve/export/print），
        // 避免动作关键字被误认为资源ID
        let start_idx = if path_parts.len() >= 5 && is_module_prefix(path_parts[3]) {
            5
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

/// V15 P0-S07：失效指定角色的权限缓存（角色权限变更或删除时调用）
pub fn invalidate_permission_cache(role_id: i32) {
    PERMISSION_CACHE.remove(&role_id);
    tracing::info!(role_id, "权限缓存已失效");
}

/// V15 P0-S07：失效全部权限缓存（大规模权限变更时调用）
pub fn invalidate_all_permission_cache() {
    PERMISSION_CACHE.clear();
    tracing::info!("全部权限缓存已失效");
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
    permissions.iter().any(|p| {
        matches_permission(p, resource_type, resource_id, action)
    })
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
        let (rt, rid) = extract_resource_info("/api/v1/erp/purchase/orders");
        assert_eq!(rt, "orders");
        assert_eq!(rid, None);
    }

    // ===== extract_segment3 测试 =====

    #[test]
    fn test_extract_segment3_标准路径() {
        assert_eq!(extract_segment3("/api/v1/erp/users"), Some("users"));
        assert_eq!(
            extract_segment3("/api/v1/erp/sales/orders"),
            Some("sales")
        );
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
        assert_eq!(
            extract_action_from_path("/api/v1/erp/users/profile"),
            None
        );
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

    // ===== extract_action_from_query 测试（V15 P0-S10）=====

    #[test]
    fn test_extract_action_from_query_print动作() {
        let uri: axum::http::Uri = "/api/v1/erp/sales/orders?action=print"
            .parse()
            .unwrap();
        assert_eq!(
            extract_action_from_query(&uri),
            Some("print".to_string())
        );
    }

    #[test]
    fn test_extract_action_from_query_export动作() {
        let uri: axum::http::Uri = "/api/v1/erp/inventory/stocks?action=export"
            .parse()
            .unwrap();
        assert_eq!(
            extract_action_from_query(&uri),
            Some("export".to_string())
        );
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
        let uri: axum::http::Uri = "/api/v1/erp/sales/orders?page=1"
            .parse()
            .unwrap();
        assert_eq!(extract_action_from_query(&uri), None);
    }

    #[test]
    fn test_extract_action_from_query_白名单外动作返回None() {
        // action=read 不在白名单中，防止客户端绕过权限
        let uri: axum::http::Uri = "/api/v1/erp/sales/orders?action=read"
            .parse()
            .unwrap();
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
        assert_eq!(
            extract_action_from_query(&uri),
            Some("print".to_string())
        );
    }

    #[test]
    fn test_extract_action_from_query_url编码解码() {
        // %70%72%69%6e%74 = "print"
        let uri: axum::http::Uri = "/api/v1/erp/sales/orders?action=%70%72%69%6e%74"
            .parse()
            .unwrap();
        assert_eq!(
            extract_action_from_query(&uri),
            Some("print".to_string())
        );
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
