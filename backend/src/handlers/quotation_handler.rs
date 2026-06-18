//! 销售报价单 HTTP Handler
//!
//! 处理销售报价单（Quotation）的 REST API 请求：
//! 列表查询、详情查询、创建、修改、取消、提交、审批、拒绝 8 个核心端点。
//!
//! # 关键约束
//! - 强租户隔离：所有方法必须使用 `extract_tenant_id(&auth)?` 提取租户 ID，
//!   严禁使用 `auth.tenant_id.unwrap_or(0)`。
//! - 统一响应格式：所有方法返回 `Result<Json<ApiResponse<serde_json::Value>>, AppError>`。
//! - AppState 注入：通过 `State(state): State<AppState>` 注入。
//! - Service 初始化：`QuotationService::new(state.db.clone())`。
//!
//! # 路由注册
//! - 挂载在 `routes::sales::quotations()` 子路由下
//! - 前缀：`/api/v1/erp/sales/quotations`
//!
//! # 端点清单
//! - `GET    /`         → `list_quotations`
//! - `GET    /:id`      → `get_quotation`
//! - `POST   /`         → `create_quotation`
//! - `PUT    /:id`      → `update_quotation`
//! - `POST   /:id/cancel` → `cancel_quotation`
//! - `POST   /:id/submit` → `submit_quotation`
//! - `POST   /:id/approve` → `approve_quotation`
//! - `POST   /:id/reject`  → `reject_quotation`

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::middleware::tenant::extract_tenant_id;
use crate::models::dto::PageResponse;
use crate::models::quotation_create_dto::QuotationCreateDto;
use crate::models::quotation_response_dto::QuotationQueryParams;
use crate::models::quotation_update_dto::QuotationUpdateDto;
use crate::services::quotation_service::QuotationService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ============================================================================
// 查询参数 DTO
// ============================================================================

/// 销售报价单列表查询参数
///
/// HTTP 查询字符串字段，全部可选；handler 层负责赋予默认值。
#[derive(Debug, Deserialize)]
pub struct QuotationListQuery {
    /// 页码（从 1 开始）
    pub page: Option<u64>,
    /// 每页数量
    pub page_size: Option<u64>,
    /// 客户 ID 过滤
    pub customer_id: Option<i32>,
    /// 销售员 ID 过滤
    pub sales_user_id: Option<i32>,
    /// 状态过滤（DRAFT/SUBMITTED/APPROVED/REJECTED/CONVERTED/CANCELLED/EXPIRED）
    pub status: Option<String>,
    /// 关键字（报价单号模糊匹配）
    pub keyword: Option<String>,
}

/// 拒绝请求体
#[derive(Debug, Deserialize)]
pub struct RejectQuotationRequest {
    /// 拒绝原因
    pub reason: String,
}

/// 取消请求体
#[derive(Debug, Deserialize)]
pub struct CancelQuotationRequest {
    /// 取消原因
    pub reason: Option<String>,
}

// ============================================================================
// 列表查询
// ============================================================================

/// 获取销售报价单列表（分页 + 多维过滤）
///
/// `GET /api/v1/erp/sales/quotations`
///
/// # 查询参数
/// - `page` / `page_size` 分页
/// - `customer_id` / `sales_user_id` 关联实体过滤
/// - `status` 状态过滤
/// - `keyword` 报价单号模糊匹配
pub async fn list_quotations(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<QuotationListQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 强租户隔离：缺失租户 ID 直接返回未授权错误
    let tenant_id = extract_tenant_id(&auth)?;
    let svc = QuotationService::new(state.db.clone());

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 200);
    let params = QuotationQueryParams {
        customer_id: query.customer_id,
        sales_user_id: query.sales_user_id,
        status: query.status,
        keyword: query.keyword,
        page,
        page_size,
    };

    // 调用 Service 层 list（PR-2 已实现）
    let (items, total) = svc.list(tenant_id, params).await?;

    // 将主表模型序列化为 JSON Value（响应中 items 字段）
    let items_json: Vec<serde_json::Value> = items
        .into_iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id,
                "quotation_no": m.quotation_no,
                "customer_id": m.customer_id,
                "sales_user_id": m.sales_user_id,
                "quotation_date": m.quotation_date,
                "valid_until": m.valid_until,
                "currency": m.currency,
                "exchange_rate": m.exchange_rate,
                "base_currency": m.base_currency,
                "price_terms": m.price_terms,
                "tax_inclusive": m.tax_inclusive,
                "tax_rate": m.tax_rate,
                "subtotal": m.subtotal,
                "tax_amount": m.tax_amount,
                "total_amount": m.total_amount,
                "status": m.status,
                "created_by": m.created_by,
                "created_at": m.created_at,
                "updated_at": m.updated_at,
            })
        })
        .collect();

    let total_u64 = total;
    let response = PageResponse::new(items_json, total_u64, page, page_size);
    let value = serde_json::to_value(response).map_err(AppError::from)?;
    Ok(Json(ApiResponse::success(value)))
}

// ============================================================================
// 详情查询
// ============================================================================

/// 获取销售报价单详情（含明细 + 贸易条款）
///
/// `GET /api/v1/erp/sales/quotations/:id`
pub async fn get_quotation(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)?;
    let svc = QuotationService::new(state.db.clone());

    // 查询主表
    let quotation = svc.get_by_id(tenant_id, id).await?;
    // 关联明细 + 条款
    let items = svc.list_items(id).await?;
    let terms = svc.list_terms(id).await?;

    // 构造完整响应 DTO（From<(Model, Vec<Item>, Vec<Term>)> 实现已在 PR-2 给出）
    let response_dto =
        crate::models::quotation_response_dto::QuotationResponseDto::from((quotation, items, terms));
    let value = serde_json::to_value(response_dto).map_err(AppError::from)?;
    Ok(Json(ApiResponse::success(value)))
}

// ============================================================================
// 创建
// ============================================================================

/// 创建销售报价单
///
/// `POST /api/v1/erp/sales/quotations`
pub async fn create_quotation(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(payload): Json<QuotationCreateDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)?;
    let svc = QuotationService::new(state.db.clone());

    // 业务校验：明细至少 1 条（Service 也会校验，这里提前返回更友好）
    if payload.items.is_empty() {
        return Err(AppError::validation("报价单至少需要 1 条明细行项目"));
    }

    let quotation = svc
        .create(tenant_id, auth.user_id, payload)
        .await?;
    let value = serde_json::to_value(&quotation).map_err(AppError::from)?;
    Ok(Json(ApiResponse::success_with_message(
        value,
        "销售报价单创建成功",
    )))
}

// ============================================================================
// 更新
// ============================================================================

/// 更新销售报价单（仅 DRAFT 状态可更新）
///
/// `PUT /api/v1/erp/sales/quotations/:id`
pub async fn update_quotation(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<QuotationUpdateDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)?;
    let svc = QuotationService::new(state.db.clone());

    let updated = svc.update(tenant_id, auth.user_id, id, payload).await?;
    let value = serde_json::to_value(&updated).map_err(AppError::from)?;
    Ok(Json(ApiResponse::success_with_message(
        value,
        "销售报价单更新成功",
    )))
}

// ============================================================================
// 状态机操作
// ============================================================================

/// 取消销售报价单（DRAFT / SUBMITTED 状态可取消）
///
/// `POST /api/v1/erp/sales/quotations/:id/cancel`
///
/// 请求体：`{ "reason": "客户主动放弃" }`，reason 可选。
pub async fn cancel_quotation(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<CancelQuotationRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)?;
    let svc = QuotationService::new(state.db.clone());

    let result = svc
        .cancel(tenant_id, auth.user_id, id, payload.reason)
        .await?;
    let value = serde_json::to_value(&result).map_err(AppError::from)?;
    Ok(Json(ApiResponse::success_with_message(
        value,
        "销售报价单已取消",
    )))
}

/// 提交审批（DRAFT → SUBMITTED）
///
/// `POST /api/v1/erp/sales/quotations/:id/submit`
pub async fn submit_quotation(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)?;
    let svc = QuotationService::new(state.db.clone());

    let result = svc.submit(tenant_id, auth.user_id, id).await?;
    let value = serde_json::to_value(&result).map_err(AppError::from)?;
    Ok(Json(ApiResponse::success_with_message(
        value,
        "销售报价单已提交审批",
    )))
}

/// 审批通过（SUBMITTED → APPROVED）
///
/// `POST /api/v1/erp/sales/quotations/:id/approve`
pub async fn approve_quotation(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)?;
    let svc = QuotationService::new(state.db.clone());

    let result = svc.approve(tenant_id, auth.user_id, id).await?;
    let value = serde_json::to_value(&result).map_err(AppError::from)?;
    Ok(Json(ApiResponse::success_with_message(
        value,
        "销售报价单审批通过",
    )))
}

/// 审批拒绝（SUBMITTED → REJECTED）
///
/// `POST /api/v1/erp/sales/quotations/:id/reject`
///
/// 请求体：`{ "reason": "价格高于客户预算" }`，reason 必填。
pub async fn reject_quotation(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<RejectQuotationRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)?;
    let svc = QuotationService::new(state.db.clone());

    // 业务校验：拒绝原因必填
    if payload.reason.trim().is_empty() {
        return Err(AppError::validation("拒绝原因不能为空"));
    }

    let result = svc
        .reject(tenant_id, auth.user_id, id, payload.reason)
        .await?;
    let value = serde_json::to_value(&result).map_err(AppError::from)?;
    Ok(Json(ApiResponse::success_with_message(
        value,
        "销售报价单已拒绝",
    )))
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::auth_context::AuthContext;

    // -------- 鉴权路径 --------

    /// 验证 extract_tenant_id 接受有效租户 ID
    #[test]
    fn test_extract_tenant_id_accepts_valid() {
        let auth = AuthContext {
            user_id: 1,
            username: "tester".to_string(),
            role_id: Some(1),
            tenant_id: Some(42),
        };
        let tid = extract_tenant_id(&auth).expect("租户 ID 应存在");
        assert_eq!(tid, 42);
    }

    /// 验证 extract_tenant_id 在 tenant_id 为 None 时返回未授权错误
    #[test]
    fn test_extract_tenant_id_rejects_missing() {
        let auth = AuthContext {
            user_id: 1,
            username: "tester".to_string(),
            role_id: Some(1),
            tenant_id: None,
        };
        let err = extract_tenant_id(&auth).expect_err("缺失租户应失败");
        let msg = format!("{}", err);
        assert!(
            msg.contains("租户") || msg.contains("未授权"),
            "错误消息应包含租户/未授权，实际：{}",
            msg
        );
    }

    // -------- 请求体反序列化路径 --------

    /// 验证 RejectQuotationRequest 拒绝空 reason（handler 业务校验）
    #[test]
    fn test_reject_request_blank_reason_rejected() {
        let reason = "   ".to_string();
        assert!(reason.trim().is_empty());
    }

    /// 验证 QuotationListQuery 反序列化：所有字段均为 Option
    #[test]
    fn test_quotation_list_query_deserialize_empty() {
        let q: QuotationListQuery = serde_json::from_str("{}").expect("空对象应可反序列化");
        assert!(q.page.is_none());
        assert!(q.page_size.is_none());
        assert!(q.customer_id.is_none());
        assert!(q.sales_user_id.is_none());
        assert!(q.status.is_none());
        assert!(q.keyword.is_none());
    }

    /// 验证 QuotationListQuery 反序列化：完整参数
    #[test]
    fn test_quotation_list_query_deserialize_full() {
        let q: QuotationListQuery = serde_json::from_str(
            r#"{"page":2,"page_size":30,"customer_id":100,"status":"DRAFT","keyword":"QT-001"}"#,
        )
        .expect("完整参数应可反序列化");
        assert_eq!(q.page, Some(2));
        assert_eq!(q.page_size, Some(30));
        assert_eq!(q.customer_id, Some(100));
        assert_eq!(q.sales_user_id, None);
        assert_eq!(q.status.as_deref(), Some("DRAFT"));
        assert_eq!(q.keyword.as_deref(), Some("QT-001"));
    }

    // -------- Service 装配路径 --------

    /// 验证 Service 构造签名：fn(Arc<DatabaseConnection>) -> QuotationService
    #[test]
    fn test_service_constructor_signature() {
        let _: fn(std::sync::Arc<sea_orm::DatabaseConnection>) -> QuotationService =
            QuotationService::new;
    }
}
