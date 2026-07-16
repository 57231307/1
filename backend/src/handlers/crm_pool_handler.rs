//! CRM客户公海 Handler
//!
//! 提供客户公海池的列表查询、领取和回收功能

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::models::dto::crm_dto::BatchClaimRequest;
use crate::services::crm::cust::CrmService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 公海客户查询参数
#[derive(Debug, Deserialize)]
pub struct PoolQueryParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    // 批次 111 P1-10：source / keyword 接入 LeadQuery 过滤（移除 dead_code 标注）
    pub source: Option<String>,
    // v11 批次 153 P2-A：接入 industry 过滤（crm_lead.industry 列已通过 m0043 迁移添加）
    pub industry: Option<String>,
    pub keyword: Option<String>,
}

/// 领取客户请求
#[derive(Debug, Deserialize)]
pub struct ClaimRequest {
    pub lead_id: i32,
}

/// 回收客户请求
#[derive(Debug, Deserialize)]
pub struct RecycleRequest {
    pub lead_id: i32,
    pub reason: Option<String>,
}

/// GET /api/v1/erp/crm/pool - 获取公海客户列表
pub async fn list_pool(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<PoolQueryParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());

    let page = params.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    // 查询公海客户（owner_id为空或特定状态的线索）
    let query = crate::models::dto::crm_dto::LeadQuery {
        lead_status: Some("pool".to_string()),
        // 批次 111 P1-10：透传 source / keyword 到 LeadQuery，由 list_leads 服务执行过滤
        source: params.source,
        keyword: params.keyword,
        // v11 批次 153 P2-A：透传 industry 到 LeadQuery
        industry: params.industry,
        page: Some(page),
        page_size: Some(page_size),
    };

    let result = service.list_leads(query, None).await?;

    // 转换分页结果为列表
    let data = result
        .get("data")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let total = result
        .get("total")
        .and_then(|v| v.as_u64())
        .unwrap_or_default();

    // 转换为响应格式
    let items: Vec<serde_json::Value> = data
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|lead| {
            // lead 是 serde_json::Value，使用 .get("field") 访问
            let created_at_str = lead
                .get("created_at")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let days_in_pool = created_at_str
                .as_deref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| {
                    chrono::Utc::now()
                        .signed_duration_since(dt.with_timezone(&chrono::Utc))
                        .num_days()
                })
                .unwrap_or_default();

            serde_json::json!({
                "id": lead.get("id").cloned().unwrap_or(serde_json::Value::Null),
                "lead_no": lead.get("lead_no").cloned().unwrap_or(serde_json::Value::Null),
                "company_name": lead.get("company_name").cloned().unwrap_or(serde_json::Value::Null),
                "contact_name": lead.get("contact_name").cloned().unwrap_or(serde_json::Value::Null),
                "mobile_phone": lead.get("mobile_phone").cloned().unwrap_or(serde_json::Value::Null),
                "email": lead.get("email").cloned().unwrap_or(serde_json::Value::Null),
                "lead_source": lead.get("lead_source").cloned().unwrap_or(serde_json::Value::Null),
                "created_at": created_at_str,
                "days_in_pool": days_in_pool,
            })
        })
        .collect();

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// POST /api/v1/erp/crm/pool/claim - 从公海领取客户
pub async fn claim_from_pool(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<ClaimRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());

    // 获取线索
    let lead = service.get_lead(req.lead_id, None).await?;

    // 检查是否在公海中
    if lead.lead_status.as_deref() != Some("pool") {
        return Err(AppError::business("该客户不在公海中"));
    }

    // 更新线索归属人
    let update_req = crate::models::dto::crm_dto::UpdateLeadRequest {
        lead_status: Some("new".to_string()),
        ..Default::default()
    };

    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    let updated_lead = service
        .update_lead(req.lead_id, update_req, auth.user_id)
        .await?;

    // 记录领取日志
    tracing::info!(
        "用户 {} 从公海领取客户 {}: {}",
        auth.username,
        updated_lead.id,
        updated_lead.company_name.as_deref().unwrap_or("未知")
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(updated_lead)?,
        "客户领取成功",
    )))
}

/// POST /api/v1/erp/crm/pool/recycle - 回收客户到公海
pub async fn recycle_to_pool(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<RecycleRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());

    // 获取线索
    let lead = service.get_lead(req.lead_id, None).await?;

    // 检查状态
    if lead.lead_status.as_deref() == Some("pool") {
        return Err(AppError::business("该客户已在公海中"));
    }

    // 更新线索状态为公海
    let update_req = crate::models::dto::crm_dto::UpdateLeadRequest {
        lead_status: Some("pool".to_string()),
        ..Default::default()
    };

    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    let updated_lead = service
        .update_lead(req.lead_id, update_req, auth.user_id)
        .await?;

    // 记录回收日志
    tracing::info!(
        "用户 {} 回收客户 {} 到公海，原因: {:?}",
        auth.username,
        updated_lead.id,
        req.reason
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(updated_lead)?,
        "客户已回收到公海",
    )))
}

/// POST /api/v1/erp/crm/pool/:customer_id/claim - 领取指定公海客户
pub async fn claim_specific(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(customer_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let claimed = service
        .claim_pool_customers(vec![customer_id], auth.user_id, &auth.username)
        .await?;

    if claimed == 0 {
        return Err(AppError::business("该客户不在公海中或领取失败"));
    }

    tracing::info!("用户 {} 从公海领取客户 {}", auth.username, customer_id);

    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({ "claimed": claimed }),
        "客户领取成功",
    )))
}

/// POST /api/v1/erp/crm/pool/batch-claim - 批量领取公海客户
pub async fn batch_claim(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<BatchClaimRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let claimed = service
        .claim_pool_customers(req.customer_ids, auth.user_id, &auth.username)
        .await?;

    tracing::info!(
        "用户 {} 批量领取公海客户，成功 {} 条",
        auth.username,
        claimed
    );

    let msg = format!("成功领取 {} 个客户", claimed);
    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({ "claimed": claimed }),
        &msg,
    )))
}
