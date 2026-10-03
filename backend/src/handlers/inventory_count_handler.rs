//! 库存盘点 HTTP 端点
//!
//! v11 批次 143 P1-1：真实实现库存盘点功能（v8 占位已恢复）
//!
//! 提供端点：
//! - `POST /api/v1/erp/inventory/counts` — 创建盘点单（自动生成库存快照明细）
//! - `GET /api/v1/erp/inventory/counts` — 查询盘点单列表（分页 + 仓库/状态过滤）
//! - `GET /api/v1/erp/inventory/counts/:id` — 查询盘点单详情（含明细）
//! - `PUT /api/v1/erp/inventory/counts/:id` — 更新盘点单（仅 pending 状态）
//! - `DELETE /api/v1/erp/inventory/counts/:id` — 删除盘点单（仅 pending 状态）
//! - `POST /api/v1/erp/inventory/counts/:id/record` — 录入实盘数量并自动计算差异
//! - `POST /api/v1/erp/inventory/counts/:id/submit` — 提交盘点单进入审批
//! - `POST /api/v1/erp/inventory/counts/:id/approve` — 审批通过并完成盘点（同步更新库存）
//! - `POST /api/v1/erp/inventory/counts/:id/reject` — 驳回审批（退回 pending 状态）

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::inventory_count;
use crate::models::inventory_count_item;
use crate::services::inventory_count_service::{
    CountItemInput, CreateCountRequest, InventoryCountService, UpdateCountRequest,
};
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use crate::utils::response::ApiResponse;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 创建盘点单请求体
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CreateCountPayload {
    pub warehouse_id: i32,
    /// ISO 8601 日期字符串（如 "2026-07-06T00:00:00Z"）
    pub count_date: String,
    pub notes: Option<String>,
    /// 指定库存快照 ID 列表；None 或空数组表示仓库下全部库存
    pub stock_ids: Option<Vec<i32>>,
}

/// 盘点单响应
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Serialize)]
pub struct CountResponse {
    pub id: i32,
    pub count_no: String,
    pub warehouse_id: i32,
    pub count_date: DateTime<Utc>,
    pub status: String,
    pub total_items: i32,
    pub counted_items: i32,
    pub variance_items: i32,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub items: Vec<CountItemResponse>,
}

impl From<inventory_count::Model> for CountResponse {
    fn from(c: inventory_count::Model) -> Self {
        Self {
            id: c.id,
            count_no: c.count_no,
            warehouse_id: c.warehouse_id,
            count_date: c.count_date,
            status: c.status,
            total_items: c.total_items,
            counted_items: c.counted_items,
            variance_items: c.variance_items,
            notes: c.notes,
            created_by: c.created_by,
            approved_by: c.approved_by,
            approved_at: c.approved_at,
            completed_at: c.completed_at,
            created_at: c.created_at,
            updated_at: c.updated_at,
            items: Vec::new(),
        }
    }
}

/// 盘点明细响应
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Serialize)]
pub struct CountItemResponse {
    pub id: i32,
    pub count_id: i32,
    pub stock_id: i32,
    pub product_id: i32,
    pub warehouse_id: i32,
    pub quantity_before: Decimal,
    pub quantity_actual: Decimal,
    pub quantity_difference: Decimal,
    pub unit_cost: Decimal,
    pub total_cost: Decimal,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<inventory_count_item::Model> for CountItemResponse {
    fn from(it: inventory_count_item::Model) -> Self {
        Self {
            id: it.id,
            count_id: it.count_id,
            stock_id: it.stock_id,
            product_id: it.product_id,
            warehouse_id: it.warehouse_id,
            quantity_before: it.quantity_before,
            quantity_actual: it.quantity_actual,
            quantity_difference: it.quantity_difference,
            unit_cost: it.unit_cost,
            total_cost: it.total_cost,
            notes: it.notes,
            created_at: it.created_at,
            updated_at: it.updated_at,
        }
    }
}

/// 盘点单列表响应
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Serialize)]
pub struct CountListResponse {
    pub counts: Vec<CountSummary>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Serialize)]
pub struct CountSummary {
    pub id: i32,
    pub count_no: String,
    pub warehouse_id: i32,
    pub count_date: DateTime<Utc>,
    pub status: String,
    pub total_items: i32,
    pub counted_items: i32,
    pub variance_items: i32,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub warehouse_name: Option<String>,
    pub created_by_name: Option<String>,
}

/// 列表查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ListCountsParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub warehouse_id: Option<i32>,
    pub status: Option<String>,
    pub count_no: Option<String>,
}

/// 录入实盘数量请求体（PUT /inventory/counts/items/{item_id}）
///
/// 字段语义 = 显式三态部分更新（对齐 RFC 7386 JSON Merge Patch）：
/// 键缺席=保持原值、显式 `null`=清空为 NULL（仅 DB 可空列）、有值=覆盖。
/// NOT NULL 列（quantity_actual，inventory_count_item 模型为非 Option Decimal 列）
/// 不开 null 清空，显式 null 由 service 拒绝。前端 api/inventory-count.ts 的
/// `updateCountItem` 出参已声明 `notes?: string | null`——后端必须兑现该 null=清空契约，
/// 否则清空静默失效（单 Option 无适配器会把显式 null 塌成"未提供"）。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct UpdateCountItemPayload {
    /// 实盘数量：NOT NULL 列——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub quantity_actual: Option<Option<rust_decimal::Decimal>>,
    /// 备注：DB 可空列 notes（inventory_count_item 模型 Option<String>）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct RecordItemsPayload {
    pub items: Vec<RecordItemInput>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct RecordItemInput {
    pub stock_id: i32,
    /// 数量字符串（避免 JSON 浮点精度丢失）
    pub quantity_actual: String,
    pub notes: Option<String>,
}

/// JSON 三态反序列化适配器（RFC 7386 JSON Merge Patch 的"键缺席 ≠ 显式 null"语义所需）。
///
/// 为何需要：serde_json 对 `Option<Option<T>>` 的默认反序列化在遇到 JSON null 时
/// 直接调 visit_none()，把"显式 null"塌成外层 `None`，与"键缺席"不可区分。
/// 本适配器把字段先按内层 `Option<T>` 反序列化再包一层：
/// 键缺席（配合 `#[serde(default)]`）= `None`、显式 null = `Some(None)`、有值 = `Some(Some(v))`。
/// 与 handlers/department_handler.rs 中同名适配器形状一致（跨域合并到共享工具需动
/// utils，超出本批授权范围，各域 handler 内私有定义）。
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

/// 更新盘点单请求体
///
/// 字段语义 = 显式三态部分更新（对齐 RFC 7386 JSON Merge Patch）：
/// 键缺席=保持原值、显式 `null`=清空为 NULL（仅 DB 可空列）、有值=覆盖。
/// NOT NULL 列（count_date，m0001 DDL）不开 null 清空，显式 null 由 service 拒绝。
#[derive(Debug, Deserialize)]
#[allow(dead_code, reason = "反序列化输入字段")]
pub struct UpdateCountPayload {
    /// 盘点日期：NOT NULL 列——显式 null 被 service 拒绝（格式非法在入口显式报错）
    #[serde(default, deserialize_with = "double_option")]
    pub count_date: Option<Option<String>>,
    /// 备注：DB 可空列 notes TEXT（m0001 DDL）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
}

/// 创建盘点单
pub async fn create_count(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(payload): Json<CreateCountPayload>,
) -> Result<Json<ApiResponse<CountResponse>>, AppError> {
    let service = InventoryCountService::new(state.db.clone());
    let count_date: DateTime<Utc> = payload
        .count_date
        .parse::<DateTime<Utc>>()
        .map_err(|e| AppError::validation_displayable(format!("日期格式错误：{}", e)))?;

    let req = CreateCountRequest {
        warehouse_id: payload.warehouse_id,
        count_date,
        notes: payload.notes,
        created_by: Some(auth.user_id),
        stock_ids: payload.stock_ids,
    };
    let detail = service.create_count(req).await?;
    let mut resp: CountResponse = detail.count.into();
    resp.items = detail.items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(resp)))
}

/// 查询盘点单列表
pub async fn list_counts(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(params): Query<ListCountsParams>,
) -> Result<Json<ApiResponse<CountListResponse>>, AppError> {
    let service = InventoryCountService::new(state.db.clone());
    let page = params.page.unwrap_or(1).clamp(1, 1000);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    // V15 P0-S01：提取行级数据权限上下文
    let data_scope_ctx = auth.to_data_scope_context();
    let (counts, total) = service
        .list_counts(
            page,
            page_size,
            params.warehouse_id,
            params.status,
            params.count_no,
            Some(&data_scope_ctx),
        )
        .await?;
    let summaries = counts
        .into_iter()
        .map(|c| CountSummary {
            id: c.id,
            count_no: c.count_no,
            warehouse_id: c.warehouse_id,
            count_date: c.count_date,
            status: c.status,
            total_items: c.total_items,
            counted_items: c.counted_items,
            variance_items: c.variance_items,
            created_at: c.created_at,
            completed_at: c.completed_at,
            notes: c.notes,
            warehouse_name: c.warehouse_name,
            created_by_name: c.created_by_name,
        })
        .collect();
    Ok(Json(ApiResponse::success(CountListResponse {
        counts: summaries,
        total,
        page,
        page_size,
    })))
}

/// 查询盘点单详情
pub async fn get_count(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<CountResponse>>, AppError> {
    let service = InventoryCountService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();
    let detail = service.get_count(id, Some(&data_scope_ctx)).await?;
    let mut resp: CountResponse = detail.count.into();
    resp.items = detail.items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(resp)))
}

/// 更新盘点单
pub async fn update_count(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateCountPayload>,
) -> Result<Json<ApiResponse<CountResponse>>, AppError> {
    let service = InventoryCountService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——更新前先校验资源归属（复用 P0-S01 的 get_count + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_count(id, Some(&data_scope_ctx)).await?;

    // count_date 三态透传：有值解析为时间戳（格式错误在入口显式报错）、
    // 显式 null 保持 Some(None) 交 service 判 NOT NULL 列清空拒绝、缺席 None 保持原值
    let count_date = match payload.count_date {
        Some(Some(s)) => Some(Some(s.parse::<DateTime<Utc>>().map_err(|e| {
            AppError::validation_displayable(format!("日期格式错误：{}", e))
        })?)),
        Some(None) => Some(None),
        None => None,
    };
    let req = UpdateCountRequest {
        count_date,
        notes: payload.notes,
    };
    let updated = service.update_count(id, req, Some(auth.user_id)).await?;
    let detail = service.get_count(id, None).await?;
    let mut resp: CountResponse = updated.into();
    resp.items = detail.items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(resp)))
}

/// 删除盘点单
pub async fn delete_count(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = InventoryCountService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——删除前先校验资源归属（复用 P0-S01 的 get_count + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_count(id, Some(&data_scope_ctx)).await?;

    service.delete_count(id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// 录入实盘数量并自动计算差异
pub async fn record_count_items(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(payload): Json<RecordItemsPayload>,
) -> Result<Json<ApiResponse<CountResponse>>, AppError> {
    let service = InventoryCountService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——录入实盘前先校验资源归属（复用 P0-S01 的 get_count + data_scope_ctx），
    // 与 update_count/delete_count 的「先 get_count(Some(&data_scope_ctx))」写法同源；
    // record_count_items 服务侧仅 find_by_id+lock_exclusive 无归属校验，越权由 get_count 返回 403。
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_count(id, Some(&data_scope_ctx)).await?;

    let mut inputs = Vec::with_capacity(payload.items.len());
    for it in payload.items {
        let qty = it
            .quantity_actual
            .parse::<Decimal>()
            .map_err(|e| AppError::validation_displayable(format!("数量格式错误：{}", e)))?;
        inputs.push(CountItemInput {
            stock_id: it.stock_id,
            quantity_actual: qty,
            notes: it.notes,
        });
    }
    let detail = service.record_count_items(id, inputs).await?;
    let mut resp: CountResponse = detail.count.into();
    resp.items = detail.items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(resp)))
}

/// 提交盘点单进入审批
pub async fn submit_for_approval(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<CountResponse>>, AppError> {
    let service = InventoryCountService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——提交审批前先校验资源归属（复用 P0-S01 的 get_count + data_scope_ctx），
    // 与 update_count/delete_count 的「先 get_count(Some(&data_scope_ctx))」写法同源；
    // submit_for_approval 服务侧仅 find_by_id+lock_exclusive 无归属校验，越权由 get_count 返回 403。
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_count(id, Some(&data_scope_ctx)).await?;

    let updated = service.submit_for_approval(id).await?;
    let detail = service.get_count(id, None).await?;
    let mut resp: CountResponse = updated.into();
    resp.items = detail.items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(resp)))
}

/// 审批通过并完成盘点
pub async fn approve_count(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<CountResponse>>, AppError> {
    let service = InventoryCountService::new(state.db.clone());
    let updated = service.approve_count(id, auth.user_id).await?;
    let detail = service.get_count(id, None).await?;
    let mut resp: CountResponse = updated.into();
    resp.items = detail.items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(resp)))
}

/// 驳回审批
pub async fn reject_count(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<CountResponse>>, AppError> {
    let service = InventoryCountService::new(state.db.clone());
    let updated = service.reject_count(id).await?;
    let detail = service.get_count(id, None).await?;
    let mut resp: CountResponse = updated.into();
    resp.items = detail.items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(resp)))
}

/// 生成库存盘点单号 GET /api/v1/erp/inventory/counts/generate-no；单据号格式：`IC{yyyyMMdd}{3 位流水}`
/// 流水位数与落库权威 `InventoryCountService::create_count` 内
/// `generate_no_with_txn("IC")`（默认 3 位）对齐；任务 #154 修复同前缀不同位数
/// 导致列表与库内单号长度不一致（原展示 4 位）。
/// 对应前端 api/inventory-count.ts 的 generateInventoryCountNo。
pub async fn generate_no(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let count_no = DocumentNumberGenerator::generate_no(
        &*state.db,
        "IC",
        inventory_count::Entity,
        inventory_count::Column::CountNo,
    )
    .await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "count_no": count_no
    }))))
}

/// 更新单条盘点明细 PUT /api/v1/erp/inventory/counts/items/{itemId}
/// 对应前端 updateCountItem（实盘数量/备注），仅待盘点状态可改。
pub async fn update_count_item(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(item_id): Path<i32>,
    Json(payload): Json<UpdateCountItemPayload>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let updated = InventoryCountService::new(state.db.clone())
        .update_count_item(item_id, payload.quantity_actual, payload.notes)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "id": updated.id,
        "count_id": updated.count_id,
        "stock_id": updated.stock_id,
        "quantity_actual": updated.quantity_actual,
        "quantity_difference": updated.quantity_difference,
        "notes": updated.notes,
    }))))
}

/// 删除单条盘点明细 DELETE /api/v1/erp/inventory/counts/items/{itemId}
/// 对应前端 deleteCountItem，仅待盘点状态可删。
pub async fn delete_count_item(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(item_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    InventoryCountService::new(state.db.clone())
        .delete_count_item(item_id)
        .await?;
    Ok(Json(ApiResponse::success(
        serde_json::json!({ "deleted": true }),
    )))
}
