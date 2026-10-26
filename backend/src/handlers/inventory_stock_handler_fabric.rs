//! 库存处理器：面料库存业务（list_stock_fabric + create_stock_fabric）
//!
//! 拆分自 inventory_stock_handler.rs：原 2 个面料 fn 独立成文件。
//! 另含两条库存直建端点（/inventory/stock 与 /inventory/stock/fabric）共用的入库受理收口：
//! - `admit_stock_fabric_trace`——白坯/染色三维追溯收口，判定权威在
//!   services::inv::fabric_class::validate_fabric_trace；
//! - `admit_stock_fabric_measures`——数量、等级、双计量与来源单号的度量收口，判定权威在
//!   services::inv::inbound_admission；
//! - `stock_args_from_admitted`——把两组收口结果映射成落库参数（库存行 + 流水引用），
//!   两个 handler 共用同一映射，避免各自拼参数导致台账列与流水列口径分叉。

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::status::purchase_inventory::inventory_transaction_type;
use crate::services::inv::fabric_class::{self, FabricTrace};
use crate::services::inv::inbound_admission;
use crate::services::inventory_stock_service::{
    CreateStockFabricArgs, InventoryStockService, StockInboundFlowRef,
};
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

/// 入库受理侧度量准入通过后的结果（两条库存直建端点共用）
///
/// 只承载"已经过判定、可以直接落库"的值，字段与 `inventory_stocks` 与
/// `inventory_transactions` 的写入列一一对应。三维追溯值不在本结构内，由
/// [`admit_stock_fabric_trace`] 单独产出（判定族不同，权威也不同，不合并成一个上帝判定）。
#[derive(Debug, Clone)]
pub struct AdmittedStockMeasures {
    /// 落在库存等级词表内的等级
    pub grade: String,
    /// 主计量米数（大于 0）
    pub quantity_meters: Decimal,
    /// 辅计量公斤数（实称优先，缺失时由克重与幅宽经双计量单入口推算）
    pub quantity_kg: Decimal,
    /// 克重（g/m2，可缺）
    pub gram_weight: Option<Decimal>,
    /// 幅宽（cm，可缺）
    pub width: Option<Decimal>,
    /// 库位 ID（可缺）
    pub location_id: Option<i32>,
    /// 货架号（可缺）
    pub shelf_no: Option<String>,
    /// 层号（可缺）
    pub layer_no: Option<String>,
    /// 来源单据类型（缺省补 `STOCK_DIRECT_IN`）
    pub source_bill_type: String,
    /// 来源单据号（必填）
    pub source_bill_no: String,
    /// 入库备注
    pub notes: Option<String>,
}

/// 库存直建入库的度量准入（POST /inventory/stock 与 POST /inventory/stock/fabric 共用）。
///
/// 调用方是 `create_stock` 与 `create_stock_fabric` 两个 handler，入参是 DTO 引用；
/// 通过后与 [`admit_stock_fabric_trace`] 的归一追溯值一起交给 [`stock_args_from_admitted`]
/// 映射成落库参数，最终由 `InventoryStockService::create_stock_fabric_with_flow` 写库。
///
/// 判定本身不在本文件重写，一律委托入库准入的唯一实现
/// `services::inv::inbound_admission`（数量下限、等级取值域、克重幅宽为正、公斤数唯一解析、
/// 来源单号必填）。本函数只做编排，不新增规则。
///
/// 拒绝一律走 `validation_displayable`（VALIDATION_ERROR，400，文案可外显），点名缺的是哪个字段，
/// 不做任何缺省补值。数量、等级、公斤数、来源单号四项此前分别被静默塌成 0 米、任意字符串、
/// 0 公斤与无来源，入账后既查不回上游单据也无法按四维出库。
pub fn admit_stock_fabric_measures(
    payload: &CreateStockFabricRequest,
) -> Result<AdmittedStockMeasures, AppError> {
    inbound_admission::require_positive_quantity(payload.quantity_meters, "入库米数")?;
    let grade = inbound_admission::require_stock_grade(Some(payload.grade.as_str()), "等级")?;
    inbound_admission::require_positive_measure_if_present(payload.gram_weight, "克重")?;
    inbound_admission::require_positive_measure_if_present(payload.width, "幅宽")?;
    let quantity_kg = inbound_admission::resolve_inbound_quantity_kg(
        payload.quantity_meters,
        payload.gram_weight,
        payload.width,
        payload.quantity_kg,
        "库存直建入库",
    )?;
    let source_bill_no =
        inbound_admission::require_source_bill_no(payload.source_bill_no.as_deref(), "来源单号")?;

    // 来源单据类型缺省按本入口自己的 token 落，绝不借用其它链路的 token 冒充来源
    let source_bill_type = payload
        .source_bill_type
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| inventory_transaction_type::STOCK_DIRECT_IN.to_string());

    Ok(AdmittedStockMeasures {
        grade,
        quantity_meters: payload.quantity_meters,
        quantity_kg,
        gram_weight: payload.gram_weight,
        width: payload.width,
        location_id: payload.location_id,
        shelf_no: payload.shelf_no.clone(),
        layer_no: payload.layer_no.clone(),
        source_bill_type,
        source_bill_no,
        notes: payload.notes.clone(),
    })
}

/// 归一后的库存行参数与流水引用（两条直建端点共用，避免各自拼一遍导致列口径分叉）
///
/// 入参 `warehouse_id` 与 `product_id` 取自 DTO（两者都是 NOT NULL 外键列，存在性由 service 层
/// 统一校验），`trace` 取自 [`admit_stock_fabric_trace`]，`measures` 取自
/// [`admit_stock_fabric_measures`]。传给谁是 `InventoryStockService::create_stock_fabric_with_flow`。
/// 存什么存哪里是库存行的四维、等级、双计量与库位三列，以及流水行的方向类型、来源单据三列、
/// 备注与登记人。
pub fn stock_args_from_admitted(
    warehouse_id: i32,
    product_id: i32,
    trace: FabricTrace,
    measures: AdmittedStockMeasures,
    created_by: Option<i32>,
) -> (CreateStockFabricArgs, StockInboundFlowRef) {
    let AdmittedStockMeasures {
        grade,
        quantity_meters,
        quantity_kg,
        gram_weight,
        width,
        location_id,
        shelf_no,
        layer_no,
        source_bill_type,
        source_bill_no,
        notes,
    } = measures;

    let stock_args = CreateStockFabricArgs {
        warehouse_id,
        product_id,
        batch_no: trace.batch_no,
        color_no: trace.color_no,
        dye_lot_no: trace.dye_lot_no,
        grade,
        quantity_meters,
        quantity_kg,
        gram_weight,
        width,
        location_id,
        shelf_no,
        layer_no,
    };
    // 直建入口没有上游单据主键可引用（只登记单号），source_bill_id 如实留 None，不用 0 冒充外键
    let flow = StockInboundFlowRef {
        transaction_type: source_bill_type.clone(),
        source_bill_type: Some(source_bill_type),
        source_bill_no: Some(source_bill_no),
        source_bill_id: None,
        notes,
        created_by,
    };
    (stock_args, flow)
}

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
            location_id: stock.location_id,
            shelf_no: stock.shelf_no,
            layer_no: stock.layer_no,
            stock_status: stock.stock_status,
            quality_status: stock.quality_status,
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
    auth: AuthContext,
    Json(payload): Json<CreateStockFabricRequest>,
) -> Result<Json<ApiResponse<StockFabricResponse>>, AppError> {
    // 输入验证：DTO 校验拒绝只回显用户自己提交的字段规则，走 From<ValidationErrors>
    // 的可读外显链路（不得手工 AppError::validation——那会把原因压成"请求参数验证失败"，
    // 用户看不到"批次不得为空"，也就永远到不了下方 fabric_class 的判定文案）
    payload.validate().map_err(AppError::from)?;

    // 度量准入（数量下限、等级取值域、克重幅宽为正、公斤数唯一解析、来源单号必填）
    let measures = admit_stock_fabric_measures(&payload)?;

    // 白坯/染色追溯口径（色号/缸号/批次）准入：委托唯一权威判定，见 helper 文档
    let trace = admit_stock_fabric_trace(
        payload.color_no.clone(),
        payload.dye_lot_no.clone(),
        Some(payload.batch_no.clone()),
    )?;

    let (stock_args, flow) = stock_args_from_admitted(
        payload.warehouse_id,
        payload.product_id,
        trace,
        measures,
        // 登记人取会话身份，请求体不承载身份
        Some(auth.user_id),
    );

    let service = InventoryStockService::new(state.db.clone());
    let stock = service
        .create_stock_fabric_with_flow(stock_args, flow)
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
        location_id: stock.location_id,
        shelf_no: stock.shelf_no,
        layer_no: stock.layer_no,
        stock_status: stock.stock_status,
        quality_status: stock.quality_status,
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
