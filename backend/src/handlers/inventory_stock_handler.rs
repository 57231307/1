#![allow(dead_code)]

use crate::middleware::auth_context::AuthContext;
use crate::models::dto::PageRequest;
use crate::models::product;
use crate::services::inventory_stock_service::InventoryStockService;
use crate::utils::app_state::AppState;
use crate::utils::dual_unit_converter::DualUnitConverter;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateStockRequest {
    pub quantity_on_hand: Option<Decimal>,
    pub quantity_available: Option<Decimal>,
    pub quantity_reserved: Option<Decimal>,
    pub reorder_point: Option<Decimal>,
    pub reorder_quantity: Option<Decimal>,
    #[validate(length(max = 100, message = "库位长度不能超过100个字符"))]
    pub bin_location: Option<String>,
}

/// 创建库存请求（面料行业版）
#[derive(Debug, Deserialize, Validate)]
pub struct CreateStockFabricRequest {
    #[validate(range(min = 1, message = "仓库ID必须大于0"))]
    pub warehouse_id: i32,
    #[validate(range(min = 1, message = "产品ID必须大于0"))]
    pub product_id: i32,
    /// 批次号
    #[validate(length(min = 1, max = 50, message = "批次号长度必须在1-50个字符之间"))]
    pub batch_no: String,
    /// 色号
    #[validate(length(min = 1, max = 50, message = "色号长度必须在1-50个字符之间"))]
    pub color_no: String,
    /// 缸号
    #[validate(length(max = 50, message = "缸号长度不能超过50个字符"))]
    pub dye_lot_no: Option<String>,
    /// 等级
    #[validate(length(min = 1, max = 20, message = "等级长度必须在1-20个字符之间"))]
    pub grade: String,
    /// 数量（米）
    pub quantity_meters: Decimal,
    /// 数量（公斤）- 可选，会自动计算
    pub quantity_kg: Option<Decimal>,
    /// 克重 (g/m²)
    pub gram_weight: Option<Decimal>,
    /// 幅宽 (cm)
    pub width: Option<Decimal>,
    /// 库位 ID
    pub location_id: Option<i32>,
    /// 货架号
    #[validate(length(max = 50, message = "货架号长度不能超过50个字符"))]
    pub shelf_no: Option<String>,
    /// 层号
    #[validate(length(max = 50, message = "层号长度不能超过50个字符"))]
    pub layer_no: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StockResponse {
    pub id: i32,
    pub warehouse_id: i32,
    pub product_id: i32,
    pub quantity_on_hand: Decimal,
    pub quantity_available: Decimal,
    pub quantity_reserved: Decimal,
    pub reorder_point: Decimal,
    pub bin_location: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct StockListResponse {
    pub stock: Vec<StockResponse>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Serialize)]
pub struct LowStockResponse {
    pub products: Vec<StockResponse>,
    pub count: u64,
}

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

#[derive(Debug, Deserialize)]
pub struct UpdateStockWithVersionRequest {
    pub quantity_on_hand: Option<Decimal>,
    pub quantity_available: Option<Decimal>,
    pub quantity_reserved: Option<Decimal>,
    pub reorder_point: Option<Decimal>,
    pub reorder_quantity: Option<Decimal>,
    pub bin_location: Option<String>,
    pub version: i32,
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
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());

    service
        .find_by_id(id)
        .await
        .map_err(|e| AppError::not_found(e.to_string()))?;

    service
        .delete_stock(id)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}

pub async fn list_stock(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<ListStockParams>,
) -> Result<Json<crate::utils::response::ApiResponse<Vec<serde_json::Value>>>, AppError> {
    if let Err(e) = params.validate() {
        return Err(AppError::validation(e.to_string()));
    }

    let service = InventoryStockService::new(state.db.clone());

    let (stock_list, _total) = service
        .list_stock(
            params.page.unwrap_or(0),
            params.page_size.unwrap_or(20),
            params.warehouse_id,
            params.product_id,
        )
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
        for stock in &stock_responses {
            if stock.quantity_on_hand < stock.reorder_point {
                if let Ok(Some(product)) = product::Entity::find_by_id(stock.product_id)
                    .one(&*state.db)
                    .await
                {
                    let _ = event_service
                        .notify_inventory_alert(
                            0, // 系统通知，不指定特定用户
                            &product.name,
                            product.id,
                            &stock.quantity_on_hand.to_string(),
                            &stock.reorder_point.to_string(),
                        )
                        .await;
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
        stock_json,
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
        for stock in &stock_responses {
            if let Ok(Some(product)) = product::Entity::find_by_id(stock.product_id)
                .one(&*state.db)
                .await
            {
                let _ = event_service
                    .notify_inventory_alert(
                        0,
                        &product.name,
                        product.id,
                        &stock.quantity_on_hand.to_string(),
                        &stock.reorder_point.to_string(),
                    )
                    .await;
            }
        }
    }

    Ok(Json(crate::utils::response::ApiResponse::success(
        stock_responses,
    )))
}

#[derive(Debug, Deserialize, Validate)]
pub struct ListStockParams {
    #[validate(range(min = 0, message = "页码不能为负数"))]
    pub page: Option<u64>,
    #[validate(range(min = 1, max = 100, message = "每页数量必须在1-100之间"))]
    pub page_size: Option<u64>,
    #[validate(range(min = 1, message = "仓库ID必须大于0"))]
    pub warehouse_id: Option<i32>,
    #[validate(range(min = 1, message = "产品ID必须大于0"))]
    pub product_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct LowStockParams {
    pub warehouse_id: Option<i32>,
    pub product_id: Option<i32>,
    pub batch_no: Option<String>,
}

// ========== 面料行业库存管理接口 ==========

/// 按批次 + 色号查询库存（面料行业版）
pub async fn list_stock_fabric(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ListStockFabricParams>,
) -> Result<Json<crate::utils::response::ApiResponse<Vec<StockFabricResponse>>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());

    let stock_list = service
        .find_by_batch_and_color(
            &params.batch_no.unwrap_or_default(),
            &params.color_no.unwrap_or_default(),
            params.warehouse_id,
        )
        .await?;

    let stock_responses: Vec<StockFabricResponse> = stock_list
        .into_iter()
        .map(|stock| StockFabricResponse {
            id: stock.id,
            warehouse_id: stock.warehouse_id,
            product_id: stock.product_id,
            batch_no: stock.batch_no,
            color_no: stock.color_no,
            dye_lot_no: stock.dye_lot_no,
            grade: stock.grade,
            quantity_on_hand: stock.quantity_on_hand,
            quantity_available: stock.quantity_available,
            quantity_reserved: stock.quantity_reserved,
            quantity_meters: stock.quantity_meters,
            quantity_kg: stock.quantity_kg,
            gram_weight: stock.gram_weight,
            width: stock.width,
            bin_location: stock.bin_location,
            created_at: stock.created_at,
            updated_at: stock.updated_at,
        })
        .collect();

    Ok(Json(crate::utils::response::ApiResponse::success(
        stock_responses,
    )))
}

/// 查询库存流水
pub async fn list_transactions(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ListTransactionParams>,
) -> Result<Json<crate::utils::response::ApiResponse<Vec<TransactionResponse>>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());

    let page = params.page.unwrap_or(0);
    let page_size = params.page_size.unwrap_or(20);

    let (transactions, _total) = service
        .list_transactions(
            page,
            page_size,
            params.batch_no,
            params.color_no,
            params.product_id,
            params.warehouse_id,
            params.transaction_type,
            params.start_date,
            params.end_date,
        )
        .await?;

    let transaction_responses: Vec<TransactionResponse> = transactions
        .into_iter()
        .map(|txn| TransactionResponse {
            id: txn.id,
            transaction_type: txn.transaction_type,
            product_id: txn.product_id,
            warehouse_id: txn.warehouse_id,
            batch_no: txn.batch_no,
            color_no: txn.color_no,
            quantity_meters: txn.quantity_meters,
            quantity_kg: txn.quantity_kg,
            quantity_before_meters: txn.quantity_before_meters.unwrap_or(Decimal::ZERO),
            quantity_before_kg: txn.quantity_before_kg.unwrap_or(Decimal::ZERO),
            quantity_after_meters: txn.quantity_after_meters.unwrap_or(Decimal::ZERO),
            quantity_after_kg: txn.quantity_after_kg.unwrap_or(Decimal::ZERO),
            source_bill_type: txn.source_bill_type,
            source_bill_no: txn.source_bill_no,
            remarks: txn.notes,
            created_at: txn.created_at,
        })
        .collect();

    Ok(Json(crate::utils::response::ApiResponse::success(
        transaction_responses,
    )))
}

/// 获取库存汇总（按批次 + 色号）
pub async fn get_inventory_summary(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ListStockFabricParams>,
) -> Result<Json<crate::utils::response::ApiResponse<Vec<InventorySummaryItem>>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());

    let summary_items = service
        .get_inventory_summary(
            params.warehouse_id,
            params.product_id,
            params.batch_no,
            params.color_no,
            params.grade,
        )
        .await?;

    let summary: Vec<InventorySummaryItem> = summary_items
        .into_iter()
        .map(|item| InventorySummaryItem {
            product_id: item.product_id,
            product_name: item.product_name,
            batch_no: item.batch_no,
            color_no: item.color_no,
            grade: item.grade,
            total_quantity_meters: item.total_quantity_meters,
            total_quantity_kg: item.total_quantity_kg,
            warehouse_name: item.warehouse_name,
        })
        .collect();

    Ok(Json(crate::utils::response::ApiResponse::success(summary)))
}

#[derive(Debug, Deserialize)]
pub struct ListStockFabricParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub warehouse_id: Option<i32>,
    pub product_id: Option<i32>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub grade: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StockFabricResponse {
    pub id: i32,
    pub warehouse_id: i32,
    pub product_id: i32,
    pub batch_no: String,
    pub color_no: String,
    pub dye_lot_no: Option<String>,
    pub grade: String,
    pub quantity_on_hand: Decimal,
    pub quantity_available: Decimal,
    pub quantity_reserved: Decimal,
    pub quantity_meters: Decimal,
    pub quantity_kg: Decimal,
    pub gram_weight: Option<Decimal>,
    pub width: Option<Decimal>,
    pub bin_location: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct StockFabricListResponse {
    pub stock: Vec<StockFabricResponse>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Deserialize)]
pub struct ListTransactionParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub product_id: Option<i32>,
    pub warehouse_id: Option<i32>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub transaction_type: Option<String>,
    pub start_date: Option<chrono::NaiveDateTime>,
    pub end_date: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Serialize)]
pub struct TransactionResponse {
    pub id: i32,
    pub transaction_type: String,
    pub product_id: i32,
    pub warehouse_id: i32,
    pub batch_no: String,
    pub color_no: String,
    pub quantity_meters: Decimal,
    pub quantity_kg: Decimal,
    pub quantity_before_meters: Decimal,
    pub quantity_before_kg: Decimal,
    pub quantity_after_meters: Decimal,
    pub quantity_after_kg: Decimal,
    pub source_bill_type: Option<String>,
    pub source_bill_no: Option<String>,
    pub remarks: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct TransactionListResponse {
    pub transactions: Vec<TransactionResponse>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Serialize)]
pub struct InventorySummaryItem {
    pub product_id: i32,
    pub product_name: String,
    pub batch_no: String,
    pub color_no: String,
    pub grade: String,
    pub total_quantity_meters: Decimal,
    pub total_quantity_kg: Decimal,
    pub warehouse_name: String,
}

#[derive(Debug, Serialize)]
pub struct InventorySummaryResponse {
    pub summary: Vec<InventorySummaryItem>,
    pub total_meters: Decimal,
    pub total_kg: Decimal,
}

// ========== 面料行业双计量单位优化接口 ==========

/// 创建面料库存（双计量单位自动换算版）
///
/// # 请求示例
/// ```json
/// {
///     "warehouse_id": 1,
///     "product_id": 100,
///     "batch_no": "B20240101",
///     "color_no": "C001",
///     "dye_lot_no": "D20240101001",
///     "grade": "一等品",
///     "quantity_meters": "100.00",
///     "gram_weight": "180.00",
///     "width": "180.00",
///     "location_id": 1,
///     "shelf_no": "A01",
///     "layer_no": "01"
/// }
/// ```
///
/// # 说明
/// - 如果提供了 `gram_weight` 和 `width`，系统会自动计算 `quantity_kg`
/// - 计算公式：公斤数 = 米数 × 克重 (g/m²) × 幅宽 (m) ÷ 1000
/// - 如果同时提供了 `quantity_kg`，将使用自动计算的值（更精确）
pub async fn create_stock_fabric(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(payload): Json<CreateStockFabricRequest>,
) -> Result<Json<ApiResponse<StockFabricResponse>>, AppError> {
    // 输入验证
    if let Err(e) = payload.validate() {
        return Err(AppError::validation(e.to_string()));
    }

    let service = InventoryStockService::new(state.db.clone());

    // 如果提供了克重和幅宽，自动计算公斤数
    let quantity_kg = if let (Some(gram_weight), Some(width)) = (payload.gram_weight, payload.width)
    {
        DualUnitConverter::meters_to_kg(payload.quantity_meters, gram_weight, width)
            .map_err(|e| AppError::validation(format!("双计量单位换算失败：{}", e)))?
    } else {
        // 如果没有提供克重和幅宽，使用传入的公斤数或默认为 0
        payload.quantity_kg.unwrap_or(Decimal::ZERO)
    };

    let stock = service
        .create_stock_fabric(
            payload.warehouse_id,
            payload.product_id,
            payload.batch_no,
            payload.color_no,
            payload.dye_lot_no,
            payload.grade,
            payload.quantity_meters,
            quantity_kg,
            payload.gram_weight,
            payload.width,
            payload.location_id,
            payload.shelf_no,
            payload.layer_no,
        )
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(ApiResponse::success(StockFabricResponse {
        id: stock.id,
        warehouse_id: stock.warehouse_id,
        product_id: stock.product_id,
        batch_no: stock.batch_no,
        color_no: stock.color_no,
        dye_lot_no: stock.dye_lot_no,
        grade: stock.grade,
        quantity_on_hand: stock.quantity_on_hand,
        quantity_available: stock.quantity_available,
        quantity_reserved: stock.quantity_reserved,
        quantity_meters: stock.quantity_meters,
        quantity_kg: stock.quantity_kg,
        gram_weight: stock.gram_weight,
        width: stock.width,
        bin_location: stock.bin_location,
        created_at: stock.created_at,
        updated_at: stock.updated_at,
    })))
}

// ========== 库存查询增强接口 ==========

/// 按产品查询库存
/// GET /api/v1/erp/inventory/stock/product/:productId
pub async fn get_stock_by_product(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(product_id): Path<i32>,
    Query(query): Query<PageRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());

    let (stocks, total) = service
        .get_stock_by_product(product_id, query.page, query.page_size)
        .await
        .map_err(|e| AppError::internal(format!("查询产品库存失败: {}", e)))?;

    let result = serde_json::json!({
        "list": stocks,
        "total": total,
        "page": query.page,
        "page_size": query.page_size,
    });

    Ok(Json(ApiResponse::success(result)))
}

/// 获取库存告警
/// GET /api/v1/erp/inventory/stock/alerts
pub async fn get_stock_alerts(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());

    let alerts = service
        .get_stock_alerts(query)
        .await
        .map_err(|e| AppError::internal(format!("获取库存告警失败: {}", e)))?;

    Ok(Json(ApiResponse::success(alerts)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::*;

    #[test]
    fn test_create_stock_fabric_request_deserialize() {
        let json = r#"
        {
            "warehouse_id": 1,
            "product_id": 100,
            "batch_no": "B20240101",
            "color_no": "C001",
            "dye_lot_no": "D20240101001",
            "grade": "一等品",
            "quantity_meters": "100.00",
            "gram_weight": "180.00",
            "width": "180.00",
            "location_id": 1,
            "shelf_no": "A01",
            "layer_no": "01"
        }
        "#;

        let req: CreateStockFabricRequest =
            serde_json::from_str(json).expect("request json should deserialize");
        assert_eq!(req.warehouse_id, 1);
        assert_eq!(req.product_id, 100);
        assert_eq!(req.batch_no, "B20240101");
        assert_eq!(req.color_no, "C001");
        assert_eq!(
            req.quantity_meters,
            rust_decimal::Decimal::from_str("100.00").expect("decimal should parse")
        );
        assert_eq!(
            req.gram_weight,
            Some(rust_decimal::Decimal::from_str("180.00").expect("decimal should parse"))
        );
        assert_eq!(
            req.width,
            Some(rust_decimal::Decimal::from_str("180.00").expect("decimal should parse"))
        );
    }
}
