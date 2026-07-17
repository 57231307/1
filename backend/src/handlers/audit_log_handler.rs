//! 审计日志查询 Handler（P13 批 1 P3-2 增强版）
//!
//! 提供：
//! - GET    /api/v1/erp/audit-logs          分页 + 多维筛选
//! - GET    /api/v1/erp/audit-logs/{id}      详情
//! - GET    /api/v1/erp/audit-logs/export    xlsx 导出

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::models::audit_log;
use crate::utils::admin_checker::is_admin_role;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use crate::utils::sql_escape::safe_like_pattern;
// V15 P0-S15 修复（Batch 475a）：导出注入水印（操作员/导出时间/导出条数）
use crate::utils::xlsx_export::{build_xlsx_response_with_watermark, WatermarkConfig, XlsxTable};

/// P0 8-5 修复：审计日志查询要求 admin 角色
///
/// 安全原因：审计日志含全系统操作记录（含其他用户敏感操作），
/// 仅依赖全局 permission_middleware 的 RBAC 不够（管理员可能误配 audit-logs:read 权限），
/// 在 handler 层增加 admin 角色深度防御，确保合规要求。
async fn require_admin_role(
    state: &AppState,
    auth: &AuthContext,
) -> Result<(), AppError> {
    let role_id = auth
        .role_id
        .ok_or_else(|| AppError::permission_denied("用户未分配角色，无法查询审计日志"))?;
    if !is_admin_role(&state.db, role_id).await {
        tracing::warn!(
            target: "security_audit",
            event = "AUTHORIZATION_DENIED",
            user_id = auth.user_id,
            username = %auth.username,
            "[SECURITY] 非 admin 用户尝试查询审计日志被拒绝"
        );
        return Err(AppError::permission_denied(
            "查询审计日志仅限管理员（code=admin）执行",
        ));
    }
    Ok(())
}

/// 列表查询参数（全部可选）
#[derive(Debug, Default, Deserialize)]
// P1-13 修复（2026-06-25）：路由已挂载至 system::routes()，函数标记已移除。
// 结构体字段经 serde Deserialize 派生使用，标记保留待编译器验证后清理。
pub struct AuditLogListQuery {
    /// 起始时间（RFC3339 / ISO8601）
    pub start_time: Option<String>,
    /// 截止时间（RFC3339 / ISO8601）
    pub end_time: Option<String>,
    /// 用户 ID 筛选
    pub user_id: Option<i32>,
    /// 操作类型筛选（CREATE / UPDATE / ...）
    pub operation_type: Option<String>,
    /// 严重级别筛选（INFO / WARN / ERROR / CRITICAL）
    pub severity: Option<String>,
    /// 资源类型筛选（如 `user` / `order`）
    pub resource_type: Option<String>,
    /// 请求追踪 ID 筛选
    pub request_id: Option<String>,
    /// 模糊搜索资源 ID / 资源名
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 列表返回项（前端展示用）
#[derive(Debug, Serialize)]
// P1-13 修复（2026-06-25）：路由已挂载至 system::routes()，函数标记已移除。
// 结构体字段经 serde Serialize 派生使用，标记保留待编译器验证后清理。
pub struct AuditLogListItem {
    pub id: i32,
    pub user_id: Option<i32>,
    pub username: Option<String>,
    pub operation_type: Option<String>,
    pub severity: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub resource_name: Option<String>,
    pub description: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    pub request_method: Option<String>,
    pub request_path: Option<String>,
    pub created_at: Option<String>,
}

impl From<audit_log::Model> for AuditLogListItem {
    fn from(m: audit_log::Model) -> Self {
        Self {
            id: m.id,
            user_id: m.user_id,
            username: m.username,
            operation_type: m.operation_type,
            severity: m.severity,
            resource_type: m.resource_type,
            resource_id: m.resource_id,
            resource_name: m.resource_name,
            description: m.description,
            ip_address: m.ip_address,
            user_agent: m.user_agent,
            request_id: m.request_id,
            request_method: m.request_method,
            request_path: m.request_path,
            created_at: m.created_at.map(|t| t.to_rfc3339()),
        }
    }
}

/// 列表返回结构
#[derive(Debug, Serialize)]
// P1-13 修复（2026-06-25）：路由已挂载至 system::routes()，函数标记已移除。
pub struct AuditLogListResponse {
    pub items: Vec<AuditLogListItem>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// GET /api/v1/erp/audit-logs
///
/// 分页 + 多维筛选（时间范围 / user_id / operation_type / severity / resource_type / request_id）
pub async fn list_audit_logs(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<AuditLogListQuery>,
) -> Result<Json<ApiResponse<AuditLogListResponse>>, AppError> {
    // P0 8-5 修复：审计日志查询仅限 admin
    require_admin_role(&state, &auth).await?;

    let page = std::cmp::Ord::max(query.page.unwrap_or(1), 1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let mut q = audit_log::Entity::find();

    if let Some(start) = &query.start_time {
        if let Ok(ts) = start.parse::<DateTime<Utc>>() {
            q = q.filter(audit_log::Column::CreatedAt.gte(ts.naive_utc()));
        }
    }
    if let Some(end) = &query.end_time {
        if let Ok(ts) = end.parse::<DateTime<Utc>>() {
            q = q.filter(audit_log::Column::CreatedAt.lte(ts.naive_utc()));
        }
    }
    if let Some(uid) = query.user_id {
        q = q.filter(audit_log::Column::UserId.eq(uid));
    }
    if let Some(op) = &query.operation_type {
        q = q.filter(audit_log::Column::OperationType.eq(op.clone()));
    }
    if let Some(sev) = &query.severity {
        q = q.filter(audit_log::Column::Severity.eq(sev.clone()));
    }
    if let Some(rt) = &query.resource_type {
        q = q.filter(audit_log::Column::ResourceType.eq(rt.clone()));
    }
    if let Some(rid) = &query.request_id {
        q = q.filter(audit_log::Column::RequestId.eq(rid.clone()));
    }
    if let Some(kw) = &query.keyword {
        // 批次 94 P2-3 修复：LIKE 模式注入，转义 % _ \ 特殊字符
        let pattern = safe_like_pattern(kw);
        q = q.filter(
            audit_log::Column::ResourceId
                .like(pattern.clone())
                .or(audit_log::Column::ResourceName.like(pattern.clone()))
                .or(audit_log::Column::Description.like(pattern)),
        );
    }

    let paginator = q
        .order_by_desc(audit_log::Column::CreatedAt)
        .paginate(state.db.as_ref(), page_size);

    let total = paginator
        .num_items()
        .await
        .map_err(|e| AppError::internal(format!("统计审计日志失败: {}", e)))?;
    let logs = paginator
        // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
        .fetch_page(page.clamp(1, 1000).saturating_sub(1))
        .await
        .map_err(|e| AppError::internal(format!("查询审计日志失败: {}", e)))?;

    let items: Vec<AuditLogListItem> = logs.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(AuditLogListResponse {
        items,
        total,
        page,
        page_size,
    })))
}

/// 审计日志详情（含 before/after 快照原始 JSON）
#[derive(Debug, Serialize)]
// P1-13 修复（2026-06-25）：路由已挂载至 system::routes()，函数标记已移除。
// base 字段经 #[serde(flatten)] 使用，其余字段经 Serialize 派生使用。
pub struct AuditLogDetailResponse {
    #[serde(flatten)]
    pub base: AuditLogListItem,
    /// 变更前快照（直接序列化 AuditValue 内部 Value）
    pub before_snapshot: Option<serde_json::Value>,
    /// 变更后快照
    pub after_snapshot: Option<serde_json::Value>,
    /// 旧字段 old_value（兼容字段）
    pub old_value: Option<serde_json::Value>,
    /// 旧字段 new_value（兼容字段）
    pub new_value: Option<serde_json::Value>,
}

/// GET /api/v1/erp/audit-logs/{id}
pub async fn get_audit_log(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<AuditLogDetailResponse>>, AppError> {
    // P0 8-5 修复：审计日志详情查询仅限 admin
    require_admin_role(&state, &auth).await?;

    let log = audit_log::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| AppError::internal(format!("查询审计日志失败: {}", e)))?
        .ok_or_else(|| AppError::not_found("审计日志不存在"))?;

    let response = AuditLogDetailResponse {
        base: log.clone().into(),
        before_snapshot: log.before_snapshot.map(|v| v.0),
        after_snapshot: log.after_snapshot.map(|v| v.0),
        old_value: log.old_value.map(|v| v.0),
        new_value: log.new_value.map(|v| v.0),
    };
    Ok(Json(ApiResponse::success(response)))
}

/// GET /api/v1/erp/audit-logs/export
///
/// 返回 xlsx 格式（Excel），前端直接 `window.URL.createObjectURL(blob)` 下载。
pub async fn export_audit_logs(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<AuditLogListQuery>,
) -> Result<axum::response::Response, AppError> {
    // P0 8-5 修复：审计日志导出仅限 admin
    require_admin_role(&state, &auth).await?;

    let mut q = audit_log::Entity::find();
    if let Some(start) = &query.start_time {
        if let Ok(ts) = start.parse::<DateTime<Utc>>() {
            q = q.filter(audit_log::Column::CreatedAt.gte(ts.naive_utc()));
        }
    }
    if let Some(end) = &query.end_time {
        if let Ok(ts) = end.parse::<DateTime<Utc>>() {
            q = q.filter(audit_log::Column::CreatedAt.lte(ts.naive_utc()));
        }
    }
    if let Some(uid) = query.user_id {
        q = q.filter(audit_log::Column::UserId.eq(uid));
    }
    if let Some(op) = &query.operation_type {
        q = q.filter(audit_log::Column::OperationType.eq(op.clone()));
    }
    if let Some(sev) = &query.severity {
        q = q.filter(audit_log::Column::Severity.eq(sev.clone()));
    }
    if let Some(rt) = &query.resource_type {
        q = q.filter(audit_log::Column::ResourceType.eq(rt.clone()));
    }
    if let Some(rid) = &query.request_id {
        q = q.filter(audit_log::Column::RequestId.eq(rid.clone()));
    }

    let logs = q
        .order_by_desc(audit_log::Column::CreatedAt)
        .all(state.db.as_ref())
        .await
        .map_err(|e| AppError::internal(format!("查询审计日志失败: {}", e)))?;

    // V15 P0-S15 修复（Batch 475a）：保存 logs 数量用于水印（logs 后续被 into_iter 消费）
    let logs_count = logs.len();

    // 异步记录导出操作（审计自身）
    {
        use crate::models::audit_log::{OperationType, Severity};
        use crate::services::audit_log_service::{AuditEvent, AuditLogService};
        use std::sync::Arc;
        let svc = AuditLogService::new(state.db.clone());
        let event = AuditEvent {
            user_id: Some(auth.user_id),
            username: Some(auth.username.clone()),
            operation_type: OperationType::Export,
            severity: Severity::Info,
            resource_type: Some("audit_log".to_string()),
            resource_id: None,
            resource_name: Some("审计日志导出".to_string()),
            description: Some(format!("导出 {} 条审计日志", logs.len())),
            request_method: Some("GET".to_string()),
            request_path: Some("/api/v1/erp/audit-logs/export".to_string()),
            before_snapshot: None,
            after_snapshot: None,
        };
        Arc::new(svc).record_async(event, None);
    }

    // 构造 xlsx 表格（按列顺序：id/created_at/user_id/username/operation_type/severity/
    // resource_type/resource_id/description/ip_address/request_id）
    let table = XlsxTable {
        sheet_name: "审计日志".to_string(),
        headers: vec![
            "ID".to_string(),
            "创建时间".to_string(),
            "用户ID".to_string(),
            "用户名".to_string(),
            "操作类型".to_string(),
            "严重级别".to_string(),
            "资源类型".to_string(),
            "资源ID".to_string(),
            "描述".to_string(),
            "IP地址".to_string(),
            "请求ID".to_string(),
        ],
        rows: logs
            .into_iter()
            .map(|log| {
                vec![
                    log.id.to_string(),
                    log.created_at
                        .map(|t| t.to_rfc3339())
                        .unwrap_or_default(),
                    log.user_id
                        .map(|i| i.to_string())
                        .unwrap_or_default(),
                    log.username.unwrap_or_default(),
                    log.operation_type.unwrap_or_default(),
                    log.severity.unwrap_or_default(),
                    log.resource_type.unwrap_or_default(),
                    log.resource_id.unwrap_or_default(),
                    log.description.unwrap_or_default(),
                    log.ip_address.unwrap_or_default(),
                    log.request_id.unwrap_or_default(),
                ]
            })
            .collect(),
    };

    let filename = format!(
        "audit_logs_{}",
        chrono::Utc::now().format("%Y%m%d%H%M%S")
    );

    // V15 P0-S15 修复（Batch 475a）：注入水印（操作员/导出时间/导出条数）
    // 水印行在 xlsx 第 0 行（合并所有列），标题行下移到第 1 行，数据行从第 2 行起
    let watermark = WatermarkConfig {
        operator: Some(auth.username.clone()),
        ip_address: None,
        exported_at: Some(chrono::Utc::now().to_rfc3339()),
        extra: Some(format!("审计日志导出（共 {} 条，仅 admin 可导出）", logs_count)),
    };

    // 规则 3：导出统一使用 xlsx 格式，错误用 AppError 表达，成功返回 200 + xlsx 响应体
    build_xlsx_response_with_watermark(&table, &filename, &watermark)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AuditLogListQuery 默认值：所有可选字段为 None
    #[test]
    fn test_list_query_default_values() {
        let q = AuditLogListQuery::default();
        assert!(q.start_time.is_none());
        assert!(q.end_time.is_none());
        assert!(q.user_id.is_none());
        assert!(q.operation_type.is_none());
        assert!(q.severity.is_none());
        assert!(q.resource_type.is_none());
        assert!(q.request_id.is_none());
        assert!(q.keyword.is_none());
        assert!(q.page.is_none());
        assert!(q.page_size.is_none());
    }
}
