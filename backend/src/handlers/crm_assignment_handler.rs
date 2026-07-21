//! CRM客户分配 Handler
//!
//! 提供客户分配、批量分配和分配历史查询功能

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::services::assignment_history_service::{
    AssignmentHistoryQuery, AssignmentHistoryService, CreateAssignmentHistoryRequest,
};
use crate::services::crm::assign::{
    AutoAssignRequest, ClaimLeadRequest, CrmAssignService, TransferLeadRequest,
};
use crate::services::crm::cust::CrmService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 分配客户请求
#[derive(Debug, Deserialize)]
pub struct AssignCustomerRequest {
    pub lead_id: i32,
    pub assignee_id: i32,
    pub assignee_name: String,
    pub notes: Option<String>,
}

/// 批量分配请求
#[derive(Debug, Deserialize)]
pub struct BatchAssignRequest {
    pub lead_ids: Vec<i32>,
    pub assignee_id: i32,
    pub assignee_name: String,
    pub notes: Option<String>,
}

/// POST /api/v1/erp/crm/assignment - 分配客户
pub async fn assign_customer(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<AssignCustomerRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let crm_service = CrmService::new(state.db.clone());
    let history_service = AssignmentHistoryService::new(state.db.clone());

    // 获取线索（业务操作，不注入数据权限）
    let lead = crm_service.get_lead(req.lead_id, None).await?;

    // 记录原归属人
    let from_user_id = lead.owner_id;
    let from_user_name = lead.owner_name.clone();

    // 更新线索归属人
    let update_req = crate::models::dto::crm_dto::UpdateLeadRequest {
        lead_status: Some("assigned".to_string()),
        ..Default::default()
    };

    let updated_lead = crm_service
        .update_lead(req.lead_id, update_req, auth.user_id)
        .await?;

    // 记录分配历史
    history_service
        .create(
            auth.user_id,
            &auth.username,
            CreateAssignmentHistoryRequest {
                lead_id: updated_lead.id,
                lead_no: updated_lead.lead_no.clone(),
                company_name: updated_lead.company_name.clone(),
                from_user_id: Some(from_user_id),
                from_user_name: Some(from_user_name.clone()),
                to_user_id: Some(req.assignee_id),
                to_user_name: Some(req.assignee_name.clone()),
                action: "ASSIGN".to_string(),
                reason: None,
                notes: req.notes,
            },
        )
        .await?;

    tracing::info!(
        "用户 {} 将客户 {} 分配给 {} ({})",
        auth.username,
        updated_lead.id,
        req.assignee_name,
        req.assignee_id
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({
            "lead_id": updated_lead.id,
            "from_user_id": from_user_id,
            "from_user_name": from_user_name,
            "to_user_id": req.assignee_id,
            "to_user_name": req.assignee_name,
            "assigned_at": chrono::Utc::now().to_rfc3339(),
        }),
        "客户分配成功",
    )))
}

/// POST /api/v1/erp/crm/assignment/batch - 批量分配客户
pub async fn batch_assign(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<BatchAssignRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let crm_service = CrmService::new(state.db.clone());
    let history_service = AssignmentHistoryService::new(state.db.clone());

    // v16 批次 44 修复：循环外批量查询所有 lead，避免循环内逐个 get_lead（N+1 查询）
    let lead_map = load_lead_map(&state.db, &req.lead_ids).await;

    let ctx = LeadAssignCtx {
        crm_service: &crm_service,
        history_service: &history_service,
        user_id: auth.user_id,
        username: &auth.username,
        assignee_id: req.assignee_id,
        assignee_name: &req.assignee_name,
        notes: &req.notes,
    };

    let (assigned_count, failed_count, errors) =
        run_batch_assign_loop(&ctx, &lead_map, &req.lead_ids).await;

    tracing::info!(
        "用户 {} 批量分配客户: 成功={}, 失败={}",
        auth.username,
        assigned_count,
        failed_count
    );

    Ok(Json(ApiResponse::success(serde_json::json!({
        "total": req.lead_ids.len(),
        "assigned": assigned_count,
        "failed": failed_count,
        "errors": errors,
    }))))
}

/// 单个 lead 分配上下文（参数对象，避免 helper 参数过多）
struct LeadAssignCtx<'a> {
    crm_service: &'a CrmService,
    history_service: &'a AssignmentHistoryService,
    user_id: i32,
    username: &'a str,
    assignee_id: i32,
    assignee_name: &'a str,
    notes: &'a Option<String>,
}

/// 批量查询 lead 列表，返回以 id 为键的 map（空 ids 返回空 map）
async fn load_lead_map(
    db: &std::sync::Arc<sea_orm::DatabaseConnection>,
    lead_ids: &[i32],
) -> std::collections::HashMap<i32, crate::models::crm_lead::Model> {
    if lead_ids.is_empty() {
        return std::collections::HashMap::new();
    }
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    crate::models::crm_lead::Entity::find()
        .filter(crate::models::crm_lead::Column::Id.is_in(lead_ids))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|l| (l.id, l))
        .collect()
}

/// 执行批量分配循环，返回 (成功数, 失败数, 错误列表)
async fn run_batch_assign_loop(
    ctx: &LeadAssignCtx<'_>,
    lead_map: &std::collections::HashMap<i32, crate::models::crm_lead::Model>,
    lead_ids: &[i32],
) -> (u32, u32, Vec<String>) {
    let mut assigned_count = 0u32;
    let mut failed_count = 0u32;
    let mut errors = Vec::new();

    for lead_id in lead_ids {
        match lead_map.get(lead_id) {
            Some(lead) => match assign_single_lead(ctx, lead, *lead_id).await {
                Ok(_) => assigned_count += 1,
                Err(msg) => {
                    errors.push(msg);
                    failed_count += 1;
                }
            },
            None => {
                errors.push(format!("获取客户 {} 失败: 线索不存在", lead_id));
                failed_count += 1;
            }
        }
    }

    (assigned_count, failed_count, errors)
}

/// 处理单个 lead 的分配：状态检查、更新归属人、记录历史
async fn assign_single_lead(
    ctx: &LeadAssignCtx<'_>,
    lead: &crate::models::crm_lead::Model,
    lead_id: i32,
) -> Result<(), String> {
    // 检查是否可以分配
    if lead.lead_status.as_deref() == Some("converted") {
        return Err(format!("客户 {} 已转化为客户，无法分配", lead_id));
    }

    // 更新归属人
    let update_req = crate::models::dto::crm_dto::UpdateLeadRequest {
        lead_status: Some("assigned".to_string()),
        ..Default::default()
    };
    ctx.crm_service
        .update_lead(lead_id, update_req, ctx.user_id)
        .await
        .map_err(|e| format!("分配客户 {} 失败: {}", lead_id, e))?;

    // 记录分配历史（批次 114 P1-6：失败改 warn 日志，不静默吞错）
    if let Err(e) = ctx
        .history_service
        .create(
            ctx.user_id,
            ctx.username,
            CreateAssignmentHistoryRequest {
                lead_id,
                lead_no: lead.lead_no.clone(),
                company_name: lead.company_name.clone(),
                from_user_id: Some(lead.owner_id),
                from_user_name: Some(lead.owner_name.clone()),
                to_user_id: Some(ctx.assignee_id),
                to_user_name: Some(ctx.assignee_name.to_string()),
                action: "ASSIGN".to_string(),
                reason: None,
                notes: ctx.notes.clone(),
            },
        )
        .await
    {
        tracing::warn!(error = %e, lead_id, "客户分配历史记录失败");
    }

    Ok(())
}

/// GET /api/v1/erp/crm/assignments - 获取分配列表
pub async fn list_assignments(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<AssignmentHistoryQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = AssignmentHistoryService::new(state.db.clone());

    let (items, total) = service.list(query).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
    }))))
}

/// GET /api/v1/erp/crm/assignment/history - 获取分配历史
pub async fn list_assignment_history(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<AssignmentHistoryQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = AssignmentHistoryService::new(state.db.clone());

    let (items, total) = service.list(query).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
    }))))
}

/// POST /api/v1/erp/crm/assignments/auto-assign - 自动分配线索（轮询策略）
///
/// v10 P1 批次 140 新增：实现 assign 模块"保留扩展空间"中的自动分配功能。
/// 将 lead_status='new' 的未分配线索按 round-robin 轮询分配给指定销售团队。
pub async fn auto_assign(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<AutoAssignRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmAssignService::new(state.db.clone());

    let result = service
        .auto_assign_leads(req, auth.user_id, &auth.username)
        .await?;

    tracing::info!(
        "用户 {} 触发自动分配：分配 {} 条线索，参与销售 {} 人",
        auth.username,
        result.assigned_count,
        result.assignee_count
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(result)?,
        "自动分配完成",
    )))
}

/// POST /api/v1/erp/crm/assignments/transfer - 转移线索归属人
///
/// v10 P1 批次 140 新增：实现 assign 模块"保留扩展空间"中的转移分配功能。
/// 将线索从当前归属人转移给新归属人，记录转移原因和备注（action="TRANSFER"）。
pub async fn transfer_lead(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<TransferLeadRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmAssignService::new(state.db.clone());

    let result = service
        .transfer_lead(req, auth.user_id, &auth.username)
        .await?;

    tracing::info!(
        "用户 {} 转移线索 {}：从用户 {} 转移给用户 {}",
        auth.username,
        result.lead_id,
        result.from_user_id,
        result.to_user_id
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(result)?,
        "线索转移成功",
    )))
}

/// GET /api/v1/erp/crm/assignments/workload - 查询销售用户线索负载
///
/// v10 P1 批次 140 新增：辅助端点，查询指定销售用户列表的当前活跃线索数，
/// 用于自动分配前的预览（按负载升序排序，负载最少的优先分配）。
#[derive(Debug, Deserialize)]
pub struct WorkloadQuery {
    /// 逗号分隔的用户 ID 列表（如 ?user_ids=1,2,3）
    pub user_ids: String,
}

pub async fn list_workload(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<WorkloadQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let user_ids: Vec<i32> = query
        .user_ids
        .split(',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .collect();

    if user_ids.is_empty() {
        return Err(AppError::validation("user_ids 参数不能为空"));
    }

    let service = CrmAssignService::new(state.db.clone());
    let workload = service.list_assignee_workload(&user_ids).await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(workload)?)))
}

/// POST /api/v1/erp/crm/assignments/claim - 抢单模式认领线索
///
/// v10 P1 批次 140 新增：实现 assign 模块"保留扩展空间"中的抢单功能。
/// 销售主动认领一条未分配线索（FIFO，最早入库的优先），写入分配历史 action="CLAIM"。
pub async fn claim_lead(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<ClaimLeadRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmAssignService::new(state.db.clone());

    let result = service
        .claim_lead(req, auth.user_id, &auth.username)
        .await?;

    tracing::info!(
        "用户 {} 抢单认领线索 {}，归属人从 {} 变更为 {}",
        auth.username,
        result.lead_id,
        result.from_user_id,
        result.to_user_id
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(result)?,
        "线索认领成功",
    )))
}
