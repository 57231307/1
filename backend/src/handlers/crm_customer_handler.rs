//! CRM客户管理 Handler
//!
//! 提供客户信息维护、标签管理、联系人管理等 API 接口

use axum::{
    Json,
    extract::{Path, Query, State},
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::Deserialize;
use validator::Validate;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::crm_tag;
use crate::models::dto::crm_dto::{CreateLeadRequest, LeadQuery, UpdateLeadRequest};
use crate::services::crm::cust::CrmService;
use crate::services::customer_service::{
    CreateCustomerContactRequest, CustomerService, UpdateCustomerArgs, UpdateCustomerContactRequest,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 客户查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CustomerQueryParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    // 批次 111 P1-10：keyword 接入 LeadQuery 模糊搜索（移除 dead_code 标注）
    pub keyword: Option<String>,
}

/// P1-2h 修复（批次 81 v1 复审）：添加标签请求 DTO
/// 替代 add_tags 中的 Json<serde_json::Value>，提供强类型校验
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct AddTagsDto {
    /// 标签列表：必填，至少 1 个标签
    #[validate(length(min = 1, message = "标签列表不能为空"))]
    pub tags: Vec<String>,
}

/// P1-2h 修复（批次 81 v1 复审）：创建标签请求 DTO
/// 批次 122 v8 复审 P1 修复：增加 category 字段，真实持久化到 crm_tag 表
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct CreateTagDto {
    /// 标签名称：必填，长度至少 1
    #[validate(length(min = 1, max = 30, message = "标签名称长度必须在1到30字符之间"))]
    pub name: String,
    /// 颜色：可选，缺失时默认 "#1890ff"
    #[validate(length(max = 20, message = "颜色长度不能超过20字符"))]
    pub color: Option<String>,
    /// 标签分类：可选，如 customer/lead/supplier
    #[validate(length(max = 50, message = "分类长度不能超过50字符"))]
    pub category: Option<String>,
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
    auth: AuthContext,
    Query(params): Query<CustomerQueryParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());

    let query = LeadQuery {
        page: params.page,
        page_size: params.page_size,
        lead_status: params.status,
        // 批次 111 P1-10：透传 keyword 到 LeadQuery，由 list_leads 服务执行模糊搜索
        source: None,
        keyword: params.keyword,
        // v11 批次 153 P2-A：industry 字段新增，此处不过滤
        industry: None,
    };

    // V15 P0-S01：提取行级数据权限上下文
    let data_scope_ctx = auth.to_data_scope_context();
    let result = service.list_leads(query, Some(&data_scope_ctx)).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(result)?)))
}

/// GET /api/v1/erp/crm/customers/:id - 获取客户详情
pub async fn get_customer(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();
    let lead = service.get_lead(id, Some(&data_scope_ctx)).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(lead)?)))
}

/// CRM 增强客户更新请求 DTO：客户编辑弹窗提交字段，含状态（active/inactive）
/// 走客户域 CustomerService 更新链路落库
#[derive(Debug, serde::Deserialize)]
pub struct UpdateEnhancedCustomerRequest {
    pub customer_name: Option<String>,
    pub contact_person: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub address: Option<String>,
    pub customer_type: Option<String>,
    /// 税号（前端字段名 tax_number → 后端 tax_id）
    pub tax_number: Option<String>,
    pub credit_limit: Option<rust_decimal::Decimal>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    /// 客户状态（active/inactive）——31c 停用矩阵的核心字段
    pub status: Option<String>,
}

/// PUT /api/v1/erp/crm/customers/enhanced/:id - 更新客户（增强路由，客户域落库含 status）
pub async fn update_customer(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateEnhancedCustomerRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let customer_service = CustomerService::new(state.db.clone(), state.search_client.clone());

    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    let customer = customer_service
        .update_customer(UpdateCustomerArgs {
            customer_id: id,
            customer_name: req.customer_name,
            contact_person: req.contact_person,
            contact_phone: req.contact_phone,
            contact_email: req.contact_email,
            address: req.address,
            city: None,
            province: None,
            postal_code: None,
            credit_limit: req.credit_limit,
            payment_terms: None,
            tax_id: req.tax_number,
            bank_name: req.bank_name,
            bank_account: req.bank_account,
            customer_type: req.customer_type,
            status: req.status,
            country: None,
            customer_industry: None,
            main_products: None,
            annual_purchase: None,
            quality_requirement: None,
            inspection_standard: None,
            notes: None,
            user_id: auth.user_id,
        })
        .await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(customer)?)))
}

/// DELETE /api/v1/erp/crm/customers/:id - 删除客户
pub async fn delete_customer(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    service.delete_lead(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(
        serde_json::json!({"deleted": true}),
    )))
}

/// POST /api/v1/erp/crm/customers/:id/tags - 添加标签
pub async fn add_tags(
    State(state): State<AppState>,
    auth: AuthContext,
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

    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    let lead = service.update_lead(id, update_req, auth.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(lead)?)))
}

/// GET /api/v1/erp/crm/customers/:id/contacts - 获取联系人列表；批次 90b
/// P2-12：原实现从 crm_lead 拼接 JSON 伪联系人，改为查 customer_contacts 表真实数据。
pub async fn list_contacts(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(customer_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CustomerService::new(state.db.clone(), state.search_client.clone());
    let contacts = service.list_customer_contacts(customer_id).await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(contacts)?)))
}

/// POST /api/v1/erp/crm/customers/:id/contacts - 创建联系人；批次 90b P2-12：实现前端 detail.vue "新增联系人" 占位符的真实后端。
#[axum::debug_handler]
pub async fn create_contact(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(customer_id): Path<i32>,
    Json(req): Json<CreateCustomerContactRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let service = CustomerService::new(state.db.clone(), state.search_client.clone());
    let contact = service
        .create_customer_contact(customer_id, req, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(contact)?,
        "联系人创建成功",
    )))
}

/// PUT /api/v1/erp/crm/customers/:id/contacts/:contact_id - 更新联系人；批次 90b P2-12：实现联系人编辑功能。
#[axum::debug_handler]
pub async fn update_contact(
    Path((_customer_id, contact_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateCustomerContactRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let service = CustomerService::new(state.db.clone(), state.search_client.clone());
    let contact = service
        .update_customer_contact(contact_id, req, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(contact)?,
        "联系人更新成功",
    )))
}

/// DELETE /api/v1/erp/crm/customers/:id/contacts/:contact_id - 删除联系人；批次 90b P2-12：实现联系人删除功能。
pub async fn delete_contact(
    Path((_customer_id, contact_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = CustomerService::new(state.db.clone(), state.search_client.clone());
    service.delete_customer_contact(contact_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        (),
        "联系人删除成功",
    )))
}

/// GET /api/v1/erp/crm/tags - 获取标签列表；批次 122 v8 复审 P1 修复：原返回硬编码 5 个标签，改为查 crm_tag 表真实数据。
pub async fn list_tags(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<crm_tag::Model>>>, AppError> {
    let tags = crm_tag::Entity::find().all(&*state.db).await?;
    Ok(Json(ApiResponse::success(tags)))
}

/// POST /api/v1/erp/crm/tags - 创建标签；批次 122 v8 复审 P1 修复：原 create_tag 仅用时间戳生成假 id
/// 返回，不持久化。 现真实 INSERT 到 crm_tag 表，返回含 id/name/color/category/created_at 的完整记录。
pub async fn create_tag(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateTagDto>,
) -> Result<Json<ApiResponse<crm_tag::Model>>, AppError> {
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let new_tag = crm_tag::ActiveModel {
        name: Set(req.name),
        color: Set(req.color.unwrap_or_else(|| "#1890ff".to_string())),
        category: Set(req.category),
        created_by: Set(Some(auth.user_id)),
        ..Default::default()
    };

    let tag = new_tag.insert(&*state.db).await?;
    Ok(Json(ApiResponse::success_with_message(tag, "标签创建成功")))
}

/// DELETE /api/v1/erp/crm/tags/:id - 删除标签；批次 122 v8 复审 P1 修复：原 delete_tag 直接返回
/// {"deleted": true} 空操作。 现真实 DELETE FROM crm_tag WHERE id = ?，标签不存在时返回 404。
pub async fn delete_tag(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let result = crm_tag::Entity::delete_by_id(id).exec(&*state.db).await?;

    if result.rows_affected == 0 {
        return Err(AppError::not_found(format!("标签 {} 未找到", id)));
    }

    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({"deleted": true, "id": id}),
        "标签删除成功",
    )))
}

/// GET /api/v1/erp/crm/customers/:id/tags - 获取客户已挂载标签对象列表
/// 单次 JOIN 查询（customer_tag INNER JOIN crm_tag），无 N+1。
pub async fn list_customer_tags(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<crate::services::crm::CustomerTagBrief>>>, AppError> {
    use sea_orm::{ColumnTrait, QueryFilter, QueryOrder};

    let tags: Vec<crate::services::crm::CustomerTagBrief> = crm_tag::Entity::find()
        .inner_join(crate::models::customer_tag::Entity)
        .filter(crate::models::customer_tag::Column::CustomerId.eq(id))
        .order_by(crm_tag::Column::Id, sea_orm::Order::Asc)
        .into_model::<crate::services::crm::CustomerTagBrief>()
        .all(&*state.db)
        .await?;

    Ok(Json(ApiResponse::success(tags)))
}

/// POST /api/v1/erp/crm/customers/:id/tags/:tagId - 给客户挂载标签（幂等）
/// 如果关联已存在，不报错直接返回成功。
pub async fn attach_customer_tag(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((customer_id, tag_id)): Path<(i32, i32)>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use crate::models::customer_tag;
    use sea_orm::{ColumnTrait, QueryFilter};

    // 校验客户存在
    let _customer = crate::models::customer::Entity::find_by_id(customer_id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", customer_id)))?;

    // 校验标签存在
    let _tag = crm_tag::Entity::find_by_id(tag_id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found(format!("标签 {} 不存在", tag_id)))?;

    // 幂等检查：已存在则直接返回成功
    let existing = customer_tag::Entity::find()
        .filter(customer_tag::Column::CustomerId.eq(customer_id))
        .filter(customer_tag::Column::TagId.eq(tag_id))
        .one(&*state.db)
        .await?;

    if existing.is_some() {
        return Ok(Json(ApiResponse::success_with_message(
            serde_json::json!({"customer_id": customer_id, "tag_id": tag_id, "already_exists": true}),
            "标签已关联",
        )));
    }

    let new_rel = customer_tag::ActiveModel {
        customer_id: Set(customer_id),
        tag_id: Set(tag_id),
        created_by: Set(Some(auth.user_id)),
        ..Default::default()
    };
    new_rel.insert(&*state.db).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({"customer_id": customer_id, "tag_id": tag_id}),
        "标签关联成功",
    )))
}

/// DELETE /api/v1/erp/crm/customers/:id/tags/:tagId - 解除客户标签关联
pub async fn detach_customer_tag(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path((customer_id, tag_id)): Path<(i32, i32)>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use crate::models::customer_tag;
    use sea_orm::{ColumnTrait, QueryFilter};

    let result = customer_tag::Entity::delete_many()
        .filter(customer_tag::Column::CustomerId.eq(customer_id))
        .filter(customer_tag::Column::TagId.eq(tag_id))
        .exec(&*state.db)
        .await?;

    if result.rows_affected == 0 {
        return Err(AppError::not_found(format!(
            "客户 {} 与标签 {} 的关联不存在",
            customer_id, tag_id
        )));
    }

    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({"customer_id": customer_id, "tag_id": tag_id, "deleted": true}),
        "标签解除成功",
    )))
}
