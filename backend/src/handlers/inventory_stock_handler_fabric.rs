//! 库存处理器：面料库存业务（list_stock_fabric + create_stock_fabric）
//!
//! 拆分自 inventory_stock_handler.rs：原 2 个面料 fn 独立成文件。

use crate::middleware::auth_context::AuthContext;
use crate::services::inventory_stock_service::InventoryStockService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Query, State},
    Json,
};
use rust_decimal::Decimal;
use sea_orm::EntityTrait;

use super::inventory_stock_handler_dto::{
    CreateStockFabricRequest, ListStockFabricParams, StockFabricResponse,
};

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
