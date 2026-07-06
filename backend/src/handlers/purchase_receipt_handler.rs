//! 采购入库 Handler
//!
//! 采购入库 HTTP 接口层，负责处理 HTTP 请求并调用 Service 层

use crate::middleware::auth_context::AuthContext;
use crate::models::{purchase_order, purchase_receipt, warehouse};
use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
use crate::services::purchase_receipt_dto::{
    CreatePurchaseReceiptRequest, CreateReceiptItemRequest, UpdatePurchaseReceiptRequest,
    UpdateReceiptItemRequest,
};
use crate::services::purchase_receipt_service::PurchaseReceiptService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    extract::{Path, Query, State},
    Json,
};
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
        )
        .await?;

    let mut items_json: Vec<serde_json::Value> = receipts
        .into_iter()
        .map(|r| serde_json::to_value(r).unwrap_or_default())
        .collect();

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        if let Ok(Some(permission)) = state
            .data_permission_service
            .get_role_data_permission(role_id, "purchase_receipt")
            .await
        {
            state.data_permission_service.filter_fields_batch(
                &mut items_json,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
        } else if role_id != 1 {
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
    ))
    .map_err(|e| AppError::internal(e.to_string()))?;

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
    let mut receipt_json = serde_json::to_value(receipt)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        if let Ok(Some(permission)) = state
            .data_permission_service
            .get_role_data_permission(role_id, "purchase_receipt")
            .await
        {
            state.data_permission_service.filter_fields(
                &mut receipt_json,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
        } else if role_id != 1 {
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

    // 发布采购收货完成事件
    if let Some(order_id) = receipt.order_id {
        EVENT_BUS.publish(BusinessEvent::PurchaseReceiptCompleted {
            receipt_id: receipt.id,
            order_id,
            supplier_id: receipt.supplier_id,
        });
    }

    // 发送采购到货通知
    if let Some(order_id) = receipt.order_id {
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
    let mut items_json = serde_json::to_value(items)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        if let Ok(Some(permission)) = state
            .data_permission_service
            .get_role_data_permission(role_id, "purchase_receipt_item")
            .await
        {
            if let Some(list) = items_json.as_array_mut() {
                state.data_permission_service.filter_fields_batch(
                    list,
                    &permission.allowed_fields,
                    &permission.hidden_fields,
                );
            }
        } else if role_id != 1 {
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

/// 生成采购入库单号
/// GET /api/v1/erp/purchase/receipts/generate-no
///
/// 单据号格式：`RK{yyyyMMdd}{4 位流水}`，例如 `RK202605140001`。
/// 依赖数据库 `purchase_receipt.receipt_no` 列上的 `UNIQUE` 约束保证最终唯一性。
pub async fn generate_no(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let receipt_no = DocumentNumberGenerator::generate_no_with_width(
        &*state.db,
        "RK",
        purchase_receipt::Entity,
        purchase_receipt::Column::ReceiptNo,
        4,
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

/// 采购入库单查询参数
#[derive(Debug, Deserialize)]
pub struct ReceiptQueryParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
    pub order_id: Option<i32>,
}
