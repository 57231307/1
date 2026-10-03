use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 核销查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ArVerificationQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub invoice_id: Option<i32>,
    pub payment_id: Option<i32>,
    pub status: Option<String>,
}

/// 手动核销请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ManualVerifyRequest {
    pub invoice_id: i32,
    pub payment_id: i32,
    pub amount: rust_decimal::Decimal,
    pub remark: Option<String>,
}

/// 获取核销列表
/// GET /api/v1/erp/ar/verifications
pub async fn list_verifications(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ArVerificationQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let page = query.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    // v12 批次 39 修复：page_size clamp(1,100) 防 DoS（即便 service 当前为空实现，前置防护避免未来埋雷）
    let page_size = query.page_size.unwrap_or(10).clamp(1, 100);
    // V15 P0-S01：提取行级数据权限上下文
    let data_scope_ctx = auth.to_data_scope_context();

    // service 已返回 AppError，直接 ? 传播真实 status/code（脱敏转 500 会把 4xx 业务/权限错伪装成内部错误）
    let (verifications, total) = service
        .list_verifications(
            page,
            page_size,
            query.invoice_id,
            query.payment_id,
            query.status,
            Some(&data_scope_ctx),
        )
        .await?;

    let result = serde_json::json!({
        "list": verifications,
        "total": total,
        "page": page,
        "page_size": page_size,
    });

    Ok(Json(ApiResponse::success(result)))
}

/// 获取核销详情
/// GET /api/v1/erp/ar/verifications/:id
pub async fn get_verification(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();

    let verification = service.get_verification(id, Some(&data_scope_ctx)).await?;

    Ok(Json(ApiResponse::success(verification)))
}

/// 自动核销
/// POST /api/v1/erp/ar/verifications/auto
pub async fn auto_verify(
    auth: AuthContext,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let result = service.auto_verify(auth.user_id).await?;

    Ok(Json(ApiResponse::success(result)))
}

/// 手动核销
/// POST /api/v1/erp/ar/verifications/manual
pub async fn manual_verify(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(payload): Json<ManualVerifyRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let verification = service
        .manual_verify(
            payload.invoice_id,
            payload.payment_id,
            payload.amount,
            payload.remark,
            auth.user_id,
        )
        .await?;

    Ok(Json(ApiResponse::success(verification)))
}

/// 取消核销
/// POST /api/v1/erp/ar/verifications/:id/cancel
pub async fn cancel_verification(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let result = service.cancel_verification(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success(result)))
}

/// 未核销单据候选列表查询参数（typed DTO）
///
/// 形态与 `sales_order_handler::OrderStatisticsQuery` / `budget_management_handler::BudgetListQuery` 一致：
/// Query 泛参曾直接是 `serde_json::Value`，urlencoded 反序列化下所有值恒为 `Value::String`，
/// service 端 `as_i64()` 恒 `None` ⇒ customer_id 筛选静默失效（勾了客户仍返回全部单据）。
/// typed DTO 由 serde 在反序列化边界完成字符串→整数转换（非法值直接 400，不做任何
/// `unwrap_or` 静默回落），再按 service 既有契约键 `customer_id` 以 `Value::Number` 重建透传，
/// 不改 service 签名、不新造键名。空串筛选由最外层 `normalize_empty_query_params`
/// 中间件剔除后收敛为 None（不过滤），与全仓查询 DTO 边界语义一致。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct UnverifiedDocsQuery {
    pub customer_id: Option<i64>,
}

/// 按 service 既有契约重建透传 json：仅在提供了 customer_id 时写入数字型 `customer_id` 键，
/// 缺省返回空对象（service 侧 `query.get("customer_id")` 为 None 即不过滤）。
fn build_unverified_docs_params(q: UnverifiedDocsQuery) -> serde_json::Value {
    let mut params = serde_json::Map::new();
    if let Some(v) = q.customer_id {
        params.insert("customer_id".to_string(), serde_json::Value::from(v));
    }
    serde_json::Value::Object(params)
}

/// 获取未核销发票
/// GET /api/v1/erp/ar/verifications/unverified/invoices
pub async fn get_unverified_invoices(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(q): Query<UnverifiedDocsQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let invoices = service
        .get_unverified_invoices(build_unverified_docs_params(q))
        .await?;

    Ok(Json(ApiResponse::success(invoices)))
}

/// 获取未核销收款
/// GET /api/v1/erp/ar/verifications/unverified/payments
pub async fn get_unverified_payments(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(q): Query<UnverifiedDocsQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let payments = service
        .get_unverified_payments(build_unverified_docs_params(q))
        .await?;

    Ok(Json(ApiResponse::success(payments)))
}
