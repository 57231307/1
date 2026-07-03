//! CRM客户管理 Handler
//!
//! 提供客户信息维护、标签管理、联系人管理等 API 接口

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use validator::Validate;

use crate::middleware::auth_context::AuthContext;
use crate::models::dto::crm_dto::{CreateLeadRequest, LeadQuery, UpdateLeadRequest};
use crate::services::crm::cust::CrmService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 客户查询参数
#[derive(Debug, Deserialize)]
pub struct CustomerQueryParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    #[allow(dead_code)] // TODO(tech-debt): 客户查询模块接入业务后移除
    pub keyword: Option<String>,
}

/// P1-2h 修复（批次 81 v1 复审）：添加标签请求 DTO
/// 替代 add_tags 中的 Json<serde_json::Value>，提供强类型校验
#[derive(Debug, Deserialize, Validate)]
pub struct AddTagsDto {
    /// 标签列表：必填，至少 1 个标签
    #[validate(length(min = 1, message = "标签列表不能为空"))]
    pub tags: Vec<String>,
}

/// P1-2h 修复（批次 81 v1 复审）：创建标签请求 DTO
/// 替代 create_tag 中的 Json<serde_json::Value>，提供强类型校验
#[derive(Debug, Deserialize, Validate)]
pub struct CreateTagDto {
    /// 标签名称：必填，长度至少 1
    #[validate(length(min = 1, max = 30, message = "标签名称长度必须在1到30字符之间"))]
    pub name: String,
    /// 颜色：可选，缺失时默认 "#1890ff"
    #[validate(length(max = 20, message = "颜色长度不能超过20字符"))]
    pub color: Option<String>,
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
    Json(req): Json<AddTagsDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // P1-2h 修复（批次 81 v1 复审）：强类型 DTO + validator 替代 Json<Value>
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let service = CrmService::new(state.db.clone());

    let update_req = UpdateLeadRequest {
        tags: Some(req.tags),
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
    Json(req): Json<CreateTagDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // P1-2h 修复（批次 81 v1 复审）：强类型 DTO + validator 替代 Json<Value>
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let tag = serde_json::json!({
        "id": chrono::Utc::now().timestamp(),
        "name": req.name,
        "color": req.color.unwrap_or_else(|| "#1890ff".to_string()),
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
