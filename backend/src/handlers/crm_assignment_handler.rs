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

    // 获取线索
    let lead = crm_service.get_lead(req.lead_id).await?;

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

    let mut assigned_count = 0;
    let mut failed_count = 0;
    let mut errors = Vec::new();

    // v16 批次 44 修复：循环外批量查询所有 lead，避免循环内逐个 get_lead（N+1 查询）
    let lead_ids = req.lead_ids.clone();
    let lead_map: std::collections::HashMap<i32, crate::models::crm_lead::Model> =
        if lead_ids.is_empty() {
            std::collections::HashMap::new()
        } else {
            use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
            crate::models::crm_lead::Entity::find()
                .filter(crate::models::crm_lead::Column::Id.is_in(lead_ids))
                .all(&*state.db)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|l| (l.id, l))
                .collect()
        };

    for lead_id in &req.lead_ids {
        match lead_map.get(lead_id) {
            Some(lead) => {
                // 检查是否可以分配
                if lead.lead_status.as_deref() == Some("converted") {
                    errors.push(format!("客户 {} 已转化为客户，无法分配", lead_id));
                    failed_count += 1;
                    continue;
                }

                // 更新归属人
                let update_req = crate::models::dto::crm_dto::UpdateLeadRequest {
                    lead_status: Some("assigned".to_string()),
                    ..Default::default()
                };

                match crm_service
                    .update_lead(*lead_id, update_req, auth.user_id)
                    .await
                {
                    Ok(_) => {
                        // 记录分配历史
                        let _ = history_service
                            .create(
                                auth.user_id,
                                &auth.username,
                                CreateAssignmentHistoryRequest {
                                    lead_id: *lead_id,
                                    lead_no: lead.lead_no.clone(),
                                    company_name: lead.company_name.clone(),
                                    from_user_id: Some(lead.owner_id),
                                    from_user_name: Some(lead.owner_name.clone()),
                                    to_user_id: Some(req.assignee_id),
                                    to_user_name: Some(req.assignee_name.clone()),
                                    action: "ASSIGN".to_string(),
                                    reason: None,
                                    notes: req.notes.clone(),
                                },
                            )
                            .await;

                        assigned_count += 1;
                    }
                    Err(e) => {
                        errors.push(format!("分配客户 {} 失败: {}", lead_id, e));
                        failed_count += 1;
                    }
                }
            }
            None => {
                errors.push(format!("获取客户 {} 失败: 线索不存在", lead_id));
                failed_count += 1;
            }
        }
    }

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
