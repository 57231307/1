//! 采购入库 Handler
//!
//! 采购入库 HTTP 接口层，负责处理 HTTP 请求并调用 Service 层

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::{purchase_order, purchase_receipt, warehouse};
use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
use crate::services::purchase_receipt_dto::{
    CreatePurchaseReceiptRequest, CreateReceiptItemRequest, UpdatePurchaseReceiptRequest,
    UpdateReceiptItemRequest,
};
use crate::services::purchase_receipt_service::PurchaseReceiptService;
use crate::utils::admin_checker;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::NaiveDate;
use sea_orm::EntityTrait;
use serde::Deserialize;
use validator::Validate;

/// 查询采购入库单列表
pub async fn list_receipts(
    Query(params): Query<ReceiptQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReceiptService::new(state.db.clone());
    let (receipts, total) = service
        .list_receipts(
            params.page.unwrap_or(1).clamp(1, 1000), // 批次 95 P3-3~8：分页 clamp 防 DoS
            params.page_size.unwrap_or(20).clamp(1, 100),
            params.status,
            params.supplier_id,
            params.order_id,
            params.keyword,
            params.warehouse_id,
            parse_receipt_date_param(params.receipt_date_from.as_deref(), "receipt_date_from")?,
            parse_receipt_date_param(params.receipt_date_to.as_deref(), "receipt_date_to")?,
        )
        .await?;

    // 批次 406 修复：序列化失败应传播错误而非返回 Null，避免 API 返回空数据掩盖问题
    let mut items_json: Vec<serde_json::Value> = receipts
        .into_iter()
        .map(|r| serde_json::to_value(r).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        // 非静默：原 `if let Ok(Some(_))` 把权限查询 Err 与 Ok(None) 静默合并，按
        // 本仓既有做法（crm_handler::resolve_role_data_permission）对 Err 记 warn 后
        // 同走 fail-closed 默认处理，出参语义与原实现逐字一致。
        let permission = match state
            .data_permission_service
            .get_role_data_permission(role_id, "purchase_receipt")
            .await
        {
            Ok(permission) => permission,
            Err(error) => {
                tracing::warn!(
                    %error,
                    role_id,
                    resource_type = "purchase_receipt",
                    rule = "data_permission_lookup_fail_closed",
                    "角色数据权限查询失败，出参按无权限行 fail-closed 走默认处理"
                );
                None
            }
        };
        if let Some(permission) = permission {
            state.data_permission_service.filter_fields_batch(
                &mut items_json,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
        } else if !admin_checker::is_admin_role(&state.db, role_id).await {
            // D-4 收口（PR #942 波次）：admin 判定走本仓唯一权威源
            // `admin_checker::is_admin_role`（roles.code='admin'，查询失败 fail-closed=false），
            // 禁止角色主键字面量判定（播种漂移时静默剔权/静默扩权）；判定在循环外的分支
            // 条件处、每请求至多一次（admin_checker 内部带 5 分钟缓存，同 crm_handler 既有范式）。
            // 如果没有配置数据权限且不是管理员，使用默认字段隐藏
            for receipt in &mut items_json {
                if let Some(obj) = receipt.as_object_mut() {
                    obj.remove("total_amount");
                    obj.remove("tax_amount");
                    obj.remove("discount_amount");
                }
            }
        }
    }

    let result = serde_json::to_value(PaginatedResponse::new(
        items_json,
        total,
        params.page.unwrap_or(1).clamp(1, 1000), // 批次 95 P3-3~8：分页 clamp 防 DoS
        params.page_size.unwrap_or(20).clamp(1, 100),
    ))?;

    Ok(Json(ApiResponse::success(result)))
}

/// 获取采购入库单详情
pub async fn get_receipt(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReceiptService::new(state.db.clone());
    let receipt = service.get_receipt(id).await?;
    let mut receipt_json = serde_json::to_value(receipt)?;

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        // Err 记 warn 后同走 fail-closed 默认处理（与 list_receipts 同款，不静默）
        let permission = match state
            .data_permission_service
            .get_role_data_permission(role_id, "purchase_receipt")
            .await
        {
            Ok(permission) => permission,
            Err(error) => {
                tracing::warn!(
                    %error,
                    role_id,
                    resource_type = "purchase_receipt",
                    rule = "data_permission_lookup_fail_closed",
                    "角色数据权限查询失败，出参按无权限行 fail-closed 走默认处理"
                );
                None
            }
        };
        if let Some(permission) = permission {
            state.data_permission_service.filter_fields(
                &mut receipt_json,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
        } else if !admin_checker::is_admin_role(&state.db, role_id).await {
            // 与列表同一单源判定（见 list_receipts 内 D-4 收口注释），每请求至多一次
            // 如果没有配置数据权限且不是管理员，使用默认字段隐藏
            if let Some(obj) = receipt_json.as_object_mut() {
                obj.remove("total_amount");
                obj.remove("tax_amount");
                obj.remove("discount_amount");
            }
        }
    }

    Ok(Json(ApiResponse::success(receipt_json)))
}

/// 创建采购入库单
#[axum::debug_handler]
pub async fn create_receipt(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(req): Json<CreatePurchaseReceiptRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 验证请求
    req.validate()?;

    let service = PurchaseReceiptService::new(state.db.clone());
    let user_id = auth.user_id;

    let receipt = service.create_receipt(req, user_id).await?;

    // 入库事件不在创建时发布：草稿入库单尚未确认，此刻收货会让库存无凭据增加，
    // 且把单据直接推到 COMPLETED，使"确认入库"端点必然报状态不允许。
    // 发布点在 confirm_receipt（确认后才产生收货事实）。

    // 发送采购到货通知
    if let Some(order_id) = receipt.order_id {
        if state.event_notification_service.is_none() {
            tracing::error!("事件通知服务未装配（container 应无条件构造），此处站内通知将缺失");
        }
        if let Some(ref event_service) = state.event_notification_service {
            if let Ok(Some(order)) = purchase_order::Entity::find_by_id(order_id)
                .one(&*state.db)
                .await
            {
                let warehouse_name = if let Ok(Some(wh)) =
                    warehouse::Entity::find_by_id(receipt.warehouse_id)
                        .one(&*state.db)
                        .await
                {
                    wh.name
                } else {
                    String::new()
                };

                // 批次 114 P1-6：通知发送失败改 warn 日志（原 `let _ =` 静默吞错）
                if let Err(e) = event_service
                    .notify_purchase_arrived(user_id, &order.order_no, order_id, &warehouse_name)
                    .await
                {
                    tracing::warn!(error = %e, order_id, "采购入库到达通知发送失败");
                }
            }
        }
    }

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(receipt)?,
        "采购入库单创建成功",
    )))
}

/// 更新采购入库单
#[axum::debug_handler]
pub async fn update_receipt(
    auth: AuthContext,
    Path(id): Path<i32>,
    State(state): State<AppState>,
    Json(req): Json<UpdatePurchaseReceiptRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReceiptService::new(state.db.clone());
    let user_id = auth.user_id;

    let receipt = service.update_receipt(id, req, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(receipt)?,
        "采购入库单更新成功",
    )))
}

/// 确认采购入库单
pub async fn confirm_receipt(
    auth: AuthContext,
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReceiptService::new(state.db.clone());
    let user_id = auth.user_id;

    let receipt = service.confirm_receipt(id, user_id).await?;

    // 收货事实（库存增加、订单转收货态、入库单置 COMPLETED）已在 confirm_receipt 事务内完成，
    // 这里发布事件驱动账务下游：核对收货终态并在确认路径应付生成失败时补偿
    if let Some(order_id) = receipt.order_id {
        EVENT_BUS.publish(BusinessEvent::PurchaseReceiptCompleted {
            receipt_id: receipt.id,
            order_id,
            supplier_id: receipt.supplier_id,
        });
    }

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(receipt)?,
        "采购入库单已确认",
    )))
}

/// 删除采购入库单
pub async fn delete_receipt(
    auth: AuthContext,
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = PurchaseReceiptService::new(state.db.clone());
    let user_id = auth.user_id;

    service.delete_receipt(id, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        (),
        "采购入库单删除成功",
    )))
}

/// 获取入库明细列表
pub async fn list_receipt_items(
    auth: AuthContext,
    Path(receipt_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReceiptService::new(state.db.clone());
    let items = service.list_receipt_items(receipt_id).await?;
    let mut items_json = serde_json::to_value(items)?;

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        // Err 记 warn 后同走 fail-closed 默认处理（与 list_receipts 同款，不静默）
        let permission = match state
            .data_permission_service
            .get_role_data_permission(role_id, "purchase_receipt_item")
            .await
        {
            Ok(permission) => permission,
            Err(error) => {
                tracing::warn!(
                    %error,
                    role_id,
                    resource_type = "purchase_receipt_item",
                    rule = "data_permission_lookup_fail_closed",
                    "角色数据权限查询失败，出参按无权限行 fail-closed 走默认处理"
                );
                None
            }
        };
        if let Some(permission) = permission {
            if let Some(list) = items_json.as_array_mut() {
                state.data_permission_service.filter_fields_batch(
                    list,
                    &permission.allowed_fields,
                    &permission.hidden_fields,
                );
            }
        } else if !admin_checker::is_admin_role(&state.db, role_id).await {
            // 与单头同一单源判定（见 list_receipts 内 D-4 收口注释），每请求至多一次
            // 如果没有配置数据权限且不是管理员，使用默认字段隐藏
            if let Some(list) = items_json.as_array_mut() {
                for item in list {
                    if let Some(obj) = item.as_object_mut() {
                        obj.remove("unit_price");
                        obj.remove("total_price");
                        obj.remove("tax_amount");
                    }
                }
            }
        }
    }

    Ok(Json(ApiResponse::success(items_json)))
}

/// 添加入库明细
#[axum::debug_handler]
pub async fn create_receipt_item(
    auth: AuthContext,
    Path(receipt_id): Path<i32>,
    State(state): State<AppState>,
    Json(req): Json<CreateReceiptItemRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 验证请求
    req.validate()?;

    let service = PurchaseReceiptService::new(state.db.clone());
    let user_id = auth.user_id;

    let item = service.add_receipt_item(receipt_id, req, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(item)?,
        "入库明细添加成功",
    )))
}

/// 更新入库明细
#[axum::debug_handler]
pub async fn update_receipt_item(
    auth: AuthContext,
    Path((_receipt_id, item_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    Json(req): Json<UpdateReceiptItemRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseReceiptService::new(state.db.clone());
    let user_id = auth.user_id;

    let item = service.update_receipt_item(item_id, req, user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(item)?,
        "入库明细更新成功",
    )))
}

/// 删除入库明细
pub async fn delete_receipt_item(
    auth: AuthContext,
    Path((_receipt_id, item_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = PurchaseReceiptService::new(state.db.clone());
    service.delete_receipt_item(item_id, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        (),
        "入库明细删除成功",
    )))
}

/// 生成采购入库单号 GET /api/v1/erp/purchase/receipts/generate-no；单据号格式：`PR{yyyyMMdd}{3 位流水}`
/// 例如 `PR20260514001`。前缀/位数与落库权威
/// `PurchaseReceiptService::generate_receipt_no`（impl_generate_no! "PR"，默认 3 位）逐字一致，
/// 任务 #154 修复展示码≠落库码双轨缺陷（原展示 "RK"/4 位）。
/// 依赖数据库 `purchase_receipt.receipt_no` 列上的 `UNIQUE` 约束保证最终唯一性。
pub async fn generate_no(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let receipt_no = DocumentNumberGenerator::generate_no(
        &*state.db,
        "PR",
        purchase_receipt::Entity,
        purchase_receipt::Column::ReceiptNo,
    )
    .await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "receipt_no": receipt_no
    }))))
}

/// POST /api/v1/erp/purchase/receipts/:id/recalculate - 手动重算入库单总金额（运维兜底入口）
/// v11 批次 154c P2-A：接入 calculate_receipt_total，用于数据修复场景
pub async fn recalculate_receipt_total(
    auth: AuthContext,
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = PurchaseReceiptService::new(state.db.clone());
    service.calculate_receipt_total(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        (),
        "入库单总金额重算成功",
    )))
}

// =====================================================
// 请求 DTO
// =====================================================

/// 日期查询参数按 `%Y-%m-%d` 严格解析（本仓同款惯例：
/// `budget_management_handler.rs:408-409`、`tracking_handler.rs:235-247`）。
/// 为什么不把 DTO 字段直接写成 `Option<NaiveDate>`：`axum::Query` 的类型化反序列化
/// 失败走 QueryRejection，出参是纯文本 400、不经过 `AppError` 信封（本仓未覆盖
/// Rejection 响应，见 `handlers_query_param_coercion_test.rs` 对拒绝体的文本断言），
/// 会违背「字段取值错误 = VALIDATION_ERROR 信封」裁定
/// （先例：`contract_wave4_api_key_echo_and_expiry_test.rs` 非法日期断言
/// `code=VALIDATION_ERROR` + 真实文案外显）。
fn parse_receipt_date_param(raw: Option<&str>, field: &str) -> Result<Option<NaiveDate>, AppError> {
    raw.map(|v| {
        NaiveDate::parse_from_str(v, "%Y-%m-%d").map_err(|e| {
            AppError::validation_displayable(format!(
                "{field} 日期格式无效（应为 YYYY-MM-DD）：{e}"
            ))
        })
    })
    .transpose()
}

/// 采购入库单查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ReceiptQueryParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
    pub order_id: Option<i32>,
    /// 关键字：匹配入库单号或明细物料名（purchase_receipt_item.material_name）
    #[serde(
        default,
        deserialize_with = "crate::utils::query_params::empty_str_as_none"
    )]
    pub keyword: Option<String>,
    pub warehouse_id: Option<i32>,
    /// 入库日期区间下界（YYYY-MM-DD，含当日）
    #[serde(
        default,
        deserialize_with = "crate::utils::query_params::empty_str_as_none"
    )]
    pub receipt_date_from: Option<String>,
    /// 入库日期区间上界（YYYY-MM-DD，含当日）
    #[serde(
        default,
        deserialize_with = "crate::utils::query_params::empty_str_as_none"
    )]
    pub receipt_date_to: Option<String>,
}
