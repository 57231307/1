use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::inventory_adjustment;
use crate::models::inventory_adjustment_item;
use crate::services::inventory_adjustment_service::{
    AdjustmentItemRequest, CreateAdjustmentRequest, InventoryAdjustmentService,
    UpdateAdjustmentRequest,
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

/// 创建调整单请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CreateAdjustmentRequestPayload {
    pub warehouse_id: i32,
    pub adjustment_date: String,
    pub adjustment_type: String,
    pub reason_type: String,
    pub reason_description: Option<String>,
    pub notes: Option<String>,
    pub items: Vec<AdjustmentItemPayload>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct AdjustmentItemPayload {
    pub stock_id: i32,
    pub quantity: String,
    pub unit_cost: Option<String>,
    pub notes: Option<String>,
}

/// 调整单响应
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Serialize)]
pub struct AdjustmentResponse {
    pub id: i32,
    pub adjustment_no: String,
    pub warehouse_id: i32,
    pub adjustment_date: DateTime<Utc>,
    pub adjustment_type: String,
    pub reason_type: String,
    pub reason_description: Option<String>,
    pub total_quantity: Decimal,
    pub notes: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub items: Vec<AdjustmentItemResponse>,
}

#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Serialize)]
pub struct AdjustmentItemResponse {
    pub id: i32,
    pub stock_id: i32,
    pub quantity: Decimal,
    pub quantity_before: Decimal,
    pub quantity_after: Decimal,
    pub unit_cost: Option<Decimal>,
    pub amount: Option<Decimal>,
    pub notes: Option<String>,
}

/// 调整单列表响应
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Serialize)]
pub struct AdjustmentListResponse {
    pub adjustments: Vec<AdjustmentSummary>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Serialize)]
pub struct AdjustmentSummary {
    pub id: i32,
    pub adjustment_no: String,
    pub warehouse_id: i32,
    pub adjustment_type: String,
    pub reason_type: String,
    pub status: String,
    pub total_quantity: Decimal,
    pub created_at: DateTime<Utc>,
    pub adjustment_date: DateTime<Utc>,
    pub reason_description: Option<String>,
    pub warehouse_name: Option<String>,
    pub created_by_name: Option<String>,
}

/// 创建调整单
pub async fn create_adjustment(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(payload): Json<CreateAdjustmentRequestPayload>,
) -> Result<Json<ApiResponse<AdjustmentResponse>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());

    // 解析日期
    let adjustment_date: DateTime<Utc> = payload
        .adjustment_date
        .parse::<DateTime<Utc>>()
        .map_err(|e| AppError::validation_displayable(format!("日期格式错误：{}", e)))?;

    let mut items = Vec::with_capacity(payload.items.len());
    for item in payload.items {
        let quantity = item
            .quantity
            .parse::<Decimal>()
            .map_err(|e| AppError::validation_displayable(format!("数量格式错误：{}", e)))?;

        items.push(AdjustmentItemRequest {
            stock_id: item.stock_id,
            quantity,
            // unit_cost 非法字符串如实报校验错误：原 and_then(parse.ok()) 把
            // "abc" 静默塌成 None（=清空成本），与"未提供"不可区分，属吞异常兜底
            unit_cost: item
                .unit_cost
                .map(|s| {
                    s.parse::<Decimal>().map_err(|e| {
                        AppError::validation_displayable(format!("成本格式错误：{}", e))
                    })
                })
                .transpose()?,
            notes: item.notes,
        });
    }

    // 转换请求
    let request = CreateAdjustmentRequest {
        warehouse_id: payload.warehouse_id,
        adjustment_date,
        adjustment_type: payload.adjustment_type,
        reason_type: payload.reason_type,
        reason_description: payload.reason_description,
        notes: payload.notes,
        created_by: Some(auth.user_id),
        items,
    };

    let detail = service.create_adjustment(request).await?;

    Ok(Json(ApiResponse::success(AdjustmentResponse {
        id: detail.adjustment.id,
        adjustment_no: detail.adjustment.adjustment_no,
        warehouse_id: detail.adjustment.warehouse_id,
        adjustment_date: detail.adjustment.adjustment_date,
        adjustment_type: detail.adjustment.adjustment_type,
        reason_type: detail.adjustment.reason_type,
        reason_description: detail.adjustment.reason_description,
        total_quantity: detail.adjustment.total_quantity,
        notes: detail.adjustment.notes,
        status: detail.adjustment.status,
        created_at: detail.adjustment.created_at,
        items: detail
            .items
            .into_iter()
            .map(|item| AdjustmentItemResponse {
                id: item.id,
                stock_id: item.stock_id,
                quantity: item.quantity,
                quantity_before: item.quantity_before,
                quantity_after: item.quantity_after,
                unit_cost: item.unit_cost,
                amount: item.amount,
                notes: item.notes,
            })
            .collect(),
    })))
}

/// 审核调整单
pub async fn approve_adjustment(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<AdjustmentResponse>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());
    let user_id = auth.user_id;

    service.approve_adjustment(id, user_id).await?;

    let detail = service.get_adjustment(id, None).await?;

    Ok(Json(ApiResponse::success(AdjustmentResponse {
        id: detail.adjustment.id,
        adjustment_no: detail.adjustment.adjustment_no,
        warehouse_id: detail.adjustment.warehouse_id,
        adjustment_date: detail.adjustment.adjustment_date,
        adjustment_type: detail.adjustment.adjustment_type,
        reason_type: detail.adjustment.reason_type,
        reason_description: detail.adjustment.reason_description,
        total_quantity: detail.adjustment.total_quantity,
        notes: detail.adjustment.notes,
        status: detail.adjustment.status,
        created_at: detail.adjustment.created_at,
        items: detail
            .items
            .into_iter()
            .map(|item| AdjustmentItemResponse {
                id: item.id,
                stock_id: item.stock_id,
                quantity: item.quantity,
                quantity_before: item.quantity_before,
                quantity_after: item.quantity_after,
                unit_cost: item.unit_cost,
                amount: item.amount,
                notes: item.notes,
            })
            .collect(),
    })))
}

/// 驳回调整单
pub async fn reject_adjustment(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<AdjustmentResponse>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());

    service.reject_adjustment(id).await?;

    let detail = service.get_adjustment(id, None).await?;

    // 发送审批拒绝通知
    if state.event_notification_service.is_none() {
        tracing::error!("事件通知服务未装配（container 应无条件构造），此处站内通知将缺失");
    }
    if let Some(ref event_service) = state.event_notification_service {
        if let Some(created_by) = detail.adjustment.created_by {
            // 批次 114 P1-6：通知发送失败改 warn 日志（原 `let _ =` 静默吞错）
            if let Err(e) = event_service
                .notify_approval_result(
                    created_by,
                    &detail.adjustment.adjustment_no,
                    false,
                    auth.user_id,
                    &auth.username,
                    None,
                )
                .await
            {
                tracing::warn!(error = %e, adjustment_id = id, "库存调整审批拒绝通知发送失败");
            }
        }
    }

    Ok(Json(ApiResponse::success(AdjustmentResponse {
        id: detail.adjustment.id,
        adjustment_no: detail.adjustment.adjustment_no,
        warehouse_id: detail.adjustment.warehouse_id,
        adjustment_date: detail.adjustment.adjustment_date,
        adjustment_type: detail.adjustment.adjustment_type,
        reason_type: detail.adjustment.reason_type,
        reason_description: detail.adjustment.reason_description,
        total_quantity: detail.adjustment.total_quantity,
        notes: detail.adjustment.notes,
        status: detail.adjustment.status,
        created_at: detail.adjustment.created_at,
        items: detail
            .items
            .into_iter()
            .map(|item| AdjustmentItemResponse {
                id: item.id,
                stock_id: item.stock_id,
                quantity: item.quantity,
                quantity_before: item.quantity_before,
                quantity_after: item.quantity_after,
                unit_cost: item.unit_cost,
                amount: item.amount,
                notes: item.notes,
            })
            .collect(),
    })))
}

/// 查询调整单列表
pub async fn list_adjustments(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(params): Query<ListAdjustmentsParams>,
) -> Result<Json<ApiResponse<AdjustmentListResponse>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文
    let data_scope_ctx = auth.to_data_scope_context();

    let page = params.page.unwrap_or(1).clamp(1, 1000);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    let (adjustments, total) = service
        .list_adjustments(
            page,
            page_size,
            params.adjustment_no,
            params.status,
            Some(&data_scope_ctx),
        )
        .await?;

    Ok(Json(ApiResponse::success(AdjustmentListResponse {
        adjustments: adjustments
            .into_iter()
            .map(|a| AdjustmentSummary {
                id: a.id,
                adjustment_no: a.adjustment_no,
                warehouse_id: a.warehouse_id,
                adjustment_type: a.adjustment_type,
                reason_type: a.reason_type,
                status: a.status,
                total_quantity: a.total_quantity,
                created_at: a.created_at,
                adjustment_date: a.adjustment_date,
                reason_description: a.reason_description,
                warehouse_name: a.warehouse_name,
                created_by_name: a.created_by_name,
            })
            .collect(),
        total,
        page,
        page_size,
    })))
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ListAdjustmentsParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub adjustment_no: Option<String>,
    pub status: Option<String>,
}

/// 查询调整单详情
pub async fn get_adjustment(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<AdjustmentResponse>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();

    let detail = service.get_adjustment(id, Some(&data_scope_ctx)).await?;

    Ok(Json(ApiResponse::success(AdjustmentResponse {
        id: detail.adjustment.id,
        adjustment_no: detail.adjustment.adjustment_no,
        warehouse_id: detail.adjustment.warehouse_id,
        adjustment_date: detail.adjustment.adjustment_date,
        adjustment_type: detail.adjustment.adjustment_type,
        reason_type: detail.adjustment.reason_type,
        reason_description: detail.adjustment.reason_description,
        total_quantity: detail.adjustment.total_quantity,
        notes: detail.adjustment.notes,
        status: detail.adjustment.status,
        created_at: detail.adjustment.created_at,
        items: detail
            .items
            .into_iter()
            .map(|item| AdjustmentItemResponse {
                id: item.id,
                stock_id: item.stock_id,
                quantity: item.quantity,
                quantity_before: item.quantity_before,
                quantity_after: item.quantity_after,
                unit_cost: item.unit_cost,
                amount: item.amount,
                notes: item.notes,
            })
            .collect(),
    })))
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

/// 更新调整单请求 DTO
///
/// 字段语义 = 显式三态部分更新（对齐 RFC 7386 JSON Merge Patch）：
/// 键缺席=保持原值、显式 `null`=清空为 NULL（仅 DB 可空列）、有值=覆盖。
/// NOT NULL 列（warehouse_id/adjustment_date/adjustment_type/reason_type，
/// m0010_add_inventory_extensions DDL）不开 null 清空，显式 null 由 service 拒绝。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct UpdateAdjustmentRequestPayload {
    /// 仓库 ID：NOT NULL 列——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub warehouse_id: Option<Option<i32>>,
    /// 调整日期：NOT NULL 列——显式 null 被 service 拒绝（格式非法在入口显式报错）
    #[serde(default, deserialize_with = "double_option")]
    pub adjustment_date: Option<Option<String>>,
    /// 调整类型：NOT NULL 列——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub adjustment_type: Option<Option<String>>,
    /// 原因类型：NOT NULL 列——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub reason_type: Option<Option<String>>,
    /// 原因说明：DB 可空列 reason_description TEXT（m0010 DDL）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub reason_description: Option<Option<String>>,
    /// 备注：DB 可空列 notes TEXT（m0010 DDL）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
}

/// 更新调整单
pub async fn update_adjustment(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateAdjustmentRequestPayload>,
) -> Result<Json<ApiResponse<AdjustmentResponse>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());

    // V15 P0-S02：IDOR 防护——更新前先校验资源归属（复用 P0-S01 的 get_adjustment + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_adjustment(id, Some(&data_scope_ctx)).await?;

    // adjustment_date 三态透传：有值解析为时间戳（格式错误在入口显式报错）、
    // 显式 null 保持 Some(None) 交 service 判 NOT NULL 列清空拒绝、缺席 None 保持原值
    let adjustment_date = match payload.adjustment_date {
        Some(Some(s)) => Some(Some(s.parse::<DateTime<Utc>>().map_err(|e| {
            AppError::validation_displayable(format!("日期格式错误：{}", e))
        })?)),
        Some(None) => Some(None),
        None => None,
    };

    let req = UpdateAdjustmentRequest {
        warehouse_id: payload.warehouse_id,
        adjustment_date,
        adjustment_type: payload.adjustment_type,
        reason_type: payload.reason_type,
        reason_description: payload.reason_description,
        notes: payload.notes,
    };

    service.update_adjustment(id, req).await?;

    let detail = service.get_adjustment(id, None).await?;

    Ok(Json(ApiResponse::success(AdjustmentResponse {
        id: detail.adjustment.id,
        adjustment_no: detail.adjustment.adjustment_no,
        warehouse_id: detail.adjustment.warehouse_id,
        adjustment_date: detail.adjustment.adjustment_date,
        adjustment_type: detail.adjustment.adjustment_type,
        reason_type: detail.adjustment.reason_type,
        reason_description: detail.adjustment.reason_description,
        total_quantity: detail.adjustment.total_quantity,
        notes: detail.adjustment.notes,
        status: detail.adjustment.status,
        created_at: detail.adjustment.created_at,
        items: detail
            .items
            .into_iter()
            .map(|item| AdjustmentItemResponse {
                id: item.id,
                stock_id: item.stock_id,
                quantity: item.quantity,
                quantity_before: item.quantity_before,
                quantity_after: item.quantity_after,
                unit_cost: item.unit_cost,
                amount: item.amount,
                notes: item.notes,
            })
            .collect(),
    })))
}

/// 删除调整单
pub async fn delete_adjustment(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());
    // V15 P0-S02：IDOR 防护——删除前先校验资源归属（复用 P0-S01 的 get_adjustment + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_adjustment(id, Some(&data_scope_ctx)).await?;

    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    service.delete_adjustment(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// 列出调整单的所有明细项
pub async fn list_items(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<AdjustmentItemResponse>>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());
    // IDOR 防护：明细只读同样需 data-scope，先校验父调整单归属（与同文件
    // update/delete_adjustment 的 get_adjustment(Some(&data_scope_ctx)) 同源），
    // 越权由 get_adjustment 内部 check_resource_owner 返回 403，避免越权枚举他人明细。
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_adjustment(id, Some(&data_scope_ctx)).await?;

    let items = service.list_items(id).await?;
    Ok(Json(ApiResponse::success(
        items
            .into_iter()
            .map(|item| AdjustmentItemResponse {
                id: item.id,
                stock_id: item.stock_id,
                quantity: item.quantity,
                quantity_before: item.quantity_before,
                quantity_after: item.quantity_after,
                unit_cost: item.unit_cost,
                amount: item.amount,
                notes: item.notes,
            })
            .collect(),
    )))
}

/// 向调整单添加明细
pub async fn add_item(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(payload): Json<AdjustmentItemPayload>,
) -> Result<Json<ApiResponse<AdjustmentItemResponse>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());
    // IDOR 防护：路径 id 即父调整单 id，添加明细前先校验其归属（与同文件
    // update/delete_adjustment 的 get_adjustment(Some(&data_scope_ctx)) 同源），越权 403。
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_adjustment(id, Some(&data_scope_ctx)).await?;

    let quantity = payload
        .quantity
        .parse::<Decimal>()
        .map_err(|e| AppError::validation_displayable(format!("数量格式错误：{}", e)))?;
    let req = AdjustmentItemRequest {
        stock_id: payload.stock_id,
        quantity,
        // unit_cost 非法字符串如实报校验错误（不静默塌成 None=清空成本）
        unit_cost: payload
            .unit_cost
            .map(|s| {
                s.parse::<Decimal>()
                    .map_err(|e| AppError::validation_displayable(format!("成本格式错误：{}", e)))
            })
            .transpose()?,
        notes: payload.notes,
    };
    let item = service.add_item(id, req).await?;
    Ok(Json(ApiResponse::success(AdjustmentItemResponse {
        id: item.id,
        stock_id: item.stock_id,
        quantity: item.quantity,
        quantity_before: item.quantity_before,
        quantity_after: item.quantity_after,
        unit_cost: item.unit_cost,
        amount: item.amount,
        notes: item.notes,
    })))
}

/// 更新调整单明细
pub async fn update_item(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(item_id): Path<i32>,
    Json(payload): Json<AdjustmentItemPayload>,
) -> Result<Json<ApiResponse<AdjustmentItemResponse>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());
    // IDOR 防护：路径仅有 item_id，先反查其所属调整单 id，再走父调整单归属校验
    // （与同文件 update/delete_adjustment 的 get_adjustment(Some(&data_scope_ctx)) 同源），越权 403。
    let adjustment_id = service.get_adjustment_id_by_item(item_id).await?;
    let data_scope_ctx = auth.to_data_scope_context();
    service
        .get_adjustment(adjustment_id, Some(&data_scope_ctx))
        .await?;

    let quantity = payload
        .quantity
        .parse::<Decimal>()
        .map_err(|e| AppError::validation_displayable(format!("数量格式错误：{}", e)))?;
    let req = AdjustmentItemRequest {
        stock_id: payload.stock_id,
        quantity,
        // unit_cost 非法字符串如实报校验错误（不静默塌成 None=清空成本）
        unit_cost: payload
            .unit_cost
            .map(|s| {
                s.parse::<Decimal>()
                    .map_err(|e| AppError::validation_displayable(format!("成本格式错误：{}", e)))
            })
            .transpose()?,
        notes: payload.notes,
    };
    let item: inventory_adjustment_item::Model = service.update_item(item_id, req).await?;
    Ok(Json(ApiResponse::success(AdjustmentItemResponse {
        id: item.id,
        stock_id: item.stock_id,
        quantity: item.quantity,
        quantity_before: item.quantity_before,
        quantity_after: item.quantity_after,
        unit_cost: item.unit_cost,
        amount: item.amount,
        notes: item.notes,
    })))
}

/// 删除调整单明细
pub async fn delete_item(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(item_id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = InventoryAdjustmentService::new(state.db.clone());
    // IDOR 防护：路径仅有 item_id，先反查其所属调整单 id，再走父调整单归属校验
    // （与同文件 update/delete_adjustment 的 get_adjustment(Some(&data_scope_ctx)) 同源），越权 403。
    let adjustment_id = service.get_adjustment_id_by_item(item_id).await?;
    let data_scope_ctx = auth.to_data_scope_context();
    service
        .get_adjustment(adjustment_id, Some(&data_scope_ctx))
        .await?;

    service.delete_item(item_id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// 生成库存调整单号 GET /api/v1/erp/inventory/adjustments/generate-no；单据号格式：`ADJ{yyyyMMdd}{3 位流水}`
/// 例如 `ADJ20260514001`。前缀/位数与落库权威
/// `InventoryAdjustmentService::generate_adjustment_no`（impl_generate_no! "ADJ"，默认 3 位）逐字一致，
/// 任务 #154 修复展示码≠落库码双轨缺陷（原展示 "IA"/4 位）。
/// 数据库列 `inventory_adjustments.adjustment_no` 上的 `UNIQUE` 约束负责最终去重。
pub async fn generate_no(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let adjustment_no = DocumentNumberGenerator::generate_no(
        &*state.db,
        "ADJ",
        inventory_adjustment::Entity,
        inventory_adjustment::Column::AdjustmentNo,
    )
    .await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "adjustment_no": adjustment_no
    }))))
}
