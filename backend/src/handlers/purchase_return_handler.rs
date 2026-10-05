//! 采购退货 Handler
//!
//! 采购退货 HTTP 接口层

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::purchase_return_service::{
    CreatePurchaseReturnRequest, CreateReturnItemRequest, PurchaseReturnService,
    UpdatePurchaseReturnRequest, UpdateReturnItemRequest,
};
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use validator::Validate;

/// 查询采购退货单列表
pub async fn list_purchase_returns(
    Query(params): Query<ReturnQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文
    let data_scope_ctx = auth.to_data_scope_context();
    let page = params.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    let (returns, total) = service
        .list_returns(
            page,
            page_size,
            params.status,
            params.supplier_id,
            params.keyword,
            params.start_date,
            params.end_date,
            Some(&data_scope_ctx),
        )
        .await?;

    let result = serde_json::to_value(PaginatedResponse::new(returns, total, page, page_size))?;

    Ok(Json(ApiResponse::success(result)))
}

/// 获取采购退货单详情
pub async fn get_purchase_return(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();
    let return_order = service.get_return(id, Some(&data_scope_ctx)).await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(
        return_order,
    )?)))
}

/// 创建采购退货单
#[axum::debug_handler]
pub async fn create_purchase_return(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreatePurchaseReturnRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.validate()?;

    let service = PurchaseReturnService::new(state.db.clone());

    let return_order = service.create_return(req, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(return_order)?,
        "采购退货单创建成功",
    )))
}

/// 更新采购退货单
#[axum::debug_handler]
pub async fn update_purchase_return(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdatePurchaseReturnRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——更新前先校验资源归属（复用 P0-S01 的 get_return + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_return(id, Some(&data_scope_ctx)).await?;

    let return_order = service.update_return(id, req, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(return_order)?,
        "采购退货单更新成功",
    )))
}

/// 提交采购退货单
pub async fn submit_purchase_return(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());

    let return_order = service.submit_return(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(return_order)?,
        "采购退货单已提交",
    )))
}

/// 审批采购退货单
// 审批拆为「通过 / 拒绝」两条动作：本端点只处理通过，通过理由选填并落
// approval_reason 专列；拒绝动作在 /reject（理由落 rejected_reason 专列）。
pub async fn approve_purchase_return(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    payload: Option<Json<ApproveReturnRequest>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());

    // 入参形态用 `Option<Json<T>>`（本仓先例：sales_price_handler.rs approve_price）：
    // 现存不带 body 的调用方（前端 api/purchase-return.ts 批准不发体）若改强类型
    // `Json<T>` 会在 axum 解码层收到无信封 400，属破坏性变更；缺体/缺键在选填档
    // 直接按未采集放行。选填口径：空/纯空白一律归一为 None ⇒ 列保持 NULL。
    let approval_reason = payload
        .map(|Json(req)| req)
        .and_then(|r| r.approval_reason)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let return_order = service
        .approve_return(id, auth.user_id, approval_reason)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(return_order)?,
        "采购退货单已审批",
    )))
}

/// 拒绝采购退货单
#[axum::debug_handler]
pub async fn reject_purchase_return(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<RejectReturnRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 拒绝理由服务端必填：校验真实执行（本 DTO 此前无 Validate，两端皆空可入库）；
    // validator 的 min=1 拦不住纯空白，trim 非空门在后，落库为 trim 后的值
    // （口径同 quotation_handler.rs / sales_price_handler.rs reject 先例）。
    req.validate()?;
    let reason = req.reason.trim().to_string();
    if reason.is_empty() {
        tracing::warn!(
            "用户 {} 拒绝采购退货单被拒：单据 ID {id} 拒绝理由为纯空白（ID 只进日志不进文案）",
            auth.user_id
        );
        return Err(AppError::validation_displayable("审批拒绝理由不能为空"));
    }

    let service = PurchaseReturnService::new(state.db.clone());

    let return_order = service
        .reject_return(id, reason.clone(), auth.user_id)
        .await?;

    // 发送审批拒绝通知
    if state.event_notification_service.is_none() {
        tracing::error!("事件通知服务未装配（container 应无条件构造），此处站内通知将缺失");
    }
    if let Some(ref event_service) = state.event_notification_service {
        if let Some(created_by) = return_order.created_by {
            // 批次 114 P1-6：通知发送失败改 warn 日志（原 `let _ =` 静默吞错）
            if let Err(e) = event_service
                .notify_approval_result(
                    created_by,
                    &return_order.return_no,
                    false,
                    auth.user_id,
                    &auth.username,
                    Some(&reason),
                )
                .await
            {
                tracing::warn!(error = %e, return_id = id, "采购退货审批拒绝通知发送失败");
            }
        }
    }

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(return_order)?,
        "采购退货单已拒绝",
    )))
}

/// 删除采购退货单
pub async fn delete_purchase_return(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——删除前先校验资源归属（复用 P0-S01 的 get_return + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_return(id, Some(&data_scope_ctx)).await?;
    // 批次 101 v6 复审 P2-4：透传操作人 user_id 用于审计日志
    service.delete(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        (),
        "采购退货单已删除",
    )))
}

// =====================================================
// 请求 DTO
// =====================================================

/// 采购退货单查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ReturnQueryParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
    /// 关键字：匹配退货单号
    pub keyword: Option<String>,
    /// 退货日期范围下界（ISO / YYYY-MM-DD）
    pub start_date: Option<String>,
    /// 退货日期范围上界（ISO / YYYY-MM-DD）
    pub end_date: Option<String>,
}

/// 拒绝退货单请求：拒绝理由全域必填（handler 内 trim 非空门收口，min=1 只拦空串）。
/// purchase_return.rejected_reason 为 TEXT（business/m0009 建列），无列宽截断风险，
/// 不设上限——上限只在列型有容量约束时才有意义（对照 PO 的 VARCHAR(255)→255）。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct RejectReturnRequest {
    #[validate(length(min = 1, message = "拒绝原因不能为空"))]
    pub reason: String,
}

/// 审批通过请求体（选填档）：通过理由缺失/空串/纯空白一律按未采集落 NULL。
/// 字段保持 `Option<String>` 是为了让"缺键/不带 body"的调用与选填语义在同一
/// 形态下解码通过，不产生解码层裸 400。
#[derive(Debug, Deserialize)]
pub struct ApproveReturnRequest {
    pub approval_reason: Option<String>,
}

pub async fn list_purchase_return_items(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<
    Json<ApiResponse<Vec<crate::services::purchase_return_service::PurchaseReturnItemDto>>>,
    AppError,
> {
    let service = PurchaseReturnService::new(state.db.clone());
    let items = service.list_items(id).await?;
    Ok(Json(ApiResponse::success(items)))
}

pub async fn create_purchase_return_item(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<CreateReturnItemRequest>,
) -> Result<Json<ApiResponse<crate::models::purchase_return_item::Model>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    // 批次 101 v6 复审 P2-5：透传操作人 user_id 给 update_return_totals 用于审计日志
    let item = service.create_item(id, req, auth.user_id).await?;
    Ok(Json(ApiResponse::success(item)))
}

pub async fn update_purchase_return_item(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((_id, item_id)): Path<(i32, i32)>,
    Json(req): Json<UpdateReturnItemRequest>,
) -> Result<Json<ApiResponse<crate::models::purchase_return_item::Model>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    // 批次 101 v6 复审 P2-3：透传操作人 user_id 用于审计日志
    let item = service.update_item(item_id, req, auth.user_id).await?;
    Ok(Json(ApiResponse::success(item)))
}

pub async fn delete_purchase_return_item(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((_id, item_id)): Path<(i32, i32)>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = PurchaseReturnService::new(state.db.clone());
    // 批次 101 v6 复审 P2-5：透传操作人 user_id 给 update_return_totals 用于审计日志
    service.delete_item(item_id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(())))
}
