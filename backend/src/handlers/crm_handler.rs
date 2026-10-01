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
// #207/#208：导出需复用列表同一判定源/同一字段过滤函数（filter_fields_batch）
use crate::services::data_permission_service::{DataPermissionResult, DataPermissionService};
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

/// 角色数据权限取数（读路径与导出路径**共用这一份判定**，不再各写一遍
/// `if let Ok(Some(..))`）：判定源 = `data_permission_service.get_role_data_permission`
/// （admin 依据 `roles.code='admin'`，`utils/admin_checker.rs:87`）。
/// 返回 `Some(配置行)` 时调用方走 `filter_fields*`（allowed 白名单 / hidden 移除）；
/// 返回 `None` 表示"无权限行"或"查询失败"——查询失败必须显式记 warn（不静默），
/// 并按无权限行让调用方走各自的默认处理（fail-closed）。
/// admin 例外（放行原文）由各默认处理实现自身保留：读路径
/// `CrmService::mask_lead_pii_defaults` 对 `role_id == Some(1)` 原样返回，
/// 导出路径由调用方的 `role_id != 1` 门控，二者都不因本函数而改变既有原值契约。
async fn resolve_role_data_permission(
    state: &AppState,
    role_id: i32,
    resource_type: &str,
) -> Option<DataPermissionResult> {
    match state
        .data_permission_service
        .get_role_data_permission(role_id, resource_type)
        .await
    {
        Ok(permission) => permission,
        Err(error) => {
            tracing::warn!(
                %error,
                role_id,
                resource_type,
                "角色数据权限查询失败，出参按无权限行 fail-closed 走默认处理"
            );
            None
        }
    }
}

/// #204/#208：线索**字段级**数据权限的唯一实现，四个出口共用（本文件 `list_leads`
/// 列表、`get_lead` 详情，以及 `crm_pool_handler` 的公海列表与领取/回收写响应）。
/// 判定源与掩码实现都不再各写一份内联分支：
/// - 配了角色数据权限行 → 与导出同一个 `filter_fields_batch`（allowed_fields 白名单、
///   hidden_fields 移除，不叠加默认打码；admin 由 `get_role_data_permission` 返回
///   `Ok(Some{allowed:None,hidden:None})` → 空操作，保持原值契约）；
/// - 无权限行且非 admin → `CrmService::mask_lead_pii_defaults`（列集合取自
///   `utils/field_mask` 的权威定义：`mobile_phone`/`tel_phone` 掩码、`email` 掩码、
///   `address` 整键移除）。
/// fail-closed：`role_id` 缺失或权限查询 `Err` 都按"无权限行"走默认脱敏（与已提交的
/// `export_leads` 的 None 分支同口径），不放行原文。本函数只处理出参，不改状态码、
/// 不外显任何拒绝原因（权限拒绝仍走 `AppError::permission_denied` 的固定脱敏信封）。
pub(crate) async fn apply_lead_field_permission(
    state: &AppState,
    role_id: Option<i32>,
    leads: &mut [serde_json::Value],
) {
    if let Some(rid) = role_id {
        if let Some(permission) = resolve_role_data_permission(state, rid, "crm_lead").await {
            state.data_permission_service.filter_fields_batch(
                leads,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
            return;
        }
        // Ok(None) / 查询 Err（已在 resolve 内记 warn）：落到下方默认脱敏；
        // admin 由 mask_lead_pii_defaults 自身放行原文，原值契约不变。
    }

    for lead in leads.iter_mut() {
        *lead = CrmService::mask_lead_pii_defaults(std::mem::take(lead), role_id);
    }
}

/// 从分页出参中定位列表数组：兼容既有两种出参键（`services/crm/lead.rs` 手搓 `json!`
/// 用 `data`，历史列表接口用 `list`）。定位不到意味着字段级权限无处可施
/// = 原文直通，属形状漂移而非正常分支，显式记 error 不静默。
/// `pub(crate)`：客户增强入口（`crm_customer_handler::list_customers`）出参同为
/// `list_leads` 分页形状，必须复用同一数组定位实现，不再各写一份键名分支。
pub(crate) fn paginated_list_array_mut(
    value: &mut serde_json::Value,
) -> Option<&mut Vec<serde_json::Value>> {
    let key = if value.get("list").and_then(Value::as_array).is_some() {
        "list"
    } else {
        "data"
    };
    let list = value.get_mut(key).and_then(Value::as_array_mut);
    if list.is_none() {
        tracing::error!(
            key = %key,
            "分页出参未定位到列表数组（list/data 均缺失），字段级数据权限未应用"
        );
    }
    list
}

/// #209：商机**字段级**数据权限的唯一实现，四个出口共用（本文件 `list_opportunities`
/// 列表、`get_opportunity` 详情、`create_opportunity` 建单、`update_opportunity`/
/// `close_opportunity_as_lost` 写响应）。与线索侧 `apply_lead_field_permission` 同形态：
/// - 配了角色数据权限行 → 与导出同一个 `filter_fields_batch`（allowed 白名单 / hidden 移除，
///   不叠加默认处理；admin 由 `get_role_data_permission` 返回
///   `Ok(Some{allowed:None,hidden:None})` → 空操作，保持原值契约）；
/// - 无权限行且非 admin → 默认隐藏（本函数保留改造前的**逐键等价行为**，见下）。
///
/// 【等价性说明 / 待用户裁定项】改造前 `list_opportunities`/`get_opportunity` 的默认分支
/// 写的是 `obj.remove("amount")`，而 `crm_opportunity` 出参根本没有 `amount` 键（真实金额列是
/// `estimated_amount`/`actual_amount`，见 `EXPORT_AMOUNT_COLUMNS`）——即这处"移除"当前恒不
/// 生效，商机金额对非 admin 实际仍是原文。是否连带收紧列表/详情/写响应的金额默认隐藏属
/// **待用户拍板**的口径（隐藏范围是否含本人行），本轮决策结论为"仅非本人行剔除、且需用户
/// 拍板"，故本函数**只做等价重构**：把同一份既有逻辑（含这处恒不生效的 `remove("amount")`）
/// 收敛为一个函数并被四个出口共用，不改实际生效范围。该恒不生效分支现保留在本函数的默认
/// 分支中等待裁定（导出侧 `drop_export_amount_columns` 按真实列名剔除，是更严的一侧，本轮不动）。
pub(crate) async fn apply_opportunity_field_permission(
    state: &AppState,
    role_id: Option<i32>,
    opportunities: &mut [serde_json::Value],
) {
    // 与改造前 `if let Some(role_id) = auth.role_id { ... }` 逐键一致：role_id 缺失时
    // 本函数对出参不做任何处理（保持既有商机的实际行为；线索侧 None 走默认脱敏是另一条
    // 已裁定口径，不在此处对齐——本轮只收敛商机，不改变其生效范围）。
    if let Some(rid) = role_id {
        if let Some(permission) = resolve_role_data_permission(state, rid, "crm_opportunity").await
        {
            state.data_permission_service.filter_fields_batch(
                opportunities,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
            return;
        }
        if rid != 1 {
            // 无权限行且非 admin（查询 Err 亦在此，已在 resolve 内记 warn）：
            // 保留改造前的默认隐藏写法（现恒不生效，见函数文档"待用户裁定"）。
            for opportunity in opportunities.iter_mut() {
                if let Some(obj) = opportunity.as_object_mut() {
                    obj.remove("amount");
                }
            }
        }
    }
}

pub async fn create_lead(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateLeadRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    // 归属人展示名取 `auth.username`（真实登录名，见 services/crm/lead.rs::create_lead 文档）
    let res = service
        .create_lead(req, auth.user_id, &auth.username)
        .await?;
    let mut value = serde_json::to_value(res)?;
    // #209：建单成功响应不再整行原文回传——整行 crm_lead::Model 含 mobile_phone/
    // tel_phone/email/address 明文，与 GET 详情被打码形成"读打码、写原文"旁路。
    // 复用读路径唯一实现（公海写响应同款），本处不再内联掩码分支。
    apply_lead_field_permission(&state, auth.role_id, std::slice::from_mut(&mut value)).await;
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

    // 字段级数据权限：与 get_lead / 公海列表 / 公海写响应收敛到同一个实现
    // （apply_lead_field_permission），本处不再内联 mask_phone/mask_email 分支——
    // 内联分支正是"列表漏 tel_phone、写响应回原文"两处漂移的成因。
    if let Some(list) = paginated_list_array_mut(&mut value) {
        apply_lead_field_permission(&state, auth.role_id, list).await;
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
            if let Some(permission) =
                resolve_role_data_permission(&state, role_id, "crm_lead").await
            {
                // 配置了数据权限行：与列表分支同一函数 filter_fields_batch
                // （allowed_fields 白名单保留、hidden_fields 移除，不叠加默认打码）
                apply_export_field_permission(
                    &state.data_permission_service,
                    &mut table,
                    CrmService::EXPORT_LEAD_COLUMNS,
                    &permission.allowed_fields,
                    &permission.hidden_fields,
                )?;
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

/// 导出表形状硬校验（fail-closed，字段处理各分支共用的唯一入口检查）：
/// 每一行的单元格数必须与列定义表列数**逐行相等**。
/// - 行长 > 列数：多出的单元格不属于任何列定义 = 不经过任何字段级权限处理即进导出文件
///   （原文 PII 直通），且无法按列名定位，必须拒绝；
/// - 行长 < 列数：按列下标处理会整体错位（前一列的值被当后一列掩码/剔除），同样拒绝。
/// 宁可不生成文件，也不放行未处理列——行构造与列定义漂移属编程错误，显式报错到调用方。
fn validate_export_row_shape(
    table: &crate::utils::xlsx_export::XlsxTable,
    columns: &[(&str, &str)],
) -> Result<(), AppError> {
    for (row_idx, row) in table.rows.iter().enumerate() {
        if row.len() != columns.len() {
            tracing::error!(
                row_idx,
                row_len = row.len(),
                column_count = columns.len(),
                "导出行单元格数与列定义表列数不一致，已拒绝生成导出文件（不放行未处理列）"
            );
            return Err(AppError::internal(format!(
                "导出行第 {row_idx} 行单元格数({})与列定义数({})不一致，拒绝导出（列定义与行构造漂移，需修服务层）",
                row.len(),
                columns.len()
            )));
        }
    }
    Ok(())
}

/// 导出表 → 行对象数组（列定义表驱动）
///
/// #207：把导出表按**列定义表**转成行对象数组（键 = crm_* 列名，与列表/详情出参键同源），
/// 供复用列表/详情同一个判定函数 filter_fields_batch。
/// 列定义表由调用方传入（线索 `CrmService::EXPORT_LEAD_COLUMNS`、商机
/// `CrmService::EXPORT_OPP_COLUMNS`），两张导出表共用同一份转换实现。
/// 行长与列定义不一致属编程错误：入口处 `validate_export_row_shape` 整体拒绝，
/// 不再有"按空值继续"分支（空值兜底 = 该列内容去向不明，属静默）。
fn export_row_cells(
    table: &crate::utils::xlsx_export::XlsxTable,
    columns: &[(&str, &str)],
) -> Result<Vec<serde_json::Value>, AppError> {
    validate_export_row_shape(table, columns)?;
    table
        .rows
        .iter()
        .map(|row| {
            let mut obj = serde_json::Map::new();
            for (idx, (field, _)) in columns.iter().enumerate() {
                // 形状已在入口硬校验，此处取不到即校验被绕过 = 编程错误，显式失败不兜底
                let cell = row.get(idx).ok_or_else(|| {
                    AppError::internal(format!(
                        "导出行形状校验后被绕过：第 {idx} 列 {field} 缺失，拒绝导出"
                    ))
                })?;
                obj.insert(
                    (*field).to_string(),
                    serde_json::Value::String(cell.clone()),
                );
            }
            Ok(serde_json::Value::Object(obj))
        })
        .collect()
}

/// #207/#208：把处理后的行对象按列定义表回写导出表（被剔除的列落成空单元格，
/// 表头保留，避免同一角色不同入口的列结构漂移）。
/// 行形状由 `validate_export_row_shape` 在字段处理入口保证，此处任何取不到对象的
/// 分支都属编程错误，显式失败；历史上"记 error 后继续/按下标补位"的兜底属静默放行
/// 未处理内容，已移除。
fn write_back_export_rows(
    table: &mut crate::utils::xlsx_export::XlsxTable,
    columns: &[(&str, &str)],
    rows_json: &[serde_json::Value],
) -> Result<(), AppError> {
    validate_export_row_shape(table, columns)?;
    for (row_idx, row) in table.rows.iter_mut().enumerate() {
        let Some(obj) = rows_json.get(row_idx).and_then(|v| v.as_object()) else {
            return Err(AppError::internal(format!(
                "导出行掩码回写找不到第 {row_idx} 行对应行对象（行对象与表行一一对应被破坏），拒绝导出"
            )));
        };
        for (col_idx, (field, _)) in columns.iter().enumerate() {
            let value = obj
                .get(*field)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            match row.get_mut(col_idx) {
                Some(slot) => *slot = value,
                None => {
                    return Err(AppError::internal(format!(
                        "导出行第 {row_idx} 行列下标 {col_idx} 缺失（形状校验后被绕过），拒绝导出"
                    )));
                }
            }
        }
    }
    Ok(())
}

/// #207：导出文件的字段级数据权限（有配置数据权限行的分支）。
///
/// 复用列表/详情完全相同的判定实现 `filter_fields_batch`（allowed_fields 为白名单：
/// 未列入的列取值被剔除；hidden_fields 再移除），因此"配了 allowed_fields 的角色"
/// 天然就是放行原文的受控通道，无需为导出新增权限键。
fn apply_export_field_permission(
    dsp: &DataPermissionService,
    table: &mut crate::utils::xlsx_export::XlsxTable,
    columns: &[(&str, &str)],
    allowed_fields: &Option<Vec<String>>,
    hidden_fields: &Option<Vec<String>>,
) -> Result<(), AppError> {
    let mut rows_json = export_row_cells(table, columns)?;
    dsp.filter_fields_batch(&mut rows_json, allowed_fields, hidden_fields);
    write_back_export_rows(table, columns, &rows_json)
}

/// 导出文件默认字段处理的动作类型：
/// - `MaskPhone` / `MaskEmail`：与列表/详情默认脱敏同一列集合、同一实现
///   （`utils/field_mask::mask_phone` / `mask_email`）；
/// - `Drop`：整列剔除（导出侧表现为空单元格），与 filter_fields 移除隐藏列的表现一致
///   —— 商机金额在列表/详情走的就是"移除"而非掩码（`list_opportunities`），
///   导出侧同口径移除，不新造第三种处理。
#[derive(Clone, Copy)]
enum ExportColumnAction {
    MaskPhone,
    MaskEmail,
    Drop,
}

/// 按列定义表把列名定位成导出表下标。
/// 列名在定义表中找不到属编程错误，且**必须**报错 fail-closed：
/// 静默跳过该列 = 该列原文直通（正是本波次要堵的旁路）。
fn export_column_positions(
    columns: &[(&str, &str)],
    fields: &[&str],
) -> Result<Vec<usize>, AppError> {
    fields
        .iter()
        .map(|field| {
            CrmService::export_column_index(columns, field).ok_or_else(|| {
                AppError::internal(format!("导出列定义缺少敏感列 {field}，拒绝导出原文"))
            })
        })
        .collect()
}

/// 导出文件的默认字段处理（无数据权限行且非 admin 分支，与列表/详情
/// `CrmService::mask_lead_pii_defaults` 同一列集合口径）：按列定义表以列名定位，
/// 逐列执行掩码或剔除。空单元格不掩码（否则空值会变成 `*`，与列表"无值"表现不一致）。
fn apply_default_export_actions(
    table: &mut crate::utils::xlsx_export::XlsxTable,
    columns: &[(&str, &str)],
    actions: &[(&[&str], ExportColumnAction)],
) -> Result<(), AppError> {
    // 形状硬校验先行：行长 > 列数的多余单元格不进任何处理（原文直通），
    // 行长 < 列数会按下标错位处理——两种漂移都必须整体拒绝生成文件，
    // 历史分支"记 error 后让敏感列脱管"已移除。
    validate_export_row_shape(table, columns)?;
    let mut targets: Vec<(usize, ExportColumnAction)> = Vec::new();
    for (fields, action) in actions {
        for col_idx in export_column_positions(columns, fields)? {
            targets.push((col_idx, *action));
        }
    }

    for row in table.rows.iter_mut() {
        for (col_idx, action) in &targets {
            // 入口已硬校验行形状，此处取不到即校验被绕过 = 编程错误，显式失败不兜底
            let Some(cell) = row.get_mut(*col_idx) else {
                return Err(AppError::internal(format!(
                    "导出行形状校验后被绕过：单元格下标 {col_idx} 缺失（行长度 {}），拒绝导出",
                    row.len()
                )));
            };
            match action {
                ExportColumnAction::Drop => *cell = String::new(),
                ExportColumnAction::MaskPhone | ExportColumnAction::MaskEmail => {
                    // 空值不掩码：掩码空串会产出 "*" 假数据，与列表"无值"表现不一致
                    if cell.is_empty() {
                        continue;
                    }
                    *cell = match action {
                        ExportColumnAction::MaskPhone => {
                            crate::utils::field_mask::mask_phone(cell.as_str())
                        }
                        ExportColumnAction::MaskEmail => {
                            crate::utils::field_mask::mask_email(cell.as_str())
                        }
                        ExportColumnAction::Drop => String::new(),
                    };
                }
            }
        }
    }
    Ok(())
}

/// #207：线索导出的默认脱敏分支（列集合 = `EXPORT_PII_PHONE_COLUMNS` /
/// `EXPORT_PII_EMAIL_COLUMNS`，座机 `tel_phone` 与列表/详情同一实现）。
fn mask_export_pii_columns(
    table: &mut crate::utils::xlsx_export::XlsxTable,
) -> Result<(), AppError> {
    apply_default_export_actions(
        table,
        CrmService::EXPORT_LEAD_COLUMNS,
        &[
            (
                CrmService::EXPORT_PII_PHONE_COLUMNS,
                ExportColumnAction::MaskPhone,
            ),
            (
                CrmService::EXPORT_PII_EMAIL_COLUMNS,
                ExportColumnAction::MaskEmail,
            ),
        ],
    )
}

/// #208：商机导出的默认剔除分支（无数据权限行且非 admin）：金额列整列不外显，
/// 与 `list_opportunities` / `get_opportunity` 的"移除 amount 列"同一列集合口径
/// （列名取 crm_opportunity 真实列 `estimated_amount` / `actual_amount`）。
fn drop_export_amount_columns(
    table: &mut crate::utils::xlsx_export::XlsxTable,
) -> Result<(), AppError> {
    apply_default_export_actions(
        table,
        CrmService::EXPORT_OPP_COLUMNS,
        &[(CrmService::EXPORT_AMOUNT_COLUMNS, ExportColumnAction::Drop)],
    )
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
    let result = service
        .import_leads(bytes, auth.user_id, &auth.username)
        .await?;
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

    // 字段级数据权限：与列表同一实现（单元素切片复用批量函数，判定与掩码只有一份代码）
    apply_lead_field_permission(&state, auth.role_id, std::slice::from_mut(&mut value)).await;

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
    let mut value = serde_json::to_value(res)?;
    // #209：更新成功响应不再整行原文回传。update_lead 受行级 check_resource_owner 约束，
    // 但 Dept 数据范围用户可合法更新他人名下的行，其响应会把他人手机号/邮箱/地址原文
    // 送出——而同一个人 GET /crm/leads/:id 拿到的是打码值，即"读打码、写原文"旁路。
    // 复用读路径唯一实现（apply_lead_field_permission），与列表/详情/公海写响应同源。
    apply_lead_field_permission(&state, auth.role_id, std::slice::from_mut(&mut value)).await;
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
    // 归属人展示名取 `auth.username`（真实登录名，见 services/crm/opp.rs::create_opportunity 文档）
    let res = service
        .create_opportunity(req, auth.user_id, &auth.username)
        .await?;
    let mut value = serde_json::to_value(res)?;
    // #209：建单成功响应走商机字段级出参唯一实现（与列表/详情/更新同源），
    // 不再整行原文直出。本函数默认分支的等价性说明见 apply_opportunity_field_permission。
    apply_opportunity_field_permission(&state, auth.role_id, std::slice::from_mut(&mut value))
        .await;
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

    // 字段级数据权限：与 get_opportunity / 建单 / 更新收敛到同一个实现
    // （apply_opportunity_field_permission），本处不再内联分支——内联分支正是"写响应回原文"
    // 旁路的成因。列表定位 list/data 数组后批量处理（详见该函数文档的等价性/待裁定说明）。
    if let Some(list) = paginated_list_array_mut(&mut value) {
        apply_opportunity_field_permission(&state, auth.role_id, list).await;
    }

    Ok(Json(ApiResponse::success(value)))
}

/// GET /api/v1/erp/crm/opportunities/export - 导出商机为 xlsx；v11 批次 141 新增：前端 exportOpportunities API 真实接入。 v11 批次
/// 142 升级：导出格式从 CSV 升级为 xlsx（规则 3 强制要求）。 返回 application/vnd.openxmlformats-officedocument.spreadsheetml.sheet
///
/// 数据权限（#208 遗留项收口，修法与上面 `export_leads`（#207）完全同构）：商机导出必须与
/// 本文件 `list_opportunities` / `get_opportunity` 走同一条权限链——
/// 行级：`auth.to_data_scope_context()` 注入 service，套用与列表**同一个**
/// `apply_department_scope`（`services/crm/opp.rs:162`；商机无公海语义，
/// **不可**误用带 pool 放行的 `apply_department_scope_with_pool`），使导出的行集合与
/// 该用户列表可见集严格一致；
/// 字段级：同一个判定源（`data_permission_service.get_role_data_permission(role_id,
/// "crm_opportunity")`，admin 依据 roles.code='admin'）+ 同一个
/// `filter_fields_batch`；无权限行且非 admin 时按 `EXPORT_AMOUNT_COLUMNS`
/// （`estimated_amount`/`actual_amount`，出参真实列名）整列剔除金额，与列表/详情
/// 默认隐藏分支的意图一致。
/// 注：`list_opportunities`/`get_opportunity` 的该分支写的是 `obj.remove("amount")`，
/// 而 `crm_opportunity` 出参并无 `amount` 键（真实列是上面两列）——即列表侧那处
/// "移除"当前恒不生效，属同类的读错键缺陷，是否连带收紧列表出参（影响前端金额列展示）
/// 需用户拍板，本批不动（见交付报告"待决点"）。导出侧按真实列名剔除，是更严的一侧。
/// 修复前两层都没有：service 侧只按 `opportunity_stage` 过滤（不吃行级 ctx）、
/// handler 也不查角色字段权限
/// ⇒ self/dept 用户点一次"导出"即拿到全库商机含金额，属越权读 + 商业秘密外泄。
/// 两层均为强制，不按查询参数开关。
pub async fn export_opportunities(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<OpportunityQuery>,
) -> Result<axum::response::Response, AppError> {
    // V15 P1-9-1：全局导出并发控制（RAII 守卫，函数退出自动递减）
    let _guard = ExportConcurrencyGuard::acquire()?;

    let service = CrmService::new(state.db.clone());
    // 行级数据权限：与 list_opportunities（本文件 :562-566）同法构造并注入 ctx；
    // 省略 ctx 会让 service 层整体跳过行级过滤，self/dept 用户可一次拿到全库商机。
    let data_scope_ctx = auth.to_data_scope_context();
    let mut table = service
        .export_opportunities(query, Some(&data_scope_ctx))
        .await?;

    // 字段级数据权限：判定分支与 list_opportunities/get_opportunity 同构，
    // 取数与线索导出、列表/详情共用同一函数 resolve_role_data_permission
    // （fail-closed 口径一致：role_id 缺失按非 admin 处理）
    match auth.role_id {
        Some(role_id) => {
            if let Some(permission) =
                resolve_role_data_permission(&state, role_id, "crm_opportunity").await
            {
                apply_export_field_permission(
                    &state.data_permission_service,
                    &mut table,
                    CrmService::EXPORT_OPP_COLUMNS,
                    &permission.allowed_fields,
                    &permission.hidden_fields,
                )?;
            } else if role_id != 1 {
                drop_export_amount_columns(&mut table)?;
            }
        }
        None => drop_export_amount_columns(&mut table)?,
    }

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

    // 字段级数据权限：与列表/建单/更新同一实现（单元素切片复用批量函数，判定与处理只有一份代码）
    apply_opportunity_field_permission(&state, auth.role_id, std::slice::from_mut(&mut value))
        .await;

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
    let mut value = serde_json::to_value(res)?;
    // #209：更新成功响应走商机字段级出参唯一实现（与列表/详情/建单同源），不再整行原文直出。
    apply_opportunity_field_permission(&state, auth.role_id, std::slice::from_mut(&mut value))
        .await;
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
    let mut value = serde_json::to_value(res)?;
    // #209：关单（输单）同样是整行 crm_opportunity::Model 写响应，走商机字段级出参唯一实现，
    // 与列表/详情/建单/更新同源，不再整行原文直出。
    apply_opportunity_field_permission(&state, auth.role_id, std::slice::from_mut(&mut value))
        .await;
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
    // 出参形状 = 服务层 `convert_lead_to_customer`（services/crm/lead.rs）构造的三键摘要
    // {customer_id, customer_code, customer_name}：转化所需的稳定标识，不含
    // contact_phone/contact_email/address 等掩码列（整行 customer 行只落库、不回传），
    // 因此出参侧无需字段级权限处理，联系方式掩码由各读出口统一实施。
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

#[cfg(test)]
mod export_row_shape_guard_tests {
    //! #4 负例锁：导出对行长与列定义不一致必须拒绝出文件（宁可不生成，
    //! 也不放行未经字段处理的列）。行构造与列定义当前同源、HTTP 面不可达，
    //! 故以内部函数负例锁定硬校验本体，防止"按空值继续/跳过/push 补位"兜底回潮。

    use super::*;

    fn table(rows: Vec<Vec<String>>) -> crate::utils::xlsx_export::XlsxTable {
        crate::utils::xlsx_export::XlsxTable {
            sheet_name: "测试".to_string(),
            headers: vec!["列一".to_string(), "列二".to_string()],
            rows,
        }
    }

    const COLUMNS: [(&str, &str); 2] = [("col_a", "列一"), ("col_b", "列二")];

    #[test]
    fn longer_row_than_columns_is_rejected() {
        let t = table(vec![vec![
            "a".to_string(),
            "b".to_string(),
            "raw_pii_extra_cell".to_string(),
        ]]);
        validate_export_row_shape(&t, &COLUMNS)
            .expect_err("行长>列数：多余单元格未经任何字段处理，必须拒绝");
        // 处理函数本身也必须整体失败，而不是对多余列静默
        let mut t2 = table(vec![vec![
            "a".to_string(),
            "b".to_string(),
            "extra".to_string(),
        ]]);
        let pii_fields: &[&str] = &["col_a"];
        assert!(
            apply_default_export_actions(
                &mut t2,
                &COLUMNS,
                &[(pii_fields, ExportColumnAction::Drop)]
            )
            .is_err()
        );
    }

    #[test]
    fn shorter_row_than_columns_is_rejected() {
        let t = table(vec![vec!["a".to_string()]]);
        assert!(validate_export_row_shape(&t, &COLUMNS).is_err());
    }

    #[test]
    fn mismatch_is_rejected_by_export_row_cells_and_apply_export() {
        let t = table(vec![vec![
            "a".to_string(),
            "b".to_string(),
            "extra".to_string(),
        ]]);
        assert!(export_row_cells(&t, &COLUMNS).is_err());
    }

    #[test]
    fn matched_shape_still_passes_and_keeps_cells() {
        let t = table(vec![vec!["a".to_string(), "b".to_string()]]);
        let rows = export_row_cells(&t, &COLUMNS).expect("形状一致应通过");
        assert_eq!(rows[0]["col_a"], serde_json::Value::String("a".to_string()));
        assert_eq!(rows[0]["col_b"], serde_json::Value::String("b".to_string()));
    }
}
