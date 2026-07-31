//! 审计日志查询 Handler（P13 批 1 P3-2 增强版）
//!
//! 提供：
//! - GET    /api/v1/erp/audit-logs          分页 + 多维筛选
//! - GET    /api/v1/erp/audit-logs/{id}      详情
//! - GET    /api/v1/erp/audit-logs/export    xlsx 导出

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::{audit_log, audit_log_export_log};
use crate::utils::admin_checker::can_access_audit_logs;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use crate::utils::sql_escape::safe_like_pattern;
// V15 P0-S15 修复（Batch 475a）：导出注入水印（操作员/导出时间/导出条数）
use crate::utils::xlsx_export::{build_xlsx_with_watermark, xlsx_response, WatermarkConfig, XlsxTable};

/// V15 P1-14.2-C：审计日志查询要求 admin 或 auditor 角色；安全原因：审计日志含全系统操作记录（含其他用户敏感操作）， 仅依赖全局 permission_middleware 的 RBAC 不够（管理员可能误配
/// audit-logs:read 权限）， 在 handler 层增加角色深度防御，确保合规要求。 admin 不再持有 audit:read 权限码（职责分离），但保留运维排查能力； auditor 角色专门负责审计职责，独占 audit:read 权限码。
async fn require_admin_role(state: &AppState, auth: &AuthContext) -> Result<(), AppError> {
    let role_id = auth
        .role_id
        .ok_or_else(|| AppError::permission_denied("用户未分配角色，无法查询审计日志"))?;
    if !can_access_audit_logs(&state.db, role_id).await {
        tracing::warn!(
            target: "security_audit",
            event = "AUTHORIZATION_DENIED",
            user_id = auth.user_id,
            username = %auth.username,
            "[SECURITY] 非 admin/auditor 用户尝试查询审计日志被拒绝"
        );
        return Err(AppError::permission_denied(
            "查询审计日志仅限管理员（code=admin）或审计员（code=auditor）执行",
        ));
    }
    Ok(())
}

/// 列表查询参数（全部可选）
#[derive(Debug, Default, Deserialize, Serialize)]
// P1-13 修复（2026-06-25）：路由已挂载至 system::routes()，函数标记已移除。
// 结构体字段经 serde Deserialize 派生使用，标记保留待编译器验证后清理。
// V15 缺陷 10-4：追加 Serialize 派生，用于导出时序列化筛选条件写入 audit_log_export_log 防篡改表
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

/// GET /api/v1/erp/audit-logs；分页 + 多维筛选（时间范围 / user_id / operation_type / severity / resource_type / request_id）
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

/// 构建审计日志查询过滤条件（start_time/end_time/user_id/operation_type/severity/
/// resource_type/request_id）
fn build_audit_log_condition(query: &AuditLogListQuery) -> sea_orm::Condition {
    let mut cond = sea_orm::Condition::all();
    if let Some(start) = &query.start_time {
        if let Ok(ts) = start.parse::<DateTime<Utc>>() {
            cond = cond.add(audit_log::Column::CreatedAt.gte(ts.naive_utc()));
        }
    }
    if let Some(end) = &query.end_time {
        if let Ok(ts) = end.parse::<DateTime<Utc>>() {
            cond = cond.add(audit_log::Column::CreatedAt.lte(ts.naive_utc()));
        }
    }
    if let Some(uid) = query.user_id {
        cond = cond.add(audit_log::Column::UserId.eq(uid));
    }
    if let Some(op) = &query.operation_type {
        cond = cond.add(audit_log::Column::OperationType.eq(op.clone()));
    }
    if let Some(sev) = &query.severity {
        cond = cond.add(audit_log::Column::Severity.eq(sev.clone()));
    }
    if let Some(rt) = &query.resource_type {
        cond = cond.add(audit_log::Column::ResourceType.eq(rt.clone()));
    }
    if let Some(rid) = &query.request_id {
        cond = cond.add(audit_log::Column::RequestId.eq(rid.clone()));
    }
    cond
}

/// 审计日志导出表头（11 列）
fn audit_log_export_headers() -> Vec<String> {
    vec![
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
    ]
}

/// 从单条审计日志 model 构建 xlsx 行
fn build_audit_log_row(log: audit_log::Model) -> Vec<String> {
    vec![
        log.id.to_string(),
        log.created_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
        log.user_id.map(|i| i.to_string()).unwrap_or_default(),
        log.username.unwrap_or_default(),
        log.operation_type.unwrap_or_default(),
        log.severity.unwrap_or_default(),
        log.resource_type.unwrap_or_default(),
        log.resource_id.unwrap_or_default(),
        log.description.unwrap_or_default(),
        log.ip_address.unwrap_or_default(),
        log.request_id.unwrap_or_default(),
    ]
}

/// 构造审计日志 xlsx 表格
fn build_audit_logs_table(logs: Vec<audit_log::Model>) -> XlsxTable {
    XlsxTable {
        sheet_name: "审计日志".to_string(),
        headers: audit_log_export_headers(),
        rows: logs.into_iter().map(build_audit_log_row).collect(),
    }
}

/// 异步记录审计日志导出操作（审计自身）
fn record_audit_logs_export_audit(state: &AppState, auth: &AuthContext, logs_count: usize) {
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
        description: Some(format!("导出 {} 条审计日志", logs_count)),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/audit-logs/export".to_string()),
        before_snapshot: None,
        after_snapshot: None,
    };
    Arc::new(svc).record_async(event, None);
}

/// GET /api/v1/erp/audit-logs/export；返回 xlsx 格式（Excel），前端直接 `window.URL.createObjectURL(blob)` 下载。
///
/// V15 缺陷 10-4 修复：导出操作同时写入独立防篡改表 `audit_log_export_log`，
/// 该表通过数据库触发器禁止 UPDATE / DELETE，审计员无法篡改自身导出记录。
pub async fn export_audit_logs(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<AuditLogListQuery>,
    headers: HeaderMap,
) -> Result<axum::response::Response, AppError> {
    // P0 8-5 修复：审计日志导出仅限 admin
    require_admin_role(&state, &auth).await?;

    let cond = build_audit_log_condition(&query);
    let logs = audit_log::Entity::find()
        .filter(cond)
        .order_by_desc(audit_log::Column::CreatedAt)
        .all(state.db.as_ref())
        .await
        .map_err(|e| AppError::internal(format!("查询审计日志失败: {}", e)))?;

    // V15 P0-S15 修复（Batch 475a）：保存 logs 数量用于水印（logs 后续被 into_iter 消费）
    let logs_count = logs.len();
    record_audit_logs_export_audit(&state, &auth, logs_count);

    let table = build_audit_logs_table(logs);
    let filename = format!("audit_logs_{}", chrono::Utc::now().format("%Y%m%d%H%M%S"));

    // V15 P0-S15 修复（Batch 475a）：注入水印（操作员/导出时间/导出条数）
    let watermark = WatermarkConfig {
        operator: Some(auth.username.clone()),
        ip_address: None,
        exported_at: Some(chrono::Utc::now().to_rfc3339()),
        extra: Some(format!(
            "审计日志导出（共 {} 条，仅 admin 可导出）",
            logs_count
        )),
    };

    // V15 缺陷 10-4：先构建 xlsx 字节，计算 SHA256 指纹后写入防篡改表
    let xlsx_bytes = build_xlsx_with_watermark(&table, &watermark)?;
    let file_hash = hex_sha256(&xlsx_bytes);
    let file_size = xlsx_bytes.len() as i64;

    // 写入防篡改表（独立于 audit_logs，审计员无法改/删）
    record_audit_log_export_tamper_proof(
        &state,
        &auth,
        &query,
        logs_count,
        &file_hash,
        file_size,
        &headers,
    )
    .await;

    // 规则 3：导出统一使用 xlsx 格式，错误用 AppError 表达，成功返回 200 + xlsx 响应体
    Ok(xlsx_response(xlsx_bytes, &filename))
}

/// 计算 SHA256 指纹并返回小写十六进制字符串
fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// 从 HeaderMap 提取首个 header 值（IP / User-Agent / X-Request-Id）
fn header_str(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// V15 缺陷 10-4：写入防篡改表 audit_log_export_log（独立于 audit_logs）。
/// 该表通过 BEFORE UPDATE / DELETE 触发器禁止修改，审计员无法篡改自身导出记录。
/// best-effort：写入失败仅记录日志，不阻塞导出响应（导出本身已成功）。
async fn record_audit_log_export_tamper_proof(
    state: &AppState,
    auth: &AuthContext,
    query: &AuditLogListQuery,
    record_count: usize,
    file_hash: &str,
    file_size: i64,
    headers: &HeaderMap,
) {
    let query_filter = serde_json::to_string(query).ok();
    let ip = header_str(headers, "x-forwarded-for")
        .or_else(|| header_str(headers, "x-real-ip"))
        .or_else(|| header_str(headers, "true-client-ip"));
    let user_agent = header_str(headers, "user-agent");
    let request_id = header_str(headers, "x-request-id");

    let export_log = audit_log_export_log::ActiveModel {
        exporter_user_id: Set(auth.user_id),
        exporter_username: Set(auth.username.clone()),
        export_query_filter: Set(query_filter),
        export_record_count: Set(record_count as i32),
        export_file_format: Set("xlsx".to_string()),
        export_file_hash_sha256: Set(Some(file_hash.to_string())),
        export_file_size_bytes: Set(Some(file_size)),
        export_ip_address: Set(ip),
        export_user_agent: Set(user_agent),
        export_request_id: Set(request_id),
        exported_at: Set(chrono::Utc::now()),
        ..Default::default()
    };

    if let Err(e) = export_log.insert(state.db.as_ref()).await {
        tracing::error!(
            target: "security_audit",
            error = %e,
            user_id = auth.user_id,
            "[SECURITY] 写入 audit_log_export_log 防篡改表失败（导出已成功，但二次审计记录缺失）"
        );
    }
}

/// V15 缺陷 10-4：查询审计日志导出二次审计记录（仅 admin/auditor）。
/// GET /api/v1/erp/audit-logs/export-logs
#[derive(Debug, Deserialize)]
pub struct ExportLogListQuery {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
    pub exporter_user_id: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct ExportLogListItem {
    pub id: i32,
    pub exporter_user_id: i32,
    pub exporter_username: String,
    pub export_record_count: i32,
    pub export_file_format: String,
    pub export_file_hash_sha256: Option<String>,
    pub export_file_size_bytes: Option<i64>,
    pub export_ip_address: Option<String>,
    pub export_request_id: Option<String>,
    pub exported_at: String,
}

#[derive(Debug, Serialize)]
pub struct ExportLogListResponse {
    pub items: Vec<ExportLogListItem>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

pub async fn list_audit_log_export_logs(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<ExportLogListQuery>,
) -> Result<Json<ApiResponse<ExportLogListResponse>>, AppError> {
    require_admin_role(&state, &auth).await?;

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);

    let mut select = audit_log_export_log::Entity::find()
        .order_by_desc(audit_log_export_log::Column::ExportedAt);
    if let Some(uid) = query.exporter_user_id {
        select = select.filter(audit_log_export_log::Column::ExporterUserId.eq(uid));
    }

    let total = select
        .clone()
        .count(state.db.as_ref())
        .await
        .map_err(|e| AppError::internal(format!("查询导出审计记录总数失败: {}", e)))?;

    let rows = select
        .paginate(state.db.as_ref(), per_page)
        .fetch_page(page - 1)
        .await
        .map_err(|e| AppError::internal(format!("查询导出审计记录失败: {}", e)))?;

    let items = rows
        .into_iter()
        .map(|m| ExportLogListItem {
            id: m.id,
            exporter_user_id: m.exporter_user_id,
            exporter_username: m.exporter_username,
            export_record_count: m.export_record_count,
            export_file_format: m.export_file_format,
            export_file_hash_sha256: m.export_file_hash_sha256,
            export_file_size_bytes: m.export_file_size_bytes,
            export_ip_address: m.export_ip_address,
            export_request_id: m.export_request_id,
            exported_at: m.exported_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(ApiResponse::success(ExportLogListResponse {
        items,
        total,
        page,
        page_size: per_page,
    })))
}

/// V15 P1-5-3：前端打印审计埋点请求体；前端 `printData`/`printSingleDocument` 纯前端 window.print 不经过后端 handler， 无法触发
/// omni_audit 中间件落库。此端点供前端打印完成后 best-effort 上报， 后端写入 audit_logs（OperationType::Print），确保合规审计覆盖前端打印操作。
#[derive(Debug, Deserialize)]
pub struct RecordPrintEventRequest {
    /// 资源类型（如 customer / supplier / warehouse，与权限码 resource_type 对应）
    pub resource_type: String,
    /// 打印记录数（data.length）
    pub record_count: i32,
    /// 打印文档标题（PrintOptions.title）
    pub title: String,
    /// V15 P1-5-3：资源 ID（可选，单据打印时传入）
    pub resource_id: Option<String>,
}

/// V15 P1-5-3：前端打印审计埋点端点；POST /api/v1/erp/audit-logs/record-print；安全设计： - 必须认证（AuthContext 由全局 auth_middleware 注入） -
/// 字段长度校验（resource_type ≤ 64，title ≤ 200，record_count ≥ 0） - best-effort 异步落库，不阻塞响应 - 不要求 admin/auditor 角色（任何已认证用户打印均需审计）
pub async fn record_print_event(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<RecordPrintEventRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // 字段长度校验
    if req.resource_type.is_empty() || req.resource_type.len() > 64 {
        return Err(AppError::validation("resource_type 长度必须在 1-64 之间"));
    }
    if req.title.is_empty() || req.title.len() > 200 {
        return Err(AppError::validation("title 长度必须在 1-200 之间"));
    }
    if req.record_count < 0 {
        return Err(AppError::validation("record_count 不能为负数"));
    }
    if let Some(ref rid) = req.resource_id {
        if rid.len() > 64 {
            return Err(AppError::validation("resource_id 长度不能超过 64"));
        }
    }

    use crate::models::audit_log::{OperationType, Severity};
    use crate::services::audit_log_service::{AuditEvent, AuditLogService};
    use std::sync::Arc;

    let description = format!(
        "用户 {} 前端打印 {}（共 {} 条记录）",
        auth.username, req.title, req.record_count
    );
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Print,
        severity: Severity::Info,
        resource_type: Some(req.resource_type.clone()),
        resource_id: req.resource_id.clone(),
        resource_name: Some(req.title.clone()),
        description: Some(description),
        request_method: Some("POST".to_string()),
        request_path: Some("/api/v1/erp/audit-logs/record-print".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "resource_type": req.resource_type,
            "record_count": req.record_count,
            "title": req.title,
            "source": "frontend_print",
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);

    Ok(Json(ApiResponse::success(())))
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

    /// V15 缺陷 10-4：hex_sha256 对空输入返回已知常量值
    #[test]
    fn test_hex_sha256_empty() {
        let hash = hex_sha256(b"");
        assert_eq!(hash.len(), 64);
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    /// V15 缺陷 10-4：hex_sha256 对相同输入产生相同指纹（确定性）
    #[test]
    fn test_hex_sha256_deterministic() {
        let a = hex_sha256(b"audit-log-export-test");
        let b = hex_sha256(b"audit-log-export-test");
        assert_eq!(a, b);
        assert_ne!(a, hex_sha256(b"audit-log-export-test-2"));
    }

    /// V15 缺陷 10-4：header_str 从 HeaderMap 提取首个值
    #[test]
    fn test_header_str_extract() {
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", "abc-123".parse().unwrap());
        assert_eq!(header_str(&headers, "x-request-id"), Some("abc-123".to_string()));
        assert_eq!(header_str(&headers, "user-agent"), None);
    }

    /// V15 缺陷 10-4：ExportLogListQuery 默认分页参数为 None
    #[test]
    fn test_export_log_list_query_default() {
        let q = ExportLogListQuery {
            page: None,
            per_page: None,
            exporter_user_id: None,
        };
        assert!(q.page.is_none());
        assert!(q.per_page.is_none());
        assert!(q.exporter_user_id.is_none());
    }
}
