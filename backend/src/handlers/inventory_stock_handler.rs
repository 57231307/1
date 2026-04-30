use crate::services::inventory_stock_service::InventoryStockService;
use crate::utils::dual_unit_converter::DualUnitConverter;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use rust_decimal::Decimal;
use crate::utils::app_state::AppState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct UpdateStockRequest {
    pub quantity_on_hand: Option<Decimal>,
    pub quantity_available: Option<Decimal>,
    pub quantity_reserved: Option<Decimal>,
    pub reorder_point: Option<Decimal>,
    pub reorder_quantity: Option<Decimal>,
    pub bin_location: Option<String>,
}

/// 创建库存请求（面料行业版）
#[derive(Debug, Deserialize)]
pub struct CreateStockFabricRequest {
    pub warehouse_id: i32,
    pub product_id: i32,
    /// 批次号
    pub batch_no: String,
    /// 色号
    pub color_no: String,
    /// 缸号
    pub dye_lot_no: Option<String>,
    /// 等级
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
    pub shelf_no: Option<String>,
    /// 层号
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
    Path(id): Path<i32>,
) -> Result<Json<StockResponse>, (StatusCode, String)> {
    let service = InventoryStockService::new(state.db.clone());

    match service.find_by_id(id).await {
        Ok(stock) => Ok(Json(StockResponse {
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
        })),
        Err(e) => Err((StatusCode::NOT_FOUND, e.to_string())),
    }
}

pub async fn create_stock(
    State(state): State<AppState>,
    Json(payload): Json<CreateStockFabricRequest>,
) -> Result<Json<StockResponse>, (StatusCode, String)> {
    let service = InventoryStockService::new(state.db.clone());

    match service
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
    {
        Ok(stock) => Ok(Json(StockResponse {
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
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn update_stock(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateStockRequest>,
) -> Result<Json<StockResponse>, (StatusCode, String)> {
    let service = InventoryStockService::new(state.db.clone());

    let stock = match service.find_by_id(id).await {
        Ok(s) => s,
        Err(e) => return Err((StatusCode::NOT_FOUND, e.to_string())),
    };

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
    active_model.updated_at = Set(Utc::now());

    match active_model.update(&*state.db).await {
        Ok(updated) => Ok(Json(StockResponse {
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
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn delete_stock(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<()>, (StatusCode, String)> {
    let service = InventoryStockService::new(state.db.clone());

    match service.find_by_id(id).await {
        Ok(_) => {}
        Err(e) => return Err((StatusCode::NOT_FOUND, e.to_string())),
    }

    use sea_orm::EntityTrait;
    match crate::models::inventory_stock::Entity::delete_by_id(id)
        .exec(&*state.db)
        .await
    {
        Ok(_) => Ok(Json(())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn list_stock(
    State(state): State<AppState>,
    Query(params): Query<ListStockParams>,
) -> Result<Json<StockListResponse>, (StatusCode, String)> {
    let service = InventoryStockService::new(state.db.clone());

    match service
        .list_stock(
            params.page.unwrap_or(0),
            params.page_size.unwrap_or(20),
            params.warehouse_id,
            params.product_id,
        )
        .await
    {
        Ok((stock_list, total)) => {
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

            Ok(Json(StockListResponse {
                stock: stock_responses,
                total,
                page: params.page.unwrap_or(0),
                page_size: params.page_size.unwrap_or(20),
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn check_low_stock(
    State(state): State<AppState>,
    Query(params): Query<LowStockParams>,
) -> Result<Json<LowStockResponse>, (StatusCode, String)> {
    let service = InventoryStockService::new(state.db.clone());

    match service.check_low_stock(params.warehouse_id, params.product_id, params.batch_no).await {
        Ok(stock_list) => {
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

            let count = stock_responses.len() as u64;

            Ok(Json(LowStockResponse {
                products: stock_responses,
                count,
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

#[derive(Debug, Deserialize)]
pub struct ListStockParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub warehouse_id: Option<i32>,
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
    Query(params): Query<ListStockFabricParams>,
) -> Result<Json<StockFabricListResponse>, (StatusCode, String)> {
    let service = InventoryStockService::new(state.db.clone());

    let page = params.page.unwrap_or(0);
    let page_size = params.page_size.unwrap_or(20);

    match service
        .find_by_batch_and_color(
            &params.batch_no.unwrap_or_default(),
            &params.color_no.unwrap_or_default(),
            params.warehouse_id,
        )
        .await
    {
        Ok(stock_list) => {
            let total = stock_list.len() as u64;
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

            Ok(Json(StockFabricListResponse {
                stock: stock_responses,
                total,
                page,
                page_size,
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 查询库存流水
pub async fn list_transactions(
    State(state): State<AppState>,
    Query(params): Query<ListTransactionParams>,
) -> Result<Json<TransactionListResponse>, (StatusCode, String)> {
    let service = InventoryStockService::new(state.db.clone());

    let page = params.page.unwrap_or(0);
    let page_size = params.page_size.unwrap_or(20);

    match service
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
        .await
    {
        Ok((transactions, total)) => {
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

            Ok(Json(TransactionListResponse {
                transactions: transaction_responses,
                total,
                page,
                page_size,
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 获取库存汇总（按批次 + 色号）
pub async fn get_inventory_summary(
    State(state): State<AppState>,
    Query(params): Query<ListStockFabricParams>,
) -> Result<Json<InventorySummaryResponse>, (StatusCode, String)> {
    let service = InventoryStockService::new(state.db.clone());

    match service
        .get_inventory_summary(
            params.warehouse_id,
            params.product_id,
            params.batch_no,
            params.color_no,
            params.grade,
        )
        .await
    {
        Ok(summary_items) => {
            let mut total_meters = rust_decimal::Decimal::ZERO;
            let mut total_kg = rust_decimal::Decimal::ZERO;

            let summary: Vec<InventorySummaryItem> = summary_items
                .into_iter()
                .map(|item| {
                    total_meters += item.total_quantity_meters;
                    total_kg += item.total_quantity_kg;
                    InventorySummaryItem {
                        product_id: item.product_id,
                        product_name: item.product_name,
                        batch_no: item.batch_no,
                        color_no: item.color_no,
                        grade: item.grade,
                        total_quantity_meters: item.total_quantity_meters,
                        total_quantity_kg: item.total_quantity_kg,
                        warehouse_name: item.warehouse_name,
                    }
                })
                .collect();

            Ok(Json(InventorySummaryResponse {
                summary,
                total_meters,
                total_kg,
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

use axum::extract::Query;

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
    Json(payload): Json<CreateStockFabricRequest>,
) -> Result<Json<StockFabricResponse>, (StatusCode, String)> {
    let service = InventoryStockService::new(state.db.clone());

    // 如果提供了克重和幅宽，自动计算公斤数
    let quantity_kg = if let (Some(gram_weight), Some(width)) = (payload.gram_weight, payload.width)
    {
        match DualUnitConverter::meters_to_kg(payload.quantity_meters, gram_weight, width) {
            Ok(kg) => kg,
            Err(e) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("双计量单位换算失败：{}", e),
                ))
            }
        }
    } else {
        // 如果没有提供克重和幅宽，使用传入的公斤数或默认为 0
        payload.quantity_kg.unwrap_or(Decimal::ZERO)
    };

    match service
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
    {
        Ok(stock) => Ok(Json(StockFabricResponse {
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
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
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
