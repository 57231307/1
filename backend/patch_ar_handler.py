import re

with open('backend/src/handlers/ar_invoice_handler.rs', 'r') as f:
    content = f.read()

update_req = '''#[derive(Debug, Deserialize)]
pub struct CancelReason {
    pub reason: String,
}'''

if 'CancelReason' not in content:
    content = content.replace('pub struct ArInvoiceQuery {', update_req + '\n\n/// 查询参数\n#[derive(Debug, Deserialize)]\npub struct ArInvoiceQuery {')

new_handlers = '''
use axum::extract::Path;
use serde_json::Value as JsonValue;
use crate::services::ar_invoice_service::UpdateArInvoiceRequest;

/// 获取应收发票详情
pub async fn get_invoice(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = ArInvoiceService::new(state.db.clone());
    let invoice = service.get_by_id(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(invoice).map_err(|_| AppError::InternalError("序列化失败".into()))?)))
}

/// 更新应收发票
pub async fn update_invoice(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateArInvoiceRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = ArInvoiceService::new(state.db.clone());
    let invoice = service.update(id, req, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(invoice).map_err(|_| AppError::InternalError("序列化失败".into()))?,
        "应收发票更新成功",
    )))
}

/// 删除应收发票
pub async fn delete_invoice(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = ArInvoiceService::new(state.db.clone());
    service.delete(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message((), "应收发票删除成功")))
}

/// 审批应收发票
pub async fn approve_invoice(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = ArInvoiceService::new(state.db.clone());
    let invoice = service.approve(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(invoice).map_err(|_| AppError::InternalError("序列化失败".into()))?,
        "应收发票审批成功",
    )))
}

/// 取消应收发票
pub async fn cancel_invoice(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CancelReason>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = ArInvoiceService::new(state.db.clone());
    let invoice = service.cancel(id, req.reason, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(invoice).map_err(|_| AppError::InternalError("序列化失败".into()))?,
        "应收发票取消成功",
    )))
}
'''

if 'pub async fn get_invoice' not in content:
    content += new_handlers

with open('backend/src/handlers/ar_invoice_handler.rs', 'w') as f:
    f.write(content)
