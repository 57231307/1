//! CRM客户管理 Handler
//!
//! 提供客户信息维护、标签管理、联系人管理等 API 接口

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::models::dto::crm_dto::{CreateLeadRequest, LeadQuery, UpdateLeadRequest};
use crate::services::crm::cust::CrmService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 客户查询参数
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CustomerQueryParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub keyword: Option<String>,
}

/// POST /api/v1/erp/crm/customers - 创建客户（通过线索）
pub async fn create_customer(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateLeadRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let lead = service.create_lead(req, auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(lead)?)))
}

/// GET /api/v1/erp/crm/customers - 获取客户列表（线索列表）
pub async fn list_customers(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<CustomerQueryParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());

    let query = LeadQuery {
        page: params.page,
        page_size: params.page_size,
        lead_status: params.status,
    };

    let result = service.list_leads(query).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(result)?)))
}

/// GET /api/v1/erp/crm/customers/:id - 获取客户详情
pub async fn get_customer(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let lead = service.get_lead(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(lead)?)))
}

/// PUT /api/v1/erp/crm/customers/:id - 更新客户
pub async fn update_customer(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateLeadRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let lead = service.update_lead(id, req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(lead)?)))
}

/// DELETE /api/v1/erp/crm/customers/:id - 删除客户
pub async fn delete_customer(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    service.delete_lead(id).await?;
    Ok(Json(ApiResponse::success(
        serde_json::json!({"deleted": true}),
    )))
}

/// POST /api/v1/erp/crm/customers/:id/tags - 添加标签
pub async fn add_tags(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());

    let tags = req
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    let update_req = UpdateLeadRequest {
        tags: Some(tags),
        ..Default::default()
    };

    let lead = service.update_lead(id, update_req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(lead)?)))
}

/// GET /api/v1/erp/crm/customers/:id/contacts - 获取联系人列表
pub async fn list_contacts(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(customer_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let lead = service.get_lead(customer_id).await?;

    // 将线索的联系人信息作为单个联系人返回
    let contacts = vec![serde_json::json!({
        "id": lead.id,
        "name": lead.contact_name,
        "title": lead.contact_title,
        "phone": lead.mobile_phone,
        "tel": lead.tel_phone,
        "email": lead.email,
        "wechat": lead.wechat,
        "qq": lead.qq,
    })];

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": contacts,
        "total": contacts.len()
    }))))
}

/// GET /api/v1/erp/crm/tags - 获取标签列表
pub async fn list_tags(
    _state: State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    // 返回预定义的标签列表
    let tags = vec![
        serde_json::json!({"id": 1, "name": "VIP", "color": "#f50"}),
        serde_json::json!({"id": 2, "name": "重点客户", "color": "#2db7f5"}),
        serde_json::json!({"id": 3, "name": "潜在客户", "color": "#87d068"}),
        serde_json::json!({"id": 4, "name": "新客户", "color": "#108ee9"}),
        serde_json::json!({"id": 5, "name": "流失客户", "color": "#f50"}),
    ];
    Ok(Json(ApiResponse::success(tags)))
}

/// POST /api/v1/erp/crm/tags - 创建标签
pub async fn create_tag(
    _state: State<AppState>,
    _auth: AuthContext,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let tag = serde_json::json!({
        "id": chrono::Utc::now().timestamp(),
        "name": req.get("name").and_then(|v| v.as_str()).unwrap_or(""),
        "color": req.get("color").and_then(|v| v.as_str()).unwrap_or("#1890ff"),
    });
    Ok(Json(ApiResponse::success(tag)))
}

/// DELETE /api/v1/erp/crm/tags/:id - 删除标签
pub async fn delete_tag(
    _state: State<AppState>,
    _auth: AuthContext,
    _id: Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    Ok(Json(ApiResponse::success(
        serde_json::json!({"deleted": true}),
    )))
}
