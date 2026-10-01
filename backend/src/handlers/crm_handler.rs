use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::audit_log::{OperationType, Severity};
use crate::models::dto::crm_dto::{
    CloseAsLostRequest, ConvertLeadRequest, CreateLeadRequest, CreateOpportunityRequest,
    FollowUpRequest, ImportLeadsResult, LeadQuery, OpportunityQuery, UpdateLeadRequest,
    UpdateOpportunityRequest,
};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::crm::cust::CrmService;
// #207：导出需复用列表同一判定源/同一字段过滤函数（filter_fields_batch）
use crate::services::data_permission_service::DataPermissionService;
use crate::utils::error::AppError;
use crate::utils::export_concurrency::ExportConcurrencyGuard;
use crate::utils::messages::biz_msg;
use crate::utils::response::ApiResponse;
use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
};
use chrono::Datelike;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use validator::Validate;

/// P1-2g 修复（批次 81 v1 复审）：更新线索状态请求 DTO
/// 替代 update_lead_status 中的 Json<serde_json::Value>，提供强类型校验
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateLeadStatusDto {
    /// 状态：必填，长度至少 1
    #[validate(length(min = 1, max = 30, message = "状态长度必须在1到30字符之间"))]
    pub status: String,
}

pub async fn create_lead(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateLeadRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let res = service.create_lead(req, auth.user_id).await?;
    let value = serde_json::to_value(res)?;
    Ok(Json(ApiResponse::success(value)))
}

pub async fn list_leads(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<LeadQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文
    let data_scope_ctx = auth.to_data_scope_context();
    let res = service.list_leads(query, Some(&data_scope_ctx)).await?;
    let mut value = serde_json::to_value(res)?;

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        if let Ok(Some(permission)) = state
            .data_permission_service
            .get_role_data_permission(role_id, "crm_lead")
            .await
        {
            let mut list_opt = value.get_mut("list");
            if list_opt.is_none() {
                list_opt = value.get_mut("data");
            }
            if let Some(list) = list_opt.and_then(|v| v.as_array_mut()) {
                state.data_permission_service.filter_fields_batch(
                    list,
                    &permission.allowed_fields,
                    &permission.hidden_fields,
                );
            }
        } else if role_id != 1 {
            // P1-08-5 修复：默认字段脱敏（保留前 3 后 4 / 首字母 + ***），而非直接 remove
            // 原 remove 导致业务无法识别客户（如需回拨电话），实际使用中可能被绕过。
            // 脱敏后业务仍可识别客户身份，同时满足个人信息保护法最小必要原则。
            let mut list_opt = value.get_mut("list");
            if list_opt.is_none() {
                list_opt = value.get_mut("data");
            }
            if let Some(list) = list_opt.and_then(|v| v.as_array_mut()) {
                for lead in list {
                    if let Some(obj) = lead.as_object_mut() {
                        // 键名必须取 crm_lead 真实列 mobile_phone（models/crm_lead.rs:40）：
                        // 出参由 serde_json::to_value(Vec<crm_lead::Model>) 生成，
                        // 不存在 contact_phone 键（该键属 customer/sales_order/supplier 等模型），
                        // 读错键使本块对手机号恒不生效。
                        if let Some(phone) = obj.get("mobile_phone").and_then(|v| v.as_str()) {
                            obj.insert(
                                "mobile_phone".to_string(),
                                Value::String(crate::utils::field_mask::mask_phone(phone)),
                            );
                        }
                        if let Some(email) = obj.get("email").and_then(|v| v.as_str()) {
                            obj.insert(
                                "email".to_string(),
                                Value::String(crate::utils::field_mask::mask_email(email)),
                            );
                        }
                        obj.remove("address");
                    }
                }
            }
        }
    }

    Ok(Json(ApiResponse::success(value)))
}

/// GET /api/v1/erp/crm/leads/export - 导出线索为 xlsx；v11 批次 141 新增：前端 exportLeads API 真实接入。 v11 批次 142 升级
/// 导出格式从 CSV 升级为 xlsx（规则 3 强制要求）。 返回 application/vnd.openxmlformats-officedocument.spreadsheetml.sheet
///
/// 数据权限（#207）：导出必须与本文件的 list_leads/get_lead 走同一条权限链——
/// 行级：`auth.to_data_scope_context()` 注入 service，套用同一个
/// `apply_department_scope_with_pool`（services/crm/lead.rs），使导出的行集合与
/// 该用户列表可见集严格一致；字段级：同一个判定源
/// （`data_permission_service.get_role_data_permission`，admin 依据 roles.code='admin'）
/// 与同一掩码实现（`utils/field_mask::mask_phone/mask_email`）。
/// 省略任一层都会形成"列表打码、导出原文"的旁路，故两层均为强制，不按查询参数开关。
pub async fn export_leads(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<LeadQuery>,
) -> Result<axum::response::Response, AppError> {
    // V15 P1-9-1：全局导出并发控制（RAII 守卫，函数退出自动递减）
    let _guard = ExportConcurrencyGuard::acquire()?;

    let service = CrmService::new(state.db.clone());
    // 行级数据权限：与 list_leads（本文件 :53-54）同法构造并注入 ctx；
    // 省略 ctx 会让 service 层整体跳过行级过滤，self/dept 用户可一次拿到全库线索。
    let data_scope_ctx = auth.to_data_scope_context();
    let mut table = service.export_leads(query, Some(&data_scope_ctx)).await?;

    // 字段级数据权限：见上面 doc 注释的判定源；导出为二维表，按
    // `CrmService::EXPORT_LEAD_COLUMNS` 的列名（= crm_lead 出参键）定位需保留/剔除/
    // 掩码的列，不新造权限键、不重复列下标。
    match auth.role_id {
        Some(role_id) => {
            if let Ok(Some(permission)) = state
                .data_permission_service
                .get_role_data_permission(role_id, "crm_lead")
                .await
            {
                // 配置了数据权限行：与列表分支同一函数 filter_fields_batch
                // （allowed_fields 白名单保留、hidden_fields 移除，不叠加默认打码）
                apply_export_field_permission(
                    &state.data_permission_service,
                    &mut table,
                    &permission.allowed_fields,
                    &permission.hidden_fields,
                );
            } else if role_id != 1 {
                // 无权限行且非 admin（查询 Err 亦走此分支，fail-closed）：默认掩码
                mask_export_pii_columns(&mut table)?;
            }
        }
        // fail-closed：role_id 缺失（权限中间件正常已在更早处 403，此处仅防"无角色=原文放行"
        // 的旁路）——按非 admin 处理走默认掩码，而不是像列表那样不处理。
        None => mask_export_pii_columns(&mut table)?,
    }

    let row_count = table.rows.len();

    // V15 P0-S11：导出审计日志写入（best-effort，异步不阻塞响应）
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("crm_lead".to_string()),
        resource_id: None,
        resource_name: Some("crm_leads_export.xlsx".to_string()),
        description: Some(format!(
            "用户 {} 导出 CRM 线索（共 {} 条）",
            auth.username, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/crm/leads/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": row_count,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);

    crate::utils::xlsx_export::build_xlsx_response(&table, "crm_leads_export")
}

/// #207：把导出表的某一列按 `CrmService::EXPORT_LEAD_COLUMNS` 的列名读出来（供
/// 复用列表同一判定函数 filter_fields 时构造行对象）。
/// 行长度与列定义不一致属编程错误：记录 error 后按空值继续，绝不静默跳过整列
/// （跳过 = 该列原文直通）。
fn export_row_cells(table: &crate::utils::xlsx_export::XlsxTable) -> Vec<serde_json::Value> {
    table
        .rows
        .iter()
        .map(|row| {
            let mut obj = serde_json::Map::new();
            for (idx, (field, _)) in CrmService::EXPORT_LEAD_COLUMNS.iter().enumerate() {
                let cell = match row.get(idx) {
                    Some(v) => v.clone(),
                    None => {
                        tracing::error!(
                            field = %field,
                            row_len = row.len(),
                            "导出行长度与 EXPORT_LEAD_COLUMNS 列数不一致，该列按空值处理"
                        );
                        String::new()
                    }
                };
                obj.insert((*field).to_string(), serde_json::Value::String(cell));
            }
            serde_json::Value::Object(obj)
        })
        .collect()
}

/// #207：导出文件的字段级数据权限（有配置数据权限行的分支）。
///
/// 复用列表/详情完全相同的判定实现 `filter_fields_batch`（allowed_fields 为白名单：
/// 未列入的列取值被剔除；hidden_fields 再移除），因此"配了 allowed_fields 的角色"
/// 天然就是放行原文的受控通道，无需为导出新增权限键。被剔除的列在 xlsx 中落成
/// 空单元格（表头保留，避免同一角色不同入口的列结构漂移）。
fn apply_export_field_permission(
    dsp: &DataPermissionService,
    table: &mut crate::utils::xlsx_export::XlsxTable,
    allowed_fields: &Option<Vec<String>>,
    hidden_fields: &Option<Vec<String>>,
) {
    let mut rows_json = export_row_cells(table);
    dsp.filter_fields_batch(&mut rows_json, allowed_fields, hidden_fields);
    for (row_idx, row) in table.rows.iter_mut().enumerate() {
        let Some(obj) = rows_json.get(row_idx).and_then(|v| v.as_object()) else {
            tracing::error!(row_idx, "导出行掩码回写时找不到对应行对象，跳过该行");
            continue;
        };
        for (col_idx, (field, _)) in CrmService::EXPORT_LEAD_COLUMNS.iter().enumerate() {
            let value = obj
                .get(*field)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            match row.get_mut(col_idx) {
                Some(slot) => *slot = value,
                None => row.push(value),
            }
        }
    }
}

/// #207：导出文件的默认脱敏分支（无数据权限行且非 admin，与 list_leads/get_lead
/// 的 P1-08-5 分支同函数同参数：`field_mask::mask_phone` / `mask_email`）。
/// 空单元格不掩码（否则空值会变成 `*`，与列表"无值"表现不一致）。
/// 列名在导出定义中找不到时返回错误 fail-closed：宁可不出文件，不放行原文。
#[derive(Clone, Copy)]
enum ExportPiiKind {
    Phone,
    Email,
}

fn mask_export_pii_columns(
    table: &mut crate::utils::xlsx_export::XlsxTable,
) -> Result<(), AppError> {
    let mut targets: Vec<(usize, ExportPiiKind)> = Vec::new();
    for (fields, kind) in [
        (CrmService::EXPORT_PII_PHONE_COLUMNS, ExportPiiKind::Phone),
        (CrmService::EXPORT_PII_EMAIL_COLUMNS, ExportPiiKind::Email),
    ] {
        for field in fields {
            let col = CrmService::export_column_index(*field).ok_or_else(|| {
                AppError::internal(format!("导出列定义缺少 PII 列 {field}，拒绝导出原文"))
            })?;
            targets.push((col, kind));
        }
    }

    for row in table.rows.iter_mut() {
        for (col_idx, kind) in &targets {
            let Some(cell) = row.get_mut(*col_idx) else {
                tracing::error!(
                    col_idx = %col_idx,
                    row_len = row.len(),
                    "导出行长度不足，PII 列未参与掩码（列定义与行构造不一致，需修服务层）"
                );
                continue;
            };
            if cell.is_empty() {
                continue;
            }
            let masked = match kind {
                ExportPiiKind::Phone => crate::utils::field_mask::mask_phone(cell.as_str()),
                ExportPiiKind::Email => crate::utils::field_mask::mask_email(cell.as_str()),
            };
            *cell = masked;
        }
    }
    Ok(())
}

/// POST /api/v1/erp/crm/leads/import - 批量导入线索（xlsx）；v11 批次 157d-4 新增：接收
/// Multipart xlsx 文件，后端用 calamine 解析并批量创建线索。 文件大小限制 10MB，列顺序与 export_leads 一致。
pub async fn import_leads(
    State(state): State<AppState>,
    auth: AuthContext,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<ImportLeadsResult>>, AppError> {
    const MAX_IMPORT_SIZE: usize = 10 * 1024 * 1024;

    let mut file_bytes: Option<Vec<u8>> = None;

    if let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let file_name = field.file_name().unwrap_or("").to_string();
        // 仅接受 .xlsx 文件
        if !file_name.ends_with(".xlsx") {
            return Err(AppError::bad_request("仅支持 .xlsx 格式文件".to_string()));
        }
        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::bad_request(format!("文件上传失败：{}", e)))?;
        if data.len() > MAX_IMPORT_SIZE {
            return Err(AppError::bad_request(format!(
                "文件大小超过限制 ({}MB)",
                MAX_IMPORT_SIZE / 1024 / 1024
            )));
        }
        // P1-03-5 修复：增加 xlsx magic bytes 校验
        // xlsx 本质为 ZIP 文件，前 4 字节应为 50 4B 03 04。
        // 后缀校验可被绕过（如 .xlsx 实为可执行脚本），magic 校验防 zip 炸弹/XXE/恶意文件。
        if !verify_xlsx_magic(&data) {
            return Err(AppError::bad_request(
                "文件内容不是有效的 xlsx 格式（magic bytes 校验失败）".to_string(),
            ));
        }
        file_bytes = Some(data.to_vec());
    }

    let bytes = file_bytes.ok_or_else(|| AppError::bad_request("未收到文件".to_string()))?;
    // B03-P2-10 修复：xlsx 文件病毒扫描检查点（CLAMAV_ENABLED 控制开关，生产环境应启用）
    scan_leads_for_viruses(&bytes).await?;
    let service = CrmService::new(state.db.clone());
    let result = service.import_leads(bytes, auth.user_id).await?;
    Ok(Json(ApiResponse::success(result)))
}

/// P1-03-5 新增：校验 xlsx 文件 magic bytes
/// xlsx 是 OOXML 格式（实际为 ZIP），前 4 字节应为 50 4B 03 04（PK\x03\x04）。
fn verify_xlsx_magic(data: &[u8]) -> bool {
    data.starts_with(&[0x50, 0x4B, 0x03, 0x04])
}

/// B03-P2-10 修复：CRM 线索导入文件病毒扫描检查点
/// 通过 CLAMAV_ENABLED 环境变量控制开关：启用时调用 ClamAV REST API（CLAMAV_URL）扫描，
/// 未启用时记录 warn 日志并跳过。生产环境应设置 CLAMAV_ENABLED=true 并配置 CLAMAV_URL，
/// 与 email_service.rs 的附件扫描保持一致的 HTTP API 集成模式。
async fn scan_leads_for_viruses(data: &[u8]) -> Result<(), AppError> {
    let enabled = std::env::var("CLAMAV_ENABLED")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    if !enabled {
        tracing::warn!(
            target: "security_audit",
            event = "CLAMAV_SCAN_SKIPPED",
            "CLAMAV_ENABLED 未启用，CRM 线索导入跳过病毒扫描；生产环境应设置 CLAMAV_ENABLED=true 并配置 CLAMAV_URL"
        );
        return Ok(());
    }

    // 决策定案 #4：扫描依赖故障族（未配置/不可达/非 2xx/响应读取失败）不是
    // 「我方服务器坏了」（500 InternalError），而是外部扫描依赖不可用——统一走
    // AppError::service_unavailable（HTTP 503 / code=SERVICE_UNAVAILABLE / 公网脱敏文案）。
    // 真实原因只进 tracing::warn（CLAMAV_SCAN_UNAVAILABLE 事件标签），不外泄 URL/端口/配置键名；
    // fail-closed 不动摇：以下每一条故障路径都在导入之前 return Err，文件不落盘/不落库。
    let clamav_url = std::env::var("CLAMAV_URL").unwrap_or_default();
    if clamav_url.trim().is_empty() {
        tracing::warn!(
            target: "security_audit",
            event = "CLAMAV_SCAN_UNAVAILABLE",
            "CLAMAV_ENABLED 已启用但扫描服务地址未配置或为空，CRM 线索导入已拒绝（fail-closed，未进入导入）"
        );
        return Err(AppError::service_unavailable(
            "病毒扫描依赖不可用：扫描服务地址未配置（CRM 线索导入），根因见 CLAMAV_SCAN_UNAVAILABLE 事件日志",
        ));
    }

    let client = reqwest::Client::new();
    let scan_url = format!("{}/scan", clamav_url.trim_end_matches('/'));
    // V15 P1 20.1-A：注入 traceparent 到 ClamAV 出站请求
    let traceparent = crate::observability::trace_context::traceparent_from_current_span();
    let response = client
        .post(&scan_url)
        .header("Content-Type", "application/octet-stream")
        .header("traceparent", traceparent)
        .body(data.to_vec())
        .send()
        .await
        .map_err(|e| {
            tracing::warn!(
                target: "security_audit",
                event = "CLAMAV_SCAN_UNAVAILABLE",
                error = %e,
                "病毒扫描服务不可达（连接失败/超时），CRM 线索导入已拒绝（fail-closed，未进入导入）"
            );
            AppError::service_unavailable(
                "病毒扫描依赖不可用：扫描服务不可达（CRM 线索导入），根因见 CLAMAV_SCAN_UNAVAILABLE 事件日志",
            )
        })?;

    if !response.status().is_success() {
        tracing::warn!(
            target: "security_audit",
            event = "CLAMAV_SCAN_UNAVAILABLE",
            upstream_status = %response.status(),
            "病毒扫描服务返回非 2xx，CRM 线索导入已拒绝（fail-closed，未进入导入）"
        );
        return Err(AppError::service_unavailable(
            "病毒扫描依赖不可用：扫描服务返回非 2xx（CRM 线索导入），根因见 CLAMAV_SCAN_UNAVAILABLE 事件日志",
        ));
    }

    let body = response.text().await.map_err(|e| {
        tracing::warn!(
            target: "security_audit",
            event = "CLAMAV_SCAN_UNAVAILABLE",
            error = %e,
            "读取病毒扫描响应失败，CRM 线索导入已拒绝（fail-closed，未进入导入）"
        );
        AppError::service_unavailable(
            "病毒扫描依赖不可用：扫描响应读取失败（CRM 线索导入），根因见 CLAMAV_SCAN_UNAVAILABLE 事件日志",
        )
    })?;

    // ClamAV REST 返回 "stream: OK" 表示无病毒
    if body.contains("OK") {
        Ok(())
    } else {
        Err(AppError::bad_request(format!(
            "CRM 线索导入文件被 ClamAV 识别为病毒: {}",
            body
        )))
    }
}

pub async fn get_lead(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();
    let res = service.get_lead(id, Some(&data_scope_ctx)).await?;
    let mut value = serde_json::to_value(res)?;

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        if let Ok(Some(permission)) = state
            .data_permission_service
            .get_role_data_permission(role_id, "crm_lead")
            .await
        {
            state.data_permission_service.filter_fields(
                &mut value,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
        } else if role_id != 1 {
            // P1-08-5 修复：详情接口脱敏而非 remove
            if let Some(obj) = value.as_object_mut() {
                // 同列表分支：真实列名是 mobile_phone（models/crm_lead.rs:40），
                // 出参为 crm_lead::Model 序列化，无 contact_phone 键。
                if let Some(phone) = obj.get("mobile_phone").and_then(|v| v.as_str()) {
                    obj.insert(
                        "mobile_phone".to_string(),
                        Value::String(crate::utils::field_mask::mask_phone(phone)),
                    );
                }
                if let Some(email) = obj.get("email").and_then(|v| v.as_str()) {
                    obj.insert(
                        "email".to_string(),
                        Value::String(crate::utils::field_mask::mask_email(email)),
                    );
                }
                obj.remove("address");
            }
        }
    }

    Ok(Json(ApiResponse::success(value)))
}

pub async fn update_lead(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateLeadRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——更新前先校验资源归属（复用 P0-S01 的 get_lead + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_lead(id, Some(&data_scope_ctx)).await?;
    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    let res = service.update_lead(id, req, auth.user_id).await?;
    let value = serde_json::to_value(res)?;
    Ok(Json(ApiResponse::success(value)))
}

pub async fn delete_lead(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let service = CrmService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——删除前先校验资源归属（复用 P0-S01 的 get_lead + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_lead(id, Some(&data_scope_ctx)).await?;
    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    service.delete_lead(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(biz_msg::DELETE_OK.to_string())))
}

pub async fn update_lead_status(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateLeadStatusDto>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    // P1-2g 修复（批次 81 v1 复审）：强类型 DTO + validator 替代 Json<Value>
    payload.validate().map_err(AppError::from)?;

    let service = CrmService::new(state.db.clone());
    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    service
        .update_lead_status(id, &payload.status, auth.user_id)
        .await?;
    Ok(Json(ApiResponse::success("状态更新成功".to_string())))
}

pub async fn create_opportunity(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateOpportunityRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let res = service.create_opportunity(req, auth.user_id).await?;
    let value = serde_json::to_value(res)?;
    Ok(Json(ApiResponse::success(value)))
}

pub async fn list_opportunities(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<OpportunityQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文
    let data_scope_ctx = auth.to_data_scope_context();
    let res = service
        .list_opportunities(query, Some(&data_scope_ctx))
        .await?;
    let mut value = serde_json::to_value(res)?;

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        if let Ok(Some(permission)) = state
            .data_permission_service
            .get_role_data_permission(role_id, "crm_opportunity")
            .await
        {
            let mut list_opt = value.get_mut("list");
            if list_opt.is_none() {
                list_opt = value.get_mut("data");
            }
            if let Some(list) = list_opt.and_then(|v| v.as_array_mut()) {
                state.data_permission_service.filter_fields_batch(
                    list,
                    &permission.allowed_fields,
                    &permission.hidden_fields,
                );
            }
        } else if role_id != 1 {
            // 如果没有配置数据权限且不是管理员，使用默认字段隐藏
            let mut list_opt = value.get_mut("list");
            if list_opt.is_none() {
                list_opt = value.get_mut("data");
            }
            if let Some(list) = list_opt.and_then(|v| v.as_array_mut()) {
                for opportunity in list {
                    if let Some(obj) = opportunity.as_object_mut() {
                        obj.remove("amount");
                    }
                }
            }
        }
    }

    Ok(Json(ApiResponse::success(value)))
}

/// GET /api/v1/erp/crm/opportunities/export - 导出商机为 xlsx；v11 批次 141 新增：前端 exportOpportunities API 真实接入。 v11 批次
/// 142 升级：导出格式从 CSV 升级为 xlsx（规则 3 强制要求）。 返回 application/vnd.openxmlformats-officedocument.spreadsheetml.sheet
pub async fn export_opportunities(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<OpportunityQuery>,
) -> Result<axum::response::Response, AppError> {
    // V15 P1-9-1：全局导出并发控制（RAII 守卫，函数退出自动递减）
    let _guard = ExportConcurrencyGuard::acquire()?;

    let service = CrmService::new(state.db.clone());
    let table = service.export_opportunities(query).await?;
    let row_count = table.rows.len();

    // V15 P0-S11：导出审计日志写入（best-effort，异步不阻塞响应）
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("crm_opportunity".to_string()),
        resource_id: None,
        resource_name: Some("crm_opportunities_export.xlsx".to_string()),
        description: Some(format!(
            "用户 {} 导出 CRM 商机（共 {} 条）",
            auth.username, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/crm/opportunities/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": row_count,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);

    crate::utils::xlsx_export::build_xlsx_response(&table, "crm_opportunities_export")
}

pub async fn get_opportunity(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();
    let res = service.get_opportunity(id, Some(&data_scope_ctx)).await?;
    let mut value = serde_json::to_value(res)?;

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        if let Ok(Some(permission)) = state
            .data_permission_service
            .get_role_data_permission(role_id, "crm_opportunity")
            .await
        {
            state.data_permission_service.filter_fields(
                &mut value,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
        } else if role_id != 1 {
            // 如果没有配置数据权限且不是管理员，使用默认字段隐藏
            if let Some(obj) = value.as_object_mut() {
                obj.remove("amount");
            }
        }
    }

    Ok(Json(ApiResponse::success(value)))
}

pub async fn update_opportunity(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateOpportunityRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——更新前先校验资源归属（复用 P0-S01 的 get_opportunity + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_opportunity(id, Some(&data_scope_ctx)).await?;
    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    let res = service.update_opportunity(id, req, auth.user_id).await?;
    let value = serde_json::to_value(res)?;
    Ok(Json(ApiResponse::success(value)))
}

pub async fn delete_opportunity(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let service = CrmService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——删除前先校验资源归属（复用 P0-S01 的 get_opportunity + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_opportunity(id, Some(&data_scope_ctx)).await?;
    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    service.delete_opportunity(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(biz_msg::DELETE_OK.to_string())))
}

/// 将商机转化为销售订单
pub async fn convert_opportunity_to_order(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let order = service
        .convert_opportunity_to_order(id, auth.user_id)
        .await?;
    let value = serde_json::to_value(order)?;
    Ok(Json(ApiResponse::success(value)))
}

/// 关单（输单流程）— V15 P0-B09（Batch 482）；将商机状态置为 CLOSED_LOST，必须填写流失原因 设计依据：审计报告 §18.2-D2 — 输单原因未记录，销售改进无依据
pub async fn close_opportunity_as_lost(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<CloseAsLostRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护 — 关单操作前先校验资源归属
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_opportunity(id, Some(&data_scope_ctx)).await?;
    let res = service
        .close_as_lost(id, req.lost_reason, auth.user_id)
        .await?;
    let value = serde_json::to_value(res)?;
    Ok(Json(ApiResponse::success(value)))
}

/// Get lead relation info
pub async fn get_lead_relation(
    Path(lead_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let relation = service.get_lead_relation(lead_id).await?;
    let value = serde_json::to_value(relation)?;
    Ok(Json(ApiResponse::success(value)))
}

/// 转化线索为客户
pub async fn convert_lead(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<ConvertLeadRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let customer = service
        .convert_lead_to_customer(id, req, auth.user_id)
        .await?;
    let value = serde_json::to_value(customer)?;
    Ok(Json(ApiResponse::success(value)))
}

/// Get customer relation summary
pub async fn get_customer_relation_summary(
    Path(customer_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let summary = service.get_customer_relation_summary(customer_id).await?;
    let value = serde_json::to_value(summary)?;
    Ok(Json(ApiResponse::success(value)))
}

// ===== Task 13: CRM 360 视图与客户增强详情 =====

/// 分页参数（跟进记录等使用）
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct FollowUpQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// GET /api/v1/erp/crm/customers/:id/360 - 客户 360 全景视图
pub async fn get_customer_360(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();
    let value = service.get_customer_360(id, Some(&data_scope_ctx)).await?;
    Ok(Json(ApiResponse::success(value)))
}

// ===== Task 14: 跟进记录与 RFM =====

/// GET /api/v1/erp/crm/customers/:id/follow-ups - 列出跟进记录
pub async fn list_follow_ups(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<FollowUpQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();
    let page = params.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    let page_resp = service
        .list_follow_ups(id, page, page_size, Some(&data_scope_ctx))
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(page_resp)?)))
}

/// POST /api/v1/erp/crm/customers/:id/follow-ups - 创建跟进记录
pub async fn create_follow_up(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<FollowUpRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let operator_name = auth.username.clone();
    let value = service
        .create_follow_up(id, auth.user_id, operator_name, req)
        .await?;
    Ok(Json(ApiResponse::success(value)))
}

/// GET /api/v1/erp/crm/customers/:id/rfm - 获取单个客户 RFM 评分
pub async fn get_rfm_score(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let score = service.compute_rfm_score(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(score)?)))
}

/// GET /api/v1/erp/crm/rfm/distribution - 客户群体 RFM 分布
pub async fn get_rfm_distribution(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let dist = service.get_rfm_distribution().await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(dist)?)))
}

// ===== V15 P2 18.1-D4: 渠道 ROI 分析 =====

/// GET /api/v1/erp/crm/leads/channel-roi - 渠道 ROI 分析报表
pub async fn get_channel_roi_report(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ChannelRoiQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let start_date = params
        .start_date
        .ok_or_else(|| AppError::bad_request("start_date 必填".to_string()))?;
    let end_date = params
        .end_date
        .ok_or_else(|| AppError::bad_request("end_date 必填".to_string()))?;
    let report = service.channel_roi_report(start_date, end_date).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(report)?)))
}

/// POST /api/v1/erp/crm/leads/calculate-channel-roi - 计算渠道 ROI
pub async fn calculate_channel_roi(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CalculateChannelRoiRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let result = service
        .calculate_channel_roi(&req.source, req.start_date, req.end_date, req.cost)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(result)?)))
}

/// GET /api/v1/erp/crm/leads/allocation-rules - 获取线索分配规则列表
pub async fn list_allocation_rules(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<AllocationRuleQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let rules = service.list_allocation_rules(params.is_active).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(rules)?)))
}

/// POST /api/v1/erp/crm/leads/allocation-rules - 创建线索分配规则
pub async fn create_allocation_rule(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<crate::services::crm::lead::CreateAllocationRuleRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let rule = service.create_allocation_rule(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(rule)?)))
}

/// POST /api/v1/erp/crm/leads/:id/auto-assign - 自动分配线索
pub async fn auto_assign_lead(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<AutoAssignRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let assigned_user = service
        .auto_assign_lead(id, &req.source, req.industry.as_deref())
        .await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "assigned_user_id": assigned_user
    }))))
}

/// GET /api/v1/erp/crm/leads/nurture-plans - 获取线索培育计划列表
pub async fn list_nurture_plans(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<NurturePlanQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let plans = service
        .list_nurture_plans(params.lead_id, params.status.as_deref())
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(plans)?)))
}

/// POST /api/v1/erp/crm/leads/nurture-plans - 创建线索培育计划
pub async fn create_nurture_plan(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<crate::services::crm::lead::CreateNurturePlanRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let plan = service.create_nurture_plan(req, auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(plan)?)))
}

/// POST /api/v1/erp/crm/leads/nurture-plans/:id/execute - 执行线索培育计划
pub async fn execute_nurture_plan(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let plan = service.execute_nurture_plan(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(plan)?)))
}

// ===== V15 P2 18.2-D5/D6/D7: 商机增强 =====

/// GET /api/v1/erp/crm/opportunities/stage-duration - 阶段停留时长分析
pub async fn get_stage_duration_analysis(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<StageDurationQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let analysis = service
        .stage_duration_analysis(params.opportunity_id)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(analysis)?)))
}

/// POST /api/v1/erp/crm/opportunities/:id/stage-change - 记录商机阶段变更
pub async fn record_opportunity_stage_change(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<StageChangeRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    service
        .record_stage_change(id, req.from_stage, &req.to_stage, auth.user_id)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::Value::Null)))
}

/// GET /api/v1/erp/crm/opportunities/:id/competitors - 获取商机竞争对手
pub async fn list_opportunity_competitors(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let competitors = service.list_opportunity_competitors(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(
        competitors,
    )?)))
}

/// POST /api/v1/erp/crm/opportunities/:id/competitors - 添加商机竞争对手
pub async fn add_opportunity_competitor(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<crate::services::crm::opp::AddOpportunityCompetitorRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let competitor = service.add_opportunity_competitor(id, req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(
        competitor,
    )?)))
}

/// GET /api/v1/erp/crm/opportunities/:id/follow-ups - 获取商机跟进记录
pub async fn list_opportunity_follow_ups(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let follow_ups = service.list_opportunity_follow_ups(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(
        follow_ups,
    )?)))
}

/// POST /api/v1/erp/crm/opportunities/:id/follow-ups - 创建商机跟进记录
pub async fn create_opportunity_follow_up(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<crate::services::crm::opp::CreateOpportunityFollowUpRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let follow_up = service
        .create_opportunity_follow_up(id, req, auth.user_id, auth.username.clone())
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(follow_up)?)))
}

/// GET /api/v1/erp/crm/competitors - 获取竞争对手列表
pub async fn list_competitors(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let competitors = service.list_competitors().await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(
        competitors,
    )?)))
}

/// POST /api/v1/erp/crm/competitors - 创建竞争对手
pub async fn create_competitor(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<crate::services::crm::opp::CreateCompetitorRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let competitor = service.create_competitor(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(
        competitor,
    )?)))
}

// ===== V15 P2 18.4-D5/D6: 客户数据权限与操作日志 =====

/// GET /api/v1/erp/crm/customers/field-permissions/:role_id - 获取客户字段权限配置
pub async fn get_customer_field_permissions(
    Path(role_id): Path<i32>,
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let permissions = service.get_customer_field_permissions(role_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(
        permissions,
    )?)))
}

/// POST /api/v1/erp/crm/customers/field-permissions - 设置客户字段权限
pub async fn set_customer_field_permission(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<crate::services::crm::cust::SetFieldPermissionRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let permission = service.set_customer_field_permission(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(
        permission,
    )?)))
}

/// GET /api/v1/erp/crm/customers/:id/audit-logs - 获取客户操作日志
pub async fn list_customer_audit_logs(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<AuditLogQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let logs = service
        .list_customer_audit_logs(id, params.operation.as_deref())
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(logs)?)))
}

/// POST /api/v1/erp/crm/customers/:id/audit-logs - 创建客户操作日志
pub async fn create_customer_audit_log(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateAuditLogRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let audit_req = crate::services::crm::cust::CreateAuditLogRequest {
        customer_id: id,
        operation: req.operation,
        field_name: req.field_name,
        old_value: req.old_value,
        new_value: req.new_value,
        user_id: auth.user_id,
        user_name: auth.username.clone(),
        ip_address: req.ip_address,
        user_agent: req.user_agent,
    };
    service.log_customer_operation(audit_req).await?;
    Ok(Json(ApiResponse::success(serde_json::Value::Null)))
}

// ===== V15 P2 18.5-D5: 客户全生命周期价值（CLV）=====

/// POST /api/v1/erp/crm/customers/:id/clv/calculate - 计算客户 CLV
pub async fn calculate_customer_clv(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let clv = service.calculate_customer_clv(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(clv)?)))
}

/// GET /api/v1/erp/crm/customers/:id/clv - 获取客户 CLV
pub async fn get_customer_clv(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let clv = service.get_customer_clv(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(clv)?)))
}

// ===== V15 P2 18.2-D4/D5: 商机分析与预测 =====

/// GET /api/v1/erp/crm/opportunities/forecast-accuracy - 预测准确性分析
pub async fn get_forecast_accuracy(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ForecastAccuracyQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let now = chrono::Utc::now();
    let year = params.year.unwrap_or(now.year());
    let month = params.month.unwrap_or(now.month());
    let result = service.forecast_accuracy(year, month).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(result)?)))
}

/// GET /api/v1/erp/crm/opportunities/weighted-forecast - 加权销售预测
pub async fn get_weighted_forecast(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<OwnerQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let result = service.weighted_forecast(params.owner_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(result)?)))
}

/// GET /api/v1/erp/crm/opportunities/conversion-rate - 转化率分析
pub async fn get_conversion_rate(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<MonthsBackQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let result = service
        .conversion_rate_analysis(params.months_back.unwrap_or(12))
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(result)?)))
}

/// GET /api/v1/erp/crm/opportunities/sales-funnel - 销售漏斗报告
pub async fn get_sales_funnel(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<FunnelDateQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let result = service
        .sales_funnel_report(params.start_date, params.end_date)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(result)?)))
}

// ===== V15 P2 请求/查询 DTO =====

/// 渠道 ROI 查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ChannelRoiQuery {
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
}

/// 计算渠道 ROI 请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CalculateChannelRoiRequest {
    pub source: String,
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
    pub cost: rust_decimal::Decimal,
}

/// 分配规则查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct AllocationRuleQuery {
    pub is_active: Option<bool>,
}

/// 自动分配请求
#[derive(Debug, Deserialize)]
#[allow(dead_code, reason = "反序列化输入字段")]
pub struct AutoAssignRequest {
    pub source: String,
    pub industry: Option<String>,
}

/// 培育计划查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct NurturePlanQuery {
    pub lead_id: Option<i32>,
    pub status: Option<String>,
}

/// 阶段停留时长查询参数
#[derive(Debug, Deserialize)]
#[allow(dead_code, reason = "反序列化输入字段")]
pub struct StageDurationQuery {
    pub opportunity_id: Option<i32>,
}

/// 操作日志查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub operation: Option<String>,
}

/// 预测准确性查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ForecastAccuracyQuery {
    pub year: Option<i32>,
    pub month: Option<u32>,
}

/// 商机所有者查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct OwnerQuery {
    pub owner_id: Option<i32>,
}

/// 月数查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct MonthsBackQuery {
    pub months_back: Option<u32>,
}

/// 漏斗日期查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct FunnelDateQuery {
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
}

/// 创建审计日志请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CreateAuditLogRequest {
    pub operation: String,
    pub field_name: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

/// 阶段变更请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct StageChangeRequest {
    pub from_stage: Option<String>,
    pub to_stage: String,
}

/// POST /api/v1/crm/leads/:id/score - 线索评分
pub async fn score_lead(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(lead_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let result = service.score_lead(lead_id).await?;
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Update,
        severity: Severity::Info,
        resource_type: Some("crm_lead".to_string()),
        resource_id: Some(lead_id.to_string()),
        resource_name: None,
        description: Some("线索评分".to_string()),
        request_method: Some("POST".to_string()),
        request_path: Some(format!("/api/v1/crm/leads/{}/score", lead_id)),
        before_snapshot: None,
        after_snapshot: None,
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);
    let value = serde_json::to_value(result)?;
    Ok(Json(ApiResponse::success(value)))
}

/// POST /api/v1/crm/leads/detect-duplicates - 重复线索检测
pub async fn detect_duplicate_leads(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let mobile_phone = req.get("mobile_phone").and_then(|v| v.as_str());
    let company_name = req.get("company_name").and_then(|v| v.as_str());
    let result = service
        .detect_duplicate_leads(mobile_phone, company_name)
        .await?;
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Query,
        severity: Severity::Info,
        resource_type: Some("crm_lead".to_string()),
        resource_id: None,
        resource_name: None,
        description: Some("重复线索检测".to_string()),
        request_method: Some("POST".to_string()),
        request_path: Some("/api/v1/crm/leads/detect-duplicates".to_string()),
        before_snapshot: None,
        after_snapshot: None,
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);
    let value = serde_json::to_value(result)?;
    Ok(Json(ApiResponse::success(value)))
}

/// POST /api/v1/crm/leads/merge - 合并重复线索
pub async fn merge_leads(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    // 请求体是 JSON（带类型），故不做字符串→数字兼容：键缺失与值类型不符分别报不同文案，
    // 避免用户明明提交了值却被告知「必填」。回显的只是用户自己提交的原始值，不含服务端查得的数据。
    let raw_primary_id = req
        .get("primary_id")
        .ok_or_else(|| AppError::validation_displayable("缺少 primary_id 参数"))?;
    let primary_id = raw_primary_id.as_i64().ok_or_else(|| {
        AppError::validation_displayable(format!(
            "primary_id 必须为整数，当前提交值：{raw_primary_id}"
        ))
    })? as i32;
    let raw_duplicate_ids = req
        .get("duplicate_ids")
        .ok_or_else(|| AppError::validation_displayable("缺少 duplicate_ids 参数"))?;
    let duplicate_id_items = raw_duplicate_ids.as_array().ok_or_else(|| {
        AppError::validation_displayable(format!(
            "duplicate_ids 必须为整数数组，当前提交值：{raw_duplicate_ids}"
        ))
    })?;
    // 逐元素显式校验：任一元素非整数即整体拒绝，禁止 filter_map 静默丢弃（否则用户以为已合并）
    let duplicate_ids: Vec<i32> = duplicate_id_items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            item.as_i64()
                .ok_or_else(|| {
                    AppError::validation_displayable(format!(
                        "duplicate_ids 第 {} 项必须为整数，当前提交值：{item}",
                        idx + 1
                    ))
                })
                .map(|id| id as i32)
        })
        .collect::<Result<Vec<i32>, AppError>>()?;
    let result = service
        .merge_leads(primary_id, duplicate_ids.clone(), auth.user_id)
        .await?;
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Update,
        severity: Severity::Info,
        resource_type: Some("crm_lead".to_string()),
        resource_id: Some(primary_id.to_string()),
        resource_name: None,
        description: Some(format!("合并 {} 条重复线索", duplicate_ids.len())),
        request_method: Some("POST".to_string()),
        request_path: Some("/api/v1/crm/leads/merge".to_string()),
        before_snapshot: None,
        after_snapshot: None,
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);
    let value = serde_json::to_value(result)?;
    Ok(Json(ApiResponse::success(value)))
}

/// GET /api/v1/crm/leads/funnel-report - 线索漏斗报表
pub async fn lead_funnel_report(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<FunnelDateQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let result = service
        .lead_funnel_report(query.start_date, query.end_date)
        .await?;
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Query,
        severity: Severity::Info,
        resource_type: Some("crm_lead".to_string()),
        resource_id: None,
        resource_name: None,
        description: Some("线索漏斗报表".to_string()),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/crm/leads/funnel-report".to_string()),
        before_snapshot: None,
        after_snapshot: None,
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);
    let value = serde_json::to_value(result)?;
    Ok(Json(ApiResponse::success(value)))
}
