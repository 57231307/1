//! 付款申请 Handler
//!
//! 付款申请 HTTP 接口层，负责处理 HTTP 请求并调用 Service 层

use crate::middleware::auth_context::AuthContext;
use crate::models::supplier;
use crate::services::ap_payment_request_service::{
    ApPaymentRequestListQuery, ApPaymentRequestService, CreateApPaymentRequest,
    UpdateApPaymentRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDate;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tracing::{info, warn};
use validator::Validate;

/// 查询付款申请列表参数
#[derive(Debug, Deserialize)]
pub struct ApPaymentRequestQueryParams {
    pub supplier_id: Option<i32>,
    pub approval_status: Option<String>,
    pub payment_type: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 查询付款申请列表
pub async fn list_requests(
    Query(params): Query<ApPaymentRequestQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 查询付款申请列表，供应商 ID: {:?}",
        auth.username, params.supplier_id
    );

    let service = ApPaymentRequestService::new(state.db.clone());
    let page = params.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    // V15 P0-S01：提取行级数据权限上下文
    let data_scope_ctx = auth.to_data_scope_context();
    let (requests, total) = service
        .get_list(
            ApPaymentRequestListQuery {
                supplier_id: params.supplier_id,
                approval_status: params.approval_status,
                payment_type: params.payment_type,
                start_date: params.start_date,
                end_date: params.end_date,
                page,
                page_size,
            },
            Some(&data_scope_ctx),
        )
        .await?;

    info!(
        "用户 {} 查询付款申请成功，共 {} 条记录",
        auth.username, total
    );

    // 批次 406 修复：序列化失败应传播错误而非返回 Null
    let mut items_json: Vec<serde_json::Value> = requests
        .into_iter()
        .map(|r| serde_json::to_value(r).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        if let Ok(Some(permission)) = state
            .data_permission_service
            .get_role_data_permission(role_id, "ap_payment_request")
            .await
        {
            state.data_permission_service.filter_fields_batch(
                &mut items_json,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
        } else if role_id != 1 {
            // 如果没有配置数据权限且不是管理员，使用默认字段隐藏
            for request in &mut items_json {
                if let Some(obj) = request.as_object_mut() {
                    obj.remove("request_amount");
                    obj.remove("request_amount_foreign");
                    obj.remove("bank_account");
                    obj.remove("bank_name");
                }
            }
        }
    }

    let result = serde_json::to_value(PaginatedResponse::new(
        items_json,
        total,
        page,
        page_size,
    ))
    .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(ApiResponse::success(result)))
}

/// 获取付款申请详情
pub async fn get_request(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 查询付款申请详情 ID: {}", auth.username, id);

    let service = ApPaymentRequestService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();
    let request = service.get_by_id(id, Some(&data_scope_ctx)).await?;

    info!(
        "用户 {} 查询付款申请详情成功：{}",
        auth.username, request.request_no
    );

    let mut request_json = serde_json::to_value(request)?;

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        if let Ok(Some(permission)) = state
            .data_permission_service
            .get_role_data_permission(role_id, "ap_payment_request")
            .await
        {
            state.data_permission_service.filter_fields(
                &mut request_json,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
        } else if role_id != 1 {
            // 如果没有配置数据权限且不是管理员，使用默认字段隐藏
            if let Some(obj) = request_json.as_object_mut() {
                obj.remove("request_amount");
                obj.remove("request_amount_foreign");
                obj.remove("bank_account");
                obj.remove("bank_name");
            }
        }
    }

    Ok(Json(ApiResponse::success(request_json)))
}

/// 创建付款申请
#[axum::debug_handler]
pub async fn create_request(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateApPaymentRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 创建付款申请，供应商 ID: {}",
        auth.username, req.supplier_id
    );

    req.validate().map_err(|e| {
        warn!("用户 {} 创建付款申请验证失败：{}", auth.username, e);
        AppError::validation(e.to_string())
    })?;

    let service = ApPaymentRequestService::new(state.db.clone());
    let request = service.create(req, auth.user_id).await?;

    info!(
        "用户 {} 创建付款申请成功：{}",
        auth.username, request.request_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(request)?,
        "付款申请创建成功",
    )))
}

/// 更新付款申请
#[axum::debug_handler]
pub async fn update_request(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateApPaymentRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 更新付款申请 ID: {}", auth.username, id);

    req.validate().map_err(|e| {
        warn!("用户 {} 更新付款申请验证失败：{}", auth.username, e);
        AppError::validation(e.to_string())
    })?;

    let service = ApPaymentRequestService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——更新前先校验资源归属（复用 P0-S01 的 get_by_id + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_by_id(id, Some(&data_scope_ctx)).await?;

    let request = service.update(id, req, auth.user_id).await?;

    info!(
        "用户 {} 更新付款申请成功：{}",
        auth.username, request.request_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(request)?,
        "付款申请更新成功",
    )))
}

/// 删除付款申请
pub async fn delete_request(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 删除付款申请 ID: {}", auth.username, id);

    let service = ApPaymentRequestService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——删除前先校验资源归属（复用 P0-S01 的 get_by_id + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_by_id(id, Some(&data_scope_ctx)).await?;

    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    service.delete(id, auth.user_id).await?;

    info!("用户 {} 删除付款申请成功", auth.username);

    Ok(Json(ApiResponse::success_with_message(
        (),
        "付款申请删除成功",
    )))
}

/// 提交付款申请
pub async fn submit_request(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 提交付款申请 ID: {}", auth.username, id);

    let service = ApPaymentRequestService::new(state.db.clone());
    let request = service.submit(id, auth.user_id).await?;

    // 发送付款申请通知给审批人
    if let Some(ref event_service) = state.event_notification_service {
        let supplier_name = if let Ok(Some(sup)) = supplier::Entity::find_by_id(request.supplier_id)
            .one(&*state.db)
            .await
        {
            sup.supplier_name
        } else {
            String::new()
        };

        // 批次 114 P1-6：通知发送失败改 warn 日志（原 `let _ =` 静默吞错）
        if let Err(e) = event_service
            .notify_payment_request(
                auth.user_id,
                &request.request_no,
                &request.request_amount.to_string(),
                &supplier_name,
                request.id,
            )
            .await
        {
            tracing::warn!(error = %e, request_id = request.id, "付款申请提交通知发送失败");
        }
    }

    info!(
        "用户 {} 提交付款申请成功：{}",
        auth.username, request.request_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(request)?,
        "付款申请提交成功",
    )))
}

/// 审批付款申请
pub async fn approve_request(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 审批付款申请 ID: {}", auth.username, id);

    let service = ApPaymentRequestService::new(state.db.clone());
    let request = service.approve(id, auth.user_id).await?;

    // 发送审批通过通知
    if let Some(ref event_service) = state.event_notification_service {
        // 批次 114 P1-6：通知发送失败改 warn 日志（原 `let _ =` 静默吞错）
        if let Err(e) = event_service
            .notify_approval_result(
                request.created_by,
                &request.request_no,
                true,
                auth.user_id,
                &auth.username,
                None,
            )
            .await
        {
            tracing::warn!(error = %e, request_id = id, "付款申请审批通过通知发送失败");
        }
    }

    info!(
        "用户 {} 审批付款申请通过：{}",
        auth.username, request.request_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(request)?,
        "付款申请审批通过",
    )))
}

/// 拒绝付款申请
#[derive(Debug, Deserialize, Serialize)]
pub struct RejectRequest {
    pub reason: String,
}

pub async fn reject_request(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<RejectRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 拒绝付款申请 ID: {}, 原因：{}",
        auth.username, id, req.reason
    );

    let service = ApPaymentRequestService::new(state.db.clone());
    let request = service.reject(id, req.reason.clone(), auth.user_id).await?;

    // 发送审批拒绝通知
    if let Some(ref event_service) = state.event_notification_service {
        // 批次 114 P1-6：通知发送失败改 warn 日志（原 `let _ =` 静默吞错）
        if let Err(e) = event_service
            .notify_approval_result(
                request.created_by,
                &request.request_no,
                false,
                auth.user_id,
                &auth.username,
                Some(&req.reason),
            )
            .await
        {
            tracing::warn!(error = %e, request_id = id, "付款申请审批拒绝通知发送失败");
        }
    }

    info!(
        "用户 {} 拒绝付款申请成功：{}",
        auth.username, request.request_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(request)?,
        "付款申请已拒绝",
    )))
}
