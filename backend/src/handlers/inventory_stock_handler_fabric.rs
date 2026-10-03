//! 库存处理器：面料库存业务（list_stock_fabric + create_stock_fabric）
//!
//! 拆分自 inventory_stock_handler.rs：原 2 个面料 fn 独立成文件。
//! 另含 `admit_stock_fabric_trace`——两条库存直建端点（/inventory/stock 与
//! /inventory/stock/fabric）共用的白坯/染色准入收口，判定权威在
//! services::inv::fabric_class::validate_fabric_trace。

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::inv::fabric_class::{self, FabricTrace};
use crate::services::inventory_stock_service::{CreateStockFabricArgs, InventoryStockService};
use crate::utils::dual_unit_converter::DualUnitConverter;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    Json,
    extract::{Query, State},
};
use rust_decimal::Decimal;
use validator::Validate;

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
    // 输入验证：DTO 校验拒绝只回显用户自己提交的字段规则，走 From<ValidationErrors>
    // 的可读外显链路（不得手工 AppError::validation——那会把原因压成"请求参数验证失败"，
    // 用户看不到"批次不得为空"，也就永远到不了下方 fabric_class 的判定文案）
    payload.validate().map_err(AppError::from)?;

    // 白坯/染色追溯口径（色号/缸号/批次）准入：委托唯一权威判定，见 helper 文档
    let trace =
        admit_stock_fabric_trace(payload.color_no, payload.dye_lot_no, Some(payload.batch_no))?;

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
        .create_stock_fabric(CreateStockFabricArgs {
            warehouse_id: payload.warehouse_id,
            product_id: payload.product_id,
            batch_no: trace.batch_no,
            color_no: trace.color_no,
            dye_lot_no: trace.dye_lot_no,
            grade: payload.grade,
            quantity_meters: payload.quantity_meters,
            quantity_kg,
            gram_weight: payload.gram_weight,
            width: payload.width,
            location_id: payload.location_id,
            shelf_no: payload.shelf_no,
            layer_no: payload.layer_no,
        })
        .await?;

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

/// 面料库存直建（POST /inventory/stock 与 POST /inventory/stock/fabric 两条端点共用）
/// 写入前的四维追溯准入。
///
/// 判定本身不在这里重写：唯一权威是 `services::inv::fabric_class::validate_fabric_trace`
/// （色号为空=白坯免缸号、色号非空=染色布缸号+批次必填、批次任何布种必填、trim 归一、
/// 禁止按色号文本嗅探布种），本函数只做两点端侧收口：
/// 1. 拒绝文案外显：validate_fabric_trace 产出的校验错误只回显用户自己提交的色号与
///    公开业务规则，满足 `business_displayable` 安全边界（见 utils::error 模块文档）；
///    若不转则 ValidationError 出参被脱敏成"请求参数验证失败"，用户在界面上看不到原因。
///    其余变体原样传播，不强转 500。
/// 2. 白坯主动把缸号归一为 None：validate_fabric_trace 对白坯仅"免缸号"（不强制为空），
///    而出库规划 `services::inventory_deduction` 对白坯只接受 `dye_lot_no IS NULL` 的行、
///    绝不跨缸回退——若放行"白坯带缸号"入库，该行将永久提不出来。此处收口与出库同源。
pub fn admit_stock_fabric_trace(
    color_no: Option<String>,
    dye_lot_no: Option<String>,
    batch_no: Option<String>,
) -> Result<FabricTrace, AppError> {
    let trace = fabric_class::validate_fabric_trace(color_no, dye_lot_no, batch_no).map_err(
        |e| match e {
            AppError::ValidationError(msg) | AppError::ValidationErrorDisplayable(msg) => {
                AppError::business_displayable(msg)
            }
            other => other,
        },
    )?;
    if trace.color_no.is_empty() {
        // 白坯布：缸号归一为 None（白坯不具缸号属性，落库 dye_lot_no IS NULL 才能被出库规划命中）
        Ok(FabricTrace {
            dye_lot_no: None,
            ..trace
        })
    } else {
        Ok(trace)
    }
}
