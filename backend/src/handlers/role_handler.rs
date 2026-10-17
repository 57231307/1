use crate::container::AppState;
use crate::middleware::audit_context::AuditContext;
use crate::middleware::auth_context::AuthContext;
use crate::models::audit_log::{OperationType, Severity};
use crate::models::role_conflict;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::role_permission_service::RolePermissionService;
use crate::services::role_permission_service::{
    AssignPermissionRequest, BatchPermissionEntry, CreateRoleRequest, UpdateRoleRequest,
};
use crate::utils::admin_checker::{ADMIN_ROLE_CODE, clear_admin_role_cache, is_admin_role};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    Json,
    extract::{Extension, Path, State},
};
use sea_orm::{EntityTrait, QueryOrder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// C-1 修复：处理器内部 admin 角色二次校验（防 `roles:read` 低权用户提权进入写处理器）。
/// 与全局 `permission_middleware` 形成"粗粒度 + 细粒度 admin 校验"双重防线（批次 24 v6 P1-1）。
async fn require_admin_role(state: &AppState, auth: &AuthContext) -> Result<(), AppError> {
    let role_id = auth
        .role_id
        .ok_or_else(|| AppError::permission_denied("用户未分配角色，无法执行该操作"))?;
    if !is_admin_role(&state.db, role_id).await {
        return Err(AppError::permission_denied(format!(
            "该操作仅限管理员（code={}）执行",
            ADMIN_ROLE_CODE
        )));
    }
    Ok(())
}

/// 角色响应
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Serialize)]
pub struct RoleResponse {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 角色详情响应（包含权限列表）
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Serialize)]
pub struct RoleDetailResponse {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub is_system: bool,
    /// 行级数据范围（roles.data_scope 真值，词表 all/dept/self，
    /// 由 PATCH /roles/{id}/data-scope 写入）
    pub data_scope: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub permissions: Option<Vec<PermissionResponse>>,
}

/// 权限响应
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Serialize)]
pub struct PermissionResponse {
    pub id: i32,
    pub resource_type: String,
    pub resource_id: Option<i32>,
    pub action: String,
    pub allowed: bool,
}

/// 角色列表响应
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Serialize)]
pub struct RoleListResponse {
    pub roles: Vec<RoleResponse>,
    pub total: u64,
}

/// 创建角色请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CreateRolePayload {
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub is_system: Option<bool>,
}

/// 更新角色请求（V15 P1-14.12-E：移除 code 字段，禁止修改 role.code 防提权）
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct UpdateRolePayload {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_system: Option<bool>,
}

/// 分配权限请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct AssignPermissionPayload {
    pub resource_type: String,
    pub resource_id: Option<i32>,
    pub action: String,
    pub allowed: bool,
}

/// 更新行级数据范围请求（PATCH /roles/{id}/data-scope）。
/// 取值词表由服务端用 `DataScope` 往返校验（all/dept/self，小写），未知值 400。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct UpdateDataScopePayload {
    pub data_scope: String,
}

/// 获取角色列表
pub async fn list_roles(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<RoleListResponse>>, AppError> {
    let service = RolePermissionService::new(state.db.clone());

    let roles = service.list_roles().await?;

    let role_responses: Vec<RoleResponse> = roles
        .into_iter()
        .map(|role| RoleResponse {
            id: role.id,
            name: role.name,
            code: role.code,
            description: role.description,
            is_system: role.is_system,
            created_at: role.created_at,
            updated_at: role.updated_at,
        })
        .collect();

    // v9 P1-C 修复：当前接口全量返回无分页，total=len 正确。
    // 注意：若后续添加分页，需改为 COUNT(*) 查询，否则 total 退化为当前页条数。
    let total = role_responses.len() as u64;

    Ok(Json(ApiResponse::success(RoleListResponse {
        roles: role_responses,
        total,
    })))
}

/// 获取角色详情
pub async fn get_role(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<RoleDetailResponse>>, AppError> {
    let service = RolePermissionService::new(state.db.clone());

    // service 返回的本就是 AppError（角色不存在=404），直接透传保留真实 status/code
    let role = service.get_role_detail(id).await?;

    let permissions = role.permission_list.map(|perms| {
        perms
            .into_iter()
            .map(|perm| PermissionResponse {
                id: perm.id,
                resource_type: perm.resource_type,
                resource_id: perm.resource_id,
                action: perm.action,
                allowed: perm.allowed,
            })
            .collect()
    });

    Ok(Json(ApiResponse::success(RoleDetailResponse {
        id: role.id,
        name: role.name,
        code: role.code,
        description: role.description,
        is_system: role.is_system,
        data_scope: role.data_scope,
        created_at: role.created_at,
        updated_at: role.updated_at,
        permissions,
    })))
}

/// 创建角色
pub async fn create_role(
    State(state): State<AppState>,
    auth: AuthContext,
    audit_ctx: Option<Extension<AuditContext>>,
    Json(payload): Json<CreateRolePayload>,
) -> Result<Json<ApiResponse<RoleDetailResponse>>, AppError> {
    require_admin_role(&state, &auth).await?;
    let service = RolePermissionService::new(state.db.clone());

    let request = CreateRoleRequest {
        name: payload.name.clone(),
        code: payload.code.clone(),
        description: payload.description.clone(),
        is_system: payload.is_system,
    };

    let role = service.create_role(request).await?;

    // P1 8-3 修复：create_role 补审计日志
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Create,
        severity: Severity::Info,
        resource_type: Some("role".to_string()),
        resource_id: Some(role.id.to_string()),
        resource_name: Some(role.name.clone()),
        description: Some(format!(
            "管理员 {} 创建角色 {}（code={}）",
            auth.username, role.name, role.code
        )),
        request_method: Some("POST".to_string()),
        request_path: Some("/api/v1/erp/roles".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "role_id": role.id,
            "name": role.name,
            "code": role.code,
            "description": role.description,
            "is_system": role.is_system,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, audit_ctx.map(|e| e.0));

    let permissions = role.permission_list.map(|perms| {
        perms
            .into_iter()
            .map(|perm| PermissionResponse {
                id: perm.id,
                resource_type: perm.resource_type,
                resource_id: perm.resource_id,
                action: perm.action,
                allowed: perm.allowed,
            })
            .collect()
    });

    Ok(Json(ApiResponse::success(RoleDetailResponse {
        id: role.id,
        name: role.name,
        code: role.code,
        description: role.description,
        is_system: role.is_system,
        data_scope: role.data_scope,
        created_at: role.created_at,
        updated_at: role.updated_at,
        permissions,
    })))
}

/// 更新角色
pub async fn update_role(
    State(state): State<AppState>,
    auth: AuthContext,
    audit_ctx: Option<Extension<AuditContext>>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateRolePayload>,
) -> Result<Json<ApiResponse<RoleDetailResponse>>, AppError> {
    require_admin_role(&state, &auth).await?;
    let service = RolePermissionService::new(state.db.clone());

    // P1 8-3 修复：更新前查询旧角色信息作为 before_snapshot
    // service 已返回 AppError（不存在=404），透传保留真实 status/code
    let old_role = service.get_role_detail(id).await?;
    let before_snapshot = serde_json::json!({
        "role_id": old_role.id,
        "name": old_role.name,
        "code": old_role.code,
        "description": old_role.description,
        "is_system": old_role.is_system,
    });

    let request = UpdateRoleRequest {
        name: payload.name.clone(),
        description: payload.description.clone(),
        is_system: payload.is_system,
    };

    let role = service.update_role(id, request, auth.user_id).await?;

    // 批次 103 P2-3 修复：角色更新后清理 admin 缓存，避免 is_admin_role 在 TTL 内返回旧判定
    clear_admin_role_cache(Some(id));

    // P1 8-3 修复：update_role 补审计日志
    let event = build_role_audit_event(&auth, id, &before_snapshot, &role);
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, audit_ctx.map(|e| e.0));

    Ok(Json(ApiResponse::success(build_role_detail_response(role))))
}

/// 更新角色行级数据范围（PATCH /roles/{id}/data-scope，D4 行级真相落 roles.data_scope）
///
/// `roles.data_scope` 是权限链路真正被读取的行级开关（auth 中间件每请求加载注入
/// AuthContext，service 层 apply_data_scope 据此过滤），本端点是该列唯一 API 写入口。
/// 派生权限键 = `roles:update`（seg3=roles；末段 data-scope 非动作关键字，PATCH→update，
/// 与既有 roles:* 同键空间，未新增跨资源键）。词表往返校验在 service 层
/// （仅 all/dept/self 小写三值，未知值显式 400，不静默回退）。
/// 系统角色不允许修改（与 `update_role` 同口径）；data_scope 由 auth 中间件每请求
/// 现读库，无缓存需失效。
pub async fn update_role_data_scope(
    State(state): State<AppState>,
    auth: AuthContext,
    audit_ctx: Option<Extension<AuditContext>>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateDataScopePayload>,
) -> Result<Json<ApiResponse<RoleDetailResponse>>, AppError> {
    require_admin_role(&state, &auth).await?;
    let service = RolePermissionService::new(state.db.clone());

    // 更新前回读旧值，作为审计 before_snapshot（角色不存在=404 由 service 透传）
    let old_role = service.get_role_detail(id).await?;
    let before_snapshot = serde_json::json!({
        "role_id": old_role.id,
        "data_scope": old_role.data_scope,
    });

    let role = service
        .update_role_data_scope(id, &payload.data_scope, auth.user_id)
        .await?;

    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Update,
        severity: Severity::Info,
        resource_type: Some("role".to_string()),
        resource_id: Some(id.to_string()),
        resource_name: Some(role.name.clone()),
        description: Some(format!(
            "管理员 {} 更新角色 {} 的行级数据范围 {} → {}（id={}）",
            auth.username, role.name, old_role.data_scope, role.data_scope, id
        )),
        request_method: Some("PATCH".to_string()),
        request_path: Some(format!("/api/v1/erp/roles/{}/data-scope", id)),
        before_snapshot: Some(before_snapshot),
        after_snapshot: Some(serde_json::json!({
            "role_id": role.id,
            "data_scope": role.data_scope,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, audit_ctx.map(|e| e.0));

    Ok(Json(ApiResponse::success(build_role_detail_response(role))))
}

/// 构造角色更新审计事件
fn build_role_audit_event(
    auth: &AuthContext,
    id: i32,
    before_snapshot: &serde_json::Value,
    role: &crate::services::role_permission_service::RoleDetail,
) -> AuditEvent {
    AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Update,
        severity: Severity::Info,
        resource_type: Some("role".to_string()),
        resource_id: Some(id.to_string()),
        resource_name: Some(role.name.clone()),
        description: Some(format!(
            "管理员 {} 更新角色 {}（id={}）",
            auth.username, role.name, id
        )),
        request_method: Some("PUT".to_string()),
        request_path: Some(format!("/api/v1/erp/roles/{}", id)),
        before_snapshot: Some(before_snapshot.clone()),
        after_snapshot: Some(serde_json::json!({
            "role_id": role.id,
            "name": role.name,
            "code": role.code,
            "description": role.description,
            "is_system": role.is_system,
        })),
    }
}

/// 构造角色详情响应
fn build_role_detail_response(
    role: crate::services::role_permission_service::RoleDetail,
) -> RoleDetailResponse {
    let permissions = role.permission_list.map(|perms| {
        perms
            .into_iter()
            .map(|perm| PermissionResponse {
                id: perm.id,
                resource_type: perm.resource_type,
                resource_id: perm.resource_id,
                action: perm.action,
                allowed: perm.allowed,
            })
            .collect()
    });

    RoleDetailResponse {
        id: role.id,
        name: role.name,
        code: role.code,
        description: role.description,
        is_system: role.is_system,
        data_scope: role.data_scope,
        created_at: role.created_at,
        updated_at: role.updated_at,
        permissions,
    }
}

/// 删除角色
pub async fn delete_role(
    State(state): State<AppState>,
    auth: AuthContext,
    audit_ctx: Option<Extension<AuditContext>>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_admin_role(&state, &auth).await?;
    let service = RolePermissionService::new(state.db.clone());

    // P1 8-3 修复：删除前查询旧角色信息作为 before_snapshot
    // service 已返回 AppError（不存在=404），透传保留真实 status/code
    let old_role = service.get_role_detail(id).await?;

    // P1 2-3 修复（批次 64）：系统内置角色禁止删除
    // 原实现仅 require_admin_role，未检查 is_system 字段，
    // admin 角色被删除后系统永久锁定（无管理员可操作）。
    if old_role.is_system {
        return Err(AppError::bad_request("系统内置角色不可删除"));
    }

    let before_snapshot = serde_json::json!({
        "role_id": old_role.id,
        "name": old_role.name,
        "code": old_role.code,
        "description": old_role.description,
        "is_system": old_role.is_system,
    });

    service.delete_role(id, auth.user_id).await?;

    // 批次 103 P2-3 修复：角色删除后清理 admin 缓存
    // 原因：删除后 role_id 不再有效，但缓存中可能仍存在旧条目，
    // 清理避免后续对已删除 role_id 的查询命中过期缓存返回错误判定。
    clear_admin_role_cache(Some(id));

    // P1 8-3 修复：delete_role 补审计日志
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Delete,
        severity: Severity::Warn,
        resource_type: Some("role".to_string()),
        resource_id: Some(id.to_string()),
        resource_name: Some(old_role.name.clone()),
        description: Some(format!(
            "管理员 {} 删除角色 {}（id={}）",
            auth.username, old_role.name, id
        )),
        request_method: Some("DELETE".to_string()),
        request_path: Some(format!("/api/v1/erp/roles/{}", id)),
        before_snapshot: Some(before_snapshot),
        after_snapshot: None,
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, audit_ctx.map(|e| e.0));

    Ok(Json(ApiResponse::success(())))
}

/// 分配权限
pub async fn assign_permission(
    State(state): State<AppState>,
    auth: AuthContext,
    audit_ctx: Option<Extension<AuditContext>>,
    Path(role_id): Path<i32>,
    Json(payload): Json<AssignPermissionPayload>,
) -> Result<Json<ApiResponse<PermissionResponse>>, AppError> {
    require_admin_role(&state, &auth).await?;
    let service = RolePermissionService::new(state.db.clone());

    let request = AssignPermissionRequest {
        role_id,
        resource_type: payload.resource_type.clone(),
        resource_id: payload.resource_id,
        action: payload.action.clone(),
        allowed: payload.allowed,
    };

    let perm = service.assign_permission(request, auth.user_id).await?;

    // P1 8-3 修复：assign_permission 改用 record_async 落库审计日志（原仅 tracing::warn）
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Update,
        severity: Severity::Warn,
        resource_type: Some("role_permission".to_string()),
        resource_id: Some(perm.id.to_string()),
        resource_name: Some(format!(
            "role_id={}/resource_type={}/action={}",
            role_id, payload.resource_type, payload.action
        )),
        description: Some(format!(
            "管理员 {} 为角色 id={} 分配权限：resource_type={} action={} allowed={}",
            auth.username, role_id, payload.resource_type, payload.action, payload.allowed
        )),
        request_method: Some("POST".to_string()),
        request_path: Some(format!("/api/v1/erp/roles/{}/permissions", role_id)),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "permission_id": perm.id,
            "role_id": role_id,
            "resource_type": payload.resource_type,
            "resource_id": payload.resource_id,
            "action": payload.action,
            "allowed": payload.allowed,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, audit_ctx.map(|e| e.0));

    Ok(Json(ApiResponse::success(PermissionResponse {
        id: perm.id,
        resource_type: perm.resource_type,
        resource_id: perm.resource_id,
        action: perm.action,
        allowed: perm.allowed,
    })))
}

/// 批量整批替换角色级授予（PUT /roles/{id}/permissions）
///
/// B 页提速 + SoD 竞态根治：body 是期望的完整**角色级**授予集
/// `[{resource_type, action, allowed}]`，服务端单事务 diff（新增/仅 allowed 翻转的
/// 更新/期望集外的删除），一次批量审计（before/after 各一份全量快照），SoD 在
/// 写入后的最终集合上复算（并发 create+approve 双双落库的旧路径整体回滚）。
/// 记录级授权行（resource_id=Some）不在替换作用面，仍由单条 POST/DELETE 维护。
/// 派生权限键 = `roles:update`（与单条 POST/DELETE 同键空间，未新增权限键）。
/// 出参为替换后的角色级授予全集（裸数组，与 GET /roles/{id}/permissions 同形状）。
pub async fn replace_role_permissions(
    State(state): State<AppState>,
    auth: AuthContext,
    audit_ctx: Option<Extension<AuditContext>>,
    Path(role_id): Path<i32>,
    Json(payload): Json<Vec<BatchPermissionEntry>>,
) -> Result<Json<ApiResponse<Vec<PermissionResponse>>>, AppError> {
    require_admin_role(&state, &auth).await?;
    let service = RolePermissionService::new(state.db.clone());

    let result = service
        .replace_role_permissions(role_id, payload, auth.user_id)
        .await?;

    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Update,
        severity: Severity::Warn,
        resource_type: Some("role_permission".to_string()),
        resource_id: Some(role_id.to_string()),
        resource_name: Some(format!("role_id={role_id} 批量整批替换角色级授予")),
        description: Some(format!(
            "管理员 {} 对角色 id={role_id} 批量整批替换角色级授予：新增 {}、更新 {}、删除 {}",
            auth.username, result.inserted, result.updated, result.deleted
        )),
        request_method: Some("PUT".to_string()),
        request_path: Some(format!("/api/v1/erp/roles/{}/permissions", role_id)),
        before_snapshot: Some(result.before_snapshot),
        after_snapshot: Some(result.after_snapshot),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, audit_ctx.map(|e| e.0));

    let perm_responses: Vec<PermissionResponse> = result
        .after
        .into_iter()
        .map(|perm| PermissionResponse {
            id: perm.id,
            resource_type: perm.resource_type,
            resource_id: perm.resource_id,
            action: perm.action,
            allowed: perm.allowed,
        })
        .collect();

    Ok(Json(ApiResponse::success(perm_responses)))
}

/// 移除权限
pub async fn remove_permission(
    State(state): State<AppState>,
    auth: AuthContext,
    audit_ctx: Option<Extension<AuditContext>>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_admin_role(&state, &auth).await?;
    let service = RolePermissionService::new(state.db.clone());

    service.remove_permission(id, auth.user_id).await?;

    // P1 8-3 修复：remove_permission 改用 record_async 落库审计日志（原仅 tracing::warn）
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Delete,
        severity: Severity::Warn,
        resource_type: Some("role_permission".to_string()),
        resource_id: Some(id.to_string()),
        resource_name: None,
        description: Some(format!("管理员 {} 移除权限 id={}", auth.username, id)),
        request_method: Some("DELETE".to_string()),
        request_path: Some(format!("/api/v1/erp/roles/permissions/{}", id)),
        before_snapshot: Some(serde_json::json!({
            "permission_id": id,
            "action": "remove",
        })),
        after_snapshot: None,
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, audit_ctx.map(|e| e.0));

    Ok(Json(ApiResponse::success(())))
}

/// 获取角色权限列表
pub async fn get_role_permissions(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(role_id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<PermissionResponse>>>, AppError> {
    let service = RolePermissionService::new(state.db.clone());

    let permissions = service.get_role_permissions(role_id).await?;

    let perm_responses: Vec<PermissionResponse> = permissions
        .into_iter()
        .map(|perm| PermissionResponse {
            id: perm.id,
            resource_type: perm.resource_type,
            resource_id: perm.resource_id,
            action: perm.action,
            allowed: perm.allowed,
        })
        .collect();

    Ok(Json(ApiResponse::success(perm_responses)))
}

/// 获取可授予权限目录（GET /roles/permissions、GET /permissions 同一实现）。
///
/// 目录 = 后端权威注册表 `PERMISSION_RESOURCES`（init_service.rs，权限中间件键派生的
/// 资源侧同源）× `assignable_actions()`（键派生动作侧全集：方法派生动作 ∪ 路径末段
/// 动作关键字 ∪ `?action=` 查询白名单，permission.rs 内单一事实源，不另写第二套动作字面量）。
/// 原实现由 role_permission「已授权行 distinct」派生且过滤 Allowed=true：从未被任何角色
/// 授予的资源×动作在 UI 永远勾不出（缺键正解在后端注册表、不在前端清单——既有判例）。
/// 现在 role_permission 只贡献「已勾选态」（GET /roles/{id}/permissions），目录本身后端供。
///
/// 出参边界（消费方影响）：
/// - `id` 是目录序号（1..n，仅排序用途），**不是 role_permission 关联行主键**；
///   DELETE /roles/permissions/{id} 必须使用 GET /roles/{id}/permissions 返回的关联 id；
/// - `resource_id` 恒 null：目录是角色级「资源×动作」可选面，记录级授权仍走单条 POST；
/// - `allowed` 恒 true，表示「该项可授予」，不代表已授予态；
/// - 目录行是派生键的笛卡尔积，个别组合（如 `users:print`）可能没有对应路由面——
///   勾了不生效但不构成越权面（键派生与目录同源）；按需收敛需先建「资源×动作」
///   路由级注册表，不在本轮。
pub async fn list_permissions(
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<PermissionResponse>>>, AppError> {
    let actions = crate::middleware::permission::assignable_actions();
    let resources = crate::services::init_service::PERMISSION_RESOURCES;
    let mut permissions = Vec::with_capacity(resources.len() * actions.len());
    let mut id_counter = 1i32;
    for resource_type in resources {
        for action in &actions {
            permissions.push(PermissionResponse {
                id: id_counter,
                resource_type: (*resource_type).to_string(),
                resource_id: None,
                action: (*action).to_string(),
                allowed: true,
            });
            id_counter += 1;
        }
    }

    tracing::debug!(
        resources = resources.len(),
        actions = actions.len(),
        entries = permissions.len(),
        "权限目录由 PERMISSION_RESOURCES × assignable_actions 派生（不再取 role_permission distinct）"
    );
    Ok(Json(ApiResponse::success(permissions)))
}

/// GET /api/v1/erp/roles/conflicts
/// 角色互斥规则（SoD 职责分离）列表
#[allow(dead_code, reason = "E2E P1-1 与前端设置页消费")]
pub async fn list_role_conflicts(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<role_conflict::Model>>>, AppError> {
    let items = role_conflict::Entity::find()
        .order_by_asc(role_conflict::Column::Id)
        .all(state.db.as_ref())
        .await?;
    Ok(Json(ApiResponse::success(items)))
}
