
use crate::middleware::auth_context::AuthContext;
use crate::models::product;
use crate::services::inventory_stock_service::InventoryStockService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use validator::Validate as _; // 仅用于 `ListStockParams::validate()` 方法解析

use super::inventory_stock_handler_dto::{
    CreateStockFabricRequest, ListStockParams, LowStockParams, StockResponse,
    UpdateStockWithVersionRequest,
};

pub async fn get_stock(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());

    let stock = service
        .find_by_id(id)
        .await
        .map_err(|e| AppError::not_found(e.to_string()))?;

    let response = StockResponse {
        id: stock.id,
        warehouse_id: stock.warehouse_id,
        product_id: stock.product_id,
        quantity_on_hand: stock.quantity_on_hand,
        quantity_available: stock.quantity_available,
        quantity_reserved: stock.quantity_reserved,
        reorder_point: stock.reorder_point,
        bin_location: stock.bin_location,
        created_at: stock.created_at,
        updated_at: stock.updated_at,
    };

    let mut response_json = serde_json::to_value(response)?;

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        if let Ok(Some(permission)) = state
            .data_permission_service
            .get_role_data_permission(role_id, "inventory_stock")
            .await
        {
            state.data_permission_service.filter_fields(
                &mut response_json,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
        } else if role_id != 1 {
            // 如果没有配置数据权限且不是管理员，使用默认字段隐藏
            if let Some(obj) = response_json.as_object_mut() {
                obj.remove("quantity_on_hand");
                obj.remove("quantity_available");
                obj.remove("quantity_reserved");
                obj.remove("reorder_point");
                obj.remove("reorder_quantity");
            }
        }
    }

    Ok(Json(ApiResponse::success(response_json)))
}

pub async fn create_stock(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(payload): Json<CreateStockFabricRequest>,
) -> Result<Json<ApiResponse<StockResponse>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());

    let stock = service
        .create_stock(
            payload.warehouse_id,
            payload.product_id,
            payload.batch_no,
            payload.color_no,
            payload.quantity_meters,
            payload.quantity_kg.unwrap_or(Decimal::ZERO),
            payload.grade,
            payload.dye_lot_no,
            payload.gram_weight,
            payload.width,
            "active".to_string(),
            "qualified".to_string(),
        )
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(ApiResponse::success(StockResponse {
        id: stock.id,
        warehouse_id: stock.warehouse_id,
        product_id: stock.product_id,
        quantity_on_hand: stock.quantity_on_hand,
        quantity_available: stock.quantity_available,
        quantity_reserved: stock.quantity_reserved,
        reorder_point: stock.reorder_point,
        bin_location: stock.bin_location,
        created_at: stock.created_at,
        updated_at: stock.updated_at,
    })))
}

pub async fn update_stock(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateStockWithVersionRequest>,
) -> Result<Json<ApiResponse<StockResponse>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());

    let stock = service
        .find_by_id(id)
        .await
        .map_err(|e| AppError::not_found(e.to_string()))?;

    // Optimistic lock check
    if stock.version != payload.version {
        return Err(AppError::business(
            "库存记录已被其他用户修改，请刷新后重试".to_string(),
        ));
    }

    use sea_orm::{ActiveModelTrait, Set};
    let mut active_model: crate::models::inventory_stock::ActiveModel = stock.into();

    if let Some(qoh) = payload.quantity_on_hand {
        active_model.quantity_on_hand = Set(qoh);
    }
    if let Some(qavail) = payload.quantity_available {
        active_model.quantity_available = Set(qavail);
    }
    if let Some(qres) = payload.quantity_reserved {
        active_model.quantity_reserved = Set(qres);
    }
    if let Some(rop) = payload.reorder_point {
        active_model.reorder_point = Set(rop);
    }
    if let Some(roq) = payload.reorder_quantity {
        active_model.reorder_quantity = Set(roq);
    }
    if let Some(bl) = payload.bin_location {
        active_model.bin_location = Set(Some(bl));
    }
    active_model.version = Set(payload.version + 1);
    active_model.updated_at = Set(Utc::now());

    let updated = active_model
        .update(&*state.db)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(ApiResponse::success(StockResponse {
        id: updated.id,
        warehouse_id: updated.warehouse_id,
        product_id: updated.product_id,
        quantity_on_hand: updated.quantity_on_hand,
        quantity_available: updated.quantity_available,
        quantity_reserved: updated.quantity_reserved,
        reorder_point: updated.reorder_point,
        bin_location: updated.bin_location,
        created_at: updated.created_at,
        updated_at: updated.updated_at,
    })))
}

pub async fn delete_stock(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());

    service
        .find_by_id(id)
        .await
        .map_err(|e| AppError::not_found(e.to_string()))?;

    service
        .delete_stock(id, Some(auth.user_id))
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}

pub async fn list_stock(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<ListStockParams>,
) -> Result<Json<ApiResponse<PaginatedResponse<serde_json::Value>>>, AppError> {
    if let Err(e) = params.validate() {
        return Err(AppError::validation(e.to_string()));
    }

    let service = InventoryStockService::new(state.db.clone());

    // 页码采用 1-based 约定，由 service 内部转换为 0-based
    let page = params.page.unwrap_or(1).max(1); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    let (stock_list, total) = service
        .list_stock(page, page_size, params.warehouse_id, params.product_id)
        .await?;

    let stock_responses: Vec<StockResponse> = stock_list
        .into_iter()
        .map(|stock| StockResponse {
            id: stock.id,
            warehouse_id: stock.warehouse_id,
            product_id: stock.product_id,
            quantity_on_hand: stock.quantity_on_hand,
            quantity_available: stock.quantity_available,
            quantity_reserved: stock.quantity_reserved,
            reorder_point: stock.reorder_point,
            bin_location: stock.bin_location,
            created_at: stock.created_at,
            updated_at: stock.updated_at,
        })
        .collect();

    // 发送库存预警通知
    if let Some(ref event_service) = state.event_notification_service {
        // 批量查询低于 reorder_point 的库存对应的 product，避免循环内 N+1 查询
        let alert_product_ids: Vec<i32> = stock_responses
            .iter()
            .filter(|stock| stock.quantity_on_hand < stock.reorder_point)
            .map(|stock| stock.product_id)
            .collect();
        let product_map: std::collections::HashMap<i32, product::Model> =
            if alert_product_ids.is_empty() {
                std::collections::HashMap::new()
            } else {
                // 沿用原逻辑：DB 错误时静默跳过（unwrap_or_default 返回空集合）
                let products = product::Entity::find()
                    .filter(product::Column::Id.is_in(alert_product_ids))
                    .all(&*state.db)
                    .await
                    .unwrap_or_default();
                products.into_iter().map(|p| (p.id, p)).collect()
            };

        // v17 批次 47 修复：循环外预先查询 user_id=0 的通知设置（1 次查询），
        // 避免循环内每个产品都查一次设置（N+1 查询）
        let setting = event_service.get_setting_for_user(0).await.ok();

        for stock in &stock_responses {
            if stock.quantity_on_hand < stock.reorder_point {
                if let Some(product) = product_map.get(&stock.product_id) {
                    if let Some(ref s) = setting {
                        // 批次 94 P2-11：原 let _ = 静默吞错，通知发送失败时无任何日志，改为 warn 日志记录
                        if let Err(e) = event_service
                            .notify_inventory_alert_with_setting(
                                0, // 系统通知，不指定特定用户
                                s,
                                &product.name,
                                product.id,
                                &stock.quantity_on_hand.to_string(),
                                &stock.reorder_point.to_string(),
                            )
                            .await
                        {
                            tracing::warn!("批次 94 P2-11：库存预警通知(with_setting)发送失败: {}", e);
                        }
                    } else {
                        // 设置查询失败时回退到原方法（兼容性保留）
                        // 批次 94 P2-11：原 let _ = 静默吞错，通知发送失败时无任何日志，改为 warn 日志记录
                        if let Err(e) = event_service
                            .notify_inventory_alert(
                                0,
                                &product.name,
                                product.id,
                                &stock.quantity_on_hand.to_string(),
                                &stock.reorder_point.to_string(),
                            )
                            .await
                        {
                            tracing::warn!("批次 94 P2-11：库存预警通知发送失败: {}", e);
                        }
                    }
                }
            }
        }
    }

    let mut stock_json: Vec<serde_json::Value> = stock_responses
        .into_iter()
        .map(|s| serde_json::to_value(s).unwrap_or_default())
        .collect();

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        if let Ok(Some(permission)) = state
            .data_permission_service
            .get_role_data_permission(role_id, "inventory_stock")
            .await
        {
            state.data_permission_service.filter_fields_batch(
                &mut stock_json,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
        } else if role_id != 1 {
            // 如果没有配置数据权限且不是管理员，使用默认字段隐藏
            for stock in &mut stock_json {
                if let Some(obj) = stock.as_object_mut() {
                    obj.remove("quantity_on_hand");
                    obj.remove("quantity_available");
                    obj.remove("quantity_reserved");
                    obj.remove("reorder_point");
                    obj.remove("reorder_quantity");
                }
            }
        }
    }

    Ok(Json(crate::utils::response::ApiResponse::success(
        PaginatedResponse::new(stock_json, total, page, page_size),
    )))
}

pub async fn check_low_stock(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<LowStockParams>,
) -> Result<Json<crate::utils::response::ApiResponse<Vec<StockResponse>>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());

    let stock_list = service
        .check_low_stock(params.warehouse_id, params.product_id, params.batch_no)
        .await?;

    let stock_responses: Vec<StockResponse> = stock_list
        .into_iter()
        .map(|stock| StockResponse {
            id: stock.id,
            warehouse_id: stock.warehouse_id,
            product_id: stock.product_id,
            quantity_on_hand: stock.quantity_on_hand,
            quantity_available: stock.quantity_available,
            quantity_reserved: stock.quantity_reserved,
            reorder_point: stock.reorder_point,
            bin_location: stock.bin_location,
            created_at: stock.created_at,
            updated_at: stock.updated_at,
        })
        .collect();

    // 发送库存预警通知
    if let Some(ref event_service) = state.event_notification_service {
        // 批量查询 product，避免循环内 N+1 查询
        let product_ids: Vec<i32> = stock_responses
            .iter()
            .map(|stock| stock.product_id)
            .collect();
        let product_map: std::collections::HashMap<i32, product::Model> =
            if product_ids.is_empty() {
                std::collections::HashMap::new()
            } else {
                // 沿用原逻辑：DB 错误时静默跳过（unwrap_or_default 返回空集合）
                let products = product::Entity::find()
                    .filter(product::Column::Id.is_in(product_ids))
                    .all(&*state.db)
                    .await
                    .unwrap_or_default();
                products.into_iter().map(|p| (p.id, p)).collect()
            };

        // v17 批次 47 修复：循环外预先查询 user_id=0 的通知设置（1 次查询），
        // 避免循环内每个产品都查一次设置（N+1 查询）
        let setting = event_service.get_setting_for_user(0).await.ok();

        for stock in &stock_responses {
            if let Some(product) = product_map.get(&stock.product_id) {
                if let Some(ref s) = setting {
                    // 批次 94 P2-11：原 let _ = 静默吞错，通知发送失败时无任何日志，改为 warn 日志记录
                    if let Err(e) = event_service
                        .notify_inventory_alert_with_setting(
                            0,
                            s,
                            &product.name,
                            product.id,
                            &stock.quantity_on_hand.to_string(),
                            &stock.reorder_point.to_string(),
                        )
                        .await
                    {
                        tracing::warn!("批次 94 P2-11：库存预警通知(with_setting)发送失败: {}", e);
                    }
                } else {
                    // 设置查询失败时回退到原方法（兼容性保留）
                    // 批次 94 P2-11：原 let _ = 静默吞错，通知发送失败时无任何日志，改为 warn 日志记录
                    if let Err(e) = event_service
                        .notify_inventory_alert(
                            0,
                            &product.name,
                            product.id,
                            &stock.quantity_on_hand.to_string(),
                            &stock.reorder_point.to_string(),
                        )
                        .await
                    {
                        tracing::warn!("批次 94 P2-11：库存预警通知发送失败: {}", e);
                    }
                }
            }
        }
    }

    Ok(Json(crate::utils::response::ApiResponse::success(
        stock_responses,
    )))
}
