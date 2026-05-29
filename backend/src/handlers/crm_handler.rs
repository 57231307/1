use crate::middleware::auth_context::AuthContext;
use crate::models::dto::crm_dto::{
    ConvertLeadRequest, CreateLeadRequest, CreateOpportunityRequest, LeadQuery, OpportunityQuery,
    UpdateLeadRequest, UpdateOpportunityRequest,
};
use crate::services::crm_service::CrmService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};

pub async fn create_lead(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateLeadRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let res = service.create_lead(req, auth.user_id).await?;
    let value = serde_json::to_value(res)
        .map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success(value)))
}

pub async fn list_leads(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<LeadQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let res = service.list_leads(query).await?;
    let mut value = serde_json::to_value(res)
        .map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;

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
            // 如果没有配置数据权限且不是管理员，使用默认字段隐藏
            let mut list_opt = value.get_mut("list");
            if list_opt.is_none() {
                list_opt = value.get_mut("data");
            }
            if let Some(list) = list_opt.and_then(|v| v.as_array_mut()) {
                for lead in list {
                    if let Some(obj) = lead.as_object_mut() {
                        obj.remove("contact_phone");
                        obj.remove("email");
                        obj.remove("address");
                    }
                }
            }
        }
    }

    Ok(Json(ApiResponse::success(value)))
}

pub async fn get_lead(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let res = service.get_lead(id).await?;
    let mut value = serde_json::to_value(res)
        .map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;

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
            // 如果没有配置数据权限且不是管理员，使用默认字段隐藏
            if let Some(obj) = value.as_object_mut() {
                obj.remove("contact_phone");
                obj.remove("email");
                obj.remove("address");
            }
        }
    }

    Ok(Json(ApiResponse::success(value)))
}

pub async fn update_lead(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateLeadRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let res = service.update_lead(id, req).await?;
    let value = serde_json::to_value(res)
        .map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success(value)))
}

pub async fn delete_lead(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let service = CrmService::new(state.db.clone());
    service.delete_lead(id).await?;
    Ok(Json(ApiResponse::success("删除成功".to_string())))
}

pub async fn update_lead_status(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let status = payload
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("NEW");
    let service = CrmService::new(state.db.clone());
    service.update_lead_status(id, status).await?;
    Ok(Json(ApiResponse::success("状态更新成功".to_string())))
}

pub async fn create_opportunity(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateOpportunityRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let res = service.create_opportunity(req, auth.user_id).await?;
    let value = serde_json::to_value(res)
        .map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success(value)))
}

pub async fn list_opportunities(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<OpportunityQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let res = service.list_opportunities(query).await?;
    let mut value = serde_json::to_value(res)
        .map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;

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

pub async fn get_opportunity(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let res = service.get_opportunity(id).await?;
    let mut value = serde_json::to_value(res)
        .map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;

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
    Path(id): Path<i32>,
    Json(req): Json<UpdateOpportunityRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let res = service.update_opportunity(id, req).await?;
    let value = serde_json::to_value(res)
        .map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success(value)))
}

pub async fn delete_opportunity(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let service = CrmService::new(state.db.clone());
    service.delete_opportunity(id).await?;
    Ok(Json(ApiResponse::success("删除成功".to_string())))
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
    let value = serde_json::to_value(order)
        .map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success(value)))
}

/// Get lead relation info
pub async fn get_lead_relation(
    Path(lead_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let relation = service.get_lead_relation(lead_id).await?;
    let value = serde_json::to_value(relation)
        .map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;
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
    let value = serde_json::to_value(customer)
        .map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success(value)))
}

/// Get customer relation summary
pub async fn get_customer_relation_summary(
    Path(customer_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let summary = service.get_customer_relation_summary(customer_id).await?;
    let value = serde_json::to_value(summary)
        .map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success(value)))
}
