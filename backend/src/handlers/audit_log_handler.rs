//! 审计日志查询 Handler（P13 批 1 P3-2 增强版）
//!
//! 提供：
//! - GET    /api/v1/erp/audit-logs          分页 + 多维筛选
//! - GET    /api/v1/erp/audit-logs/{id}      详情
//! - GET    /api/v1/erp/audit-logs/export    CSV 导出
//!
//! 强租户隔离：所有 query 必须使用 `extract_tenant_id(&auth)?` 提取租户 ID；
//! 严禁使用 `auth.tenant_id.unwrap_or(0)` 绕过。

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::middleware::tenant::extract_tenant_id;
use crate::models::audit_log;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 列表查询参数（全部可选）
#[derive(Debug, Default, Deserialize)]
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
pub struct AuditLogListItem {
    pub id: i32,
    pub tenant_id: Option<i32>,
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
            tenant_id: m.tenant_id,
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
    let tenant_id = extract_tenant_id(&auth)?;
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

    let mut q = audit_log::Entity::find().filter(audit_log::Column::TenantId.eq(tenant_id));

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
        let pattern = format!("%{}%", kw);
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
        .fetch_page(page - 1)
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
    let tenant_id = extract_tenant_id(&auth)?;
    let log = audit_log::Entity::find_by_id(id)
        .filter(audit_log::Column::TenantId.eq(tenant_id))
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
/// 返回 CSV 格式（text/csv），前端直接 `window.URL.createObjectURL(blob)` 下载。
pub async fn export_audit_logs(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<AuditLogListQuery>,
) -> Result<impl IntoResponse, AppError> {
    let tenant_id = extract_tenant_id(&auth)?;

    let mut q = audit_log::Entity::find().filter(audit_log::Column::TenantId.eq(tenant_id));
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

    // 异步记录导出操作（审计自身）
    {
        use crate::models::audit_log::{OperationType, Severity};
        use crate::services::audit_log_service::{AuditEvent, AuditLogService};
        use std::sync::Arc;
        let svc = AuditLogService::new(state.db.clone());
        let event = AuditEvent {
            tenant_id: Some(tenant_id),
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

    // 构造 CSV（按列顺序：id/created_at/user_id/username/operation_type/severity/
    // resource_type/resource_id/description/ip_address/request_id）
    let mut csv = String::from(
        "id,created_at,user_id,username,operation_type,severity,resource_type,\
         resource_id,description,ip_address,request_id\n",
    );
    for log in logs {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{}\n",
            log.id,
            log.created_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_default()
                .replace(',', " "),
            log.user_id
                .map(|i| i.to_string())
                .unwrap_or_default()
                .replace(',', " "),
            log.username.unwrap_or_default().replace(',', " "),
            log.operation_type.unwrap_or_default().replace(',', " "),
            log.severity.unwrap_or_default().replace(',', " "),
            log.resource_type.unwrap_or_default().replace(',', " "),
            log.resource_id.unwrap_or_default().replace(',', " "),
            log.description.unwrap_or_default().replace(',', " "),
            log.ip_address.unwrap_or_default().replace(',', " "),
            log.request_id.unwrap_or_default().replace(',', " "),
        ));
    }

    let filename = format!(
        "audit_logs_{}.csv",
        chrono::Utc::now().format("%Y%m%d%H%M%S")
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename))
            .map_err(|e| AppError::internal(format!("构建下载头失败: {}", e)))?,
    );

    Ok((StatusCode::OK, headers, csv))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::auth_context::AuthContext;

    /// 提取租户 ID 在有效值时返回 Ok
    #[test]
    fn test_extract_tenant_id_accepts_valid() {
        let auth = AuthContext {
            user_id: 1,
            username: "tester".to_string(),
            role_id: Some(1),
            tenant_id: Some(42),
        };
        let tid = extract_tenant_id(&auth).expect("租户 ID 应存在");
        assert_eq!(tid, 42);
    }

    /// 提取租户 ID 在 None 时返回 AppError（强隔离）
    #[test]
    fn test_extract_tenant_id_rejects_missing() {
        let auth = AuthContext {
            user_id: 1,
            username: "tester".to_string(),
            role_id: Some(1),
            tenant_id: None,
        };
        let err = extract_tenant_id(&auth).expect_err("缺失租户应失败");
        let msg = format!("{:?}", err);
        assert!(
            msg.contains("未授权") || msg.contains("Unauthorized") || msg.contains("租户"),
            "错误信息应提示租户相关问题：{}",
            msg
        );
    }

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
