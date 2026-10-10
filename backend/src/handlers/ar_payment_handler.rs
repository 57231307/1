use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// 批次 98 P2-B 修复（v5 复审）：本地 validate_amount_range 已抽取到 utils::validator 模块，
// 统一追加 round_dp(2) 精度校验。#[validate(custom)] 引用改为 crate::utils::validator::validate_amount_range。

/// 收款查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ArPaymentQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub payment_no: Option<String>,
}

/// 创建收款请求
/// 批次 31 v7 P1-6 修复：添加 Validate + 字段验证
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct CreateArPaymentRequest {
    pub customer_id: i32,
    #[validate(custom(function = "crate::utils::validator::validate_amount_range"))]
    pub amount: rust_decimal::Decimal,
    #[validate(length(min = 1, max = 50, message = "收款方式长度必须在1到50字符之间"))]
    pub payment_method: String,
    pub payment_date: chrono::NaiveDate,
    #[validate(length(max = 50, message = "银行账号长度不能超过50字符"))]
    pub bank_account: Option<String>,
    #[validate(length(max = 500, message = "备注长度不能超过500字符"))]
    pub remark: Option<String>,
    pub invoice_ids: Option<Vec<i32>>,
}

/// JSON 三态反序列化适配器（RFC 7386 JSON Merge Patch 的"键缺席 ≠ 显式 null"语义所需）。
///
/// 为何需要：serde_json 对 `Option<Option<T>>` 的默认反序列化在遇到 JSON null 时
/// 直接调 visit_none()，把"显式 null"塌成外层 `None`，与"键缺席"不可区分。
/// 本适配器把字段先按内层 `Option<T>` 反序列化再包一层：
/// 键缺席（配合 `#[serde(default)]`）= `None`、显式 null = `Some(None)`、有值 = `Some(Some(v))`。
/// 序列化侧配合 `skip_serializing_if = "Option::is_none"`：外层 `None` 整键省略、
/// `Some(None)` 落为 JSON null 键，保证 DTO→service 的 JSON 载荷保真三态。
/// 与各 handler 内同型私有适配器一致（先例：purchase_contract_handler.rs）。
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

/// 更新收款请求
/// 字段语义 = 显式三态部分更新（对齐 RFC 7386 JSON Merge Patch）：
/// 键缺席=保持原值、显式 `null`=清空为 NULL（仅 DB 可空列）、有值=覆盖。
/// 开放 ar_collections 的业务列（m0012/m0083 DDL）：collection_method/
/// bank_account/check_no/remark 四个可空列 + collection_amount/collection_date
/// 两个 NOT NULL 列；NOT NULL 列的显式 `null` 是调用方错误，由 service 入口
/// 业务拒绝，不落默认值（先例：production_order_handler.rs::UpdateProductionOrderPayload
/// 的 planned_quantity、inventory_adjustment_service.rs 的 NOT NULL 列门控）。
/// amount 承载 collection_amount，更新时执行与创建同源的金额校验（>0、精度≤2 位小数）
/// 及"新金额不得小于已核销分配金额"一致性门（ar_ops/collection.rs::update_payment）；
/// payment_date 承载 collection_date，覆盖前执行所属期间关账检查（同创建路径判据）。
/// 单号/客户/状态等身份与审计字段不经请求体承载，操作人取 AuthContext.user_id。
/// 校验注解仅在实际携带值（Some(Some(v))）时生效，`Some(None)` 清空路径逐层跳过。
#[allow(dead_code, reason = "序列化/反序列化字段")]
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct UpdateArPaymentRequest {
    /// 收款金额：DB NOT NULL 列 ar_collections.collection_amount（模型字段见 `models/ar_collection.rs`）；
    /// 键缺席=保持原值、有值=覆盖、显式 `null`=业务拒绝。
    /// 入站形态为 JSON number 或数字字符串（rust_decimal serde），值域/精度校验由 service
    /// 与创建路径同源执行（validate_payment_amount），此处不叠第二套 DTO 校验。
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub amount: Option<Option<rust_decimal::Decimal>>,
    /// 收款日期：DB NOT NULL 列 ar_collections.collection_date（`models/ar_collection.rs` 的 `collection_date` 字段）；
    /// 键缺席=保持原值、有值=覆盖（service 侧执行所属期间关账检查）、显式 `null`=业务拒绝。
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub payment_date: Option<Option<chrono::NaiveDate>>,
    /// 收款方式：DB 可空列 ar_collections.collection_method VARCHAR(50)
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[validate(length(max = 50, message = "收款方式长度不能超过50字符"))]
    pub payment_method: Option<Option<String>>,
    /// 银行账号：DB 可空列 ar_collections.bank_account VARCHAR(50)
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[validate(length(max = 50, message = "银行账号长度不能超过50字符"))]
    pub bank_account: Option<Option<String>>,
    /// 支票号：DB 可空列 ar_collections.check_no VARCHAR(50)
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[validate(length(max = 50, message = "支票号长度不能超过50字符"))]
    pub check_no: Option<Option<String>>,
    /// 备注：DB 可空列 ar_collections.remark TEXT（m0083 补列，与支票号分列）
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[validate(length(max = 500, message = "备注长度不能超过500字符"))]
    pub remark: Option<Option<String>>,
}

/// 获取收款列表
/// GET /api/v1/erp/ar/payments
pub async fn list_payments(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ArPaymentQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let page = query.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = query.page_size.unwrap_or(10).clamp(1, 100);
    // V15 P0-S01：提取行级数据权限上下文
    let data_scope_ctx = auth.to_data_scope_context();

    let (payments, total) = service
        .list_payments(
            page,
            page_size,
            query.status,
            query.customer_id,
            query.payment_no,
            Some(&data_scope_ctx),
        )
        .await?;

    let result = serde_json::json!({
        "list": payments,
        "total": total,
        "page": page,
        "page_size": page_size,
    });

    Ok(Json(ApiResponse::success(result)))
}

/// 获取收款详情
/// GET /api/v1/erp/ar/payments/:id
pub async fn get_payment(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();

    let payment = service.get_payment(id, Some(&data_scope_ctx)).await?;

    Ok(Json(ApiResponse::success(payment)))
}

/// 创建收款
/// POST /api/v1/erp/ar/payments
pub async fn create_payment(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(payload): Json<CreateArPaymentRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // v8 P1-C 修复：调用 DTO 验证，激活 Validate 注解
    payload.validate()?;
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    // 批次 329 v10 复审 P3 修复：使用参数对象替代多参数
    let params = crate::services::ar_service::CreateArPaymentParams {
        customer_id: payload.customer_id,
        amount: payload.amount,
        payment_method: payload.payment_method,
        payment_date: payload.payment_date,
        bank_account: payload.bank_account,
        remark: payload.remark,
        invoice_ids: payload.invoice_ids,
    };
    let payment = service.create_payment(params, auth.user_id).await?;

    Ok(Json(ApiResponse::success(payment)))
}

/// 更新收款
/// PUT /api/v1/erp/ar/payments/:id
pub async fn update_payment(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateArPaymentRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 三态在反序列化即成形（缺席=None/显式 null=Some(None)/有值=Some(Some(v))）；
    // 校验仅作用于实际携带值的字段，清空路径不被长度校验阻挡。
    payload.validate()?;
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    // V15 P0-S02：IDOR 防护——更新前先校验资源归属（复用 P0-S01 的 get_payment + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_payment(id, Some(&data_scope_ctx)).await?;

    let payload_json = serde_json::to_value(payload)?;

    let payment = service
        .update_payment(id, payload_json, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success(payment)))
}

/// 确认收款
/// POST /api/v1/erp/ar/payments/:id/confirm
pub async fn confirm_payment(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let payment = service.confirm_payment(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success(payment)))
}

/// 取消收款（批次 158 v11 真实接入）
/// POST /api/v1/erp/ar/payments/:id/cancel
pub async fn cancel_payment(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::ar_service::ArService::new(state.db.clone());

    let payment = service.cancel_collection(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success(payment)))
}
