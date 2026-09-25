//! 坯布管理Handler（原料布匹管理）

use axum::{
    extract::{Path, Query, State},
    Json,
};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde::Deserialize;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::greige_fabric;
use crate::models::status::purchase_inventory::greige_fabric_status;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct GreigeFabricListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub fabric_no: Option<String>,
    pub fabric_name: Option<String>,
    pub fabric_type: Option<String>,
    pub supplier_id: Option<i32>,
    pub warehouse_id: Option<i32>,
    pub status: Option<String>,
    pub quality_grade: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CreateGreigeFabricRequest {
    pub fabric_no: Option<String>,
    pub fabric_name: Option<String>,
    pub fabric_type: Option<String>,
    pub color_code: Option<String>,
    pub width_cm: Option<f64>,
    pub weight_kg: Option<f64>,
    pub length_m: Option<f64>,
    pub supplier_id: Option<i32>,
    pub batch_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub location: Option<String>,
    pub status: Option<String>,
    pub quality_grade: Option<String>,
    pub purchase_date: Option<chrono::NaiveDate>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
    pub product_id: Option<i32>,
    pub composition: Option<String>,
    pub yarn_count: Option<String>,
    pub density: Option<String>,
    pub width: Option<f64>,
    pub gram_weight: Option<f64>,
    pub structure: Option<String>,
    pub production_date: Option<chrono::NaiveDate>,
    pub quantity_meters: Option<f64>,
    pub quantity_kg: Option<f64>,
    /// 缺陷 1.1：关联采购订单ID
    pub purchase_order_id: Option<i32>,
    /// 缺陷 1.1：关联采购入库单ID
    pub purchase_receipt_id: Option<i32>,
    /// 缺陷 1.2：安全库存（公斤）
    pub safety_stock: Option<f64>,
    /// 缺陷 1.2：订货点（公斤）
    pub reorder_point: Option<f64>,
    /// 缺陷 1.2：最大库存（公斤）
    pub max_stock_point: Option<f64>,
    /// 缺陷 1.2：补货量（公斤）
    pub reorder_quantity: Option<f64>,
    /// V15 P2 21.3：缸号（染色批次追溯）
    pub dye_lot_no: Option<String>,
    /// V15 P2 21.3：色号（颜色批次追溯）
    pub color_no: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct UpdateGreigeFabricRequest {
    pub fabric_name: Option<String>,
    pub fabric_type: Option<String>,
    pub color_code: Option<String>,
    pub width_cm: Option<f64>,
    pub weight_kg: Option<f64>,
    pub length_m: Option<f64>,
    pub supplier_id: Option<i32>,
    pub batch_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub location: Option<String>,
    pub status: Option<String>,
    pub quality_grade: Option<String>,
    pub remarks: Option<String>,
    /// 缺陷 1.2：安全库存配置
    pub safety_stock: Option<f64>,
    pub reorder_point: Option<f64>,
    pub max_stock_point: Option<f64>,
    pub reorder_quantity: Option<f64>,
    /// V15 P2 21.3：缸号（染色批次追溯）
    pub dye_lot_no: Option<String>,
    /// V15 P2 21.3：色号（颜色批次追溯）
    pub color_no: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct StockInRequest {
    pub warehouse_id: i32,
    pub location: Option<String>,
    pub weight_kg: f64,
    pub length_m: f64,
    pub quality_grade: Option<String>,
    pub remarks: Option<String>,
    /// 缺陷 1.1：入库时关联采购入库单ID
    pub purchase_receipt_id: Option<i32>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct StockOutRequest {
    pub weight_kg: Option<f64>,
    pub length_m: Option<f64>,
    pub remarks: Option<String>,
}

/// 坯布状态入参校验（create/update 提交的 `status`）。
///
/// `greige_fabrics.status` 取值域是中文主数据（见 `greige_fabric_status`），前端表单
/// 曾用通用主数据英文 token（`active`/`inactive`）直接提交，越界值原样写库后与出库/
/// 门控使用的中文值逐字符不符，导致「在库不允许删除」门控永不命中、库里中英混杂。
/// 现按取值域拒绝并回显允许值；`None` 由调用方保持不覆盖，不进入本校验。
fn validate_greige_status(status: &str) -> Result<(), AppError> {
    if greige_fabric_status::ALL.contains(&status) {
        return Ok(());
    }
    Err(AppError::validation(format!(
        "无效的坯布状态：{}（允许值：{}）",
        status,
        greige_fabric_status::ALL.join("/")
    )))
}

/// 错误类型从 StatusCode 改为 AppError，并使用 `?` 运算符简化错误传播；
/// `AppError: From<sea_orm::DbErr>` 已实现自动转换。
pub async fn list_greige_fabrics(
    State(state): State<AppState>,
    Query(query): Query<GreigeFabricListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<greige_fabric::Model>>>, AppError> {
    let page = query.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let mut q = greige_fabric::Entity::find().filter(greige_fabric::Column::IsDeleted.eq(false));

    if let Some(fabric_no) = &query.fabric_no {
        q = q.filter(greige_fabric::Column::FabricNo.contains(fabric_no));
    }
    if let Some(fabric_name) = &query.fabric_name {
        q = q.filter(greige_fabric::Column::FabricName.contains(fabric_name));
    }
    if let Some(fabric_type) = &query.fabric_type {
        q = q.filter(greige_fabric::Column::FabricType.eq(fabric_type));
    }
    if let Some(supplier_id) = query.supplier_id {
        q = q.filter(greige_fabric::Column::SupplierId.eq(supplier_id));
    }
    if let Some(warehouse_id) = query.warehouse_id {
        q = q.filter(greige_fabric::Column::WarehouseId.eq(warehouse_id));
    }
    if let Some(status) = &query.status {
        q = q.filter(greige_fabric::Column::Status.eq(status));
    }
    if let Some(grade) = &query.quality_grade {
        q = q.filter(greige_fabric::Column::QualityGrade.eq(grade));
    }

    q = q.order_by_desc(greige_fabric::Column::CreatedAt);

    let paginator = q.paginate(&*state.db, page_size);
    let total = paginator.num_items().await?;
    // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
    let fabrics = paginator
        .fetch_page(page.clamp(1, 1000).saturating_sub(1))
        .await?;
    Ok(Json(ApiResponse::success_paginated(
        fabrics, total, page, page_size,
    )))
}

pub async fn get_greige_fabric(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<greige_fabric::Model>>, AppError> {
    let fabric = greige_fabric::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("坯布不存在"))?;
    Ok(Json(ApiResponse::success(fabric)))
}

pub async fn create_greige_fabric(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateGreigeFabricRequest>,
) -> Result<Json<ApiResponse<greige_fabric::Model>>, AppError> {
    // fabric_type 为领域必填属性（DB NOT NULL），缺失时返回清晰校验错误而非裸 500
    let fabric_type = req
        .fabric_type
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| AppError::validation("坯布类型(fabric_type)不能为空"))?;

    // 自动生成编号
    let fabric_no = req.fabric_no.unwrap_or_else(|| {
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_4_digit();
        format!("GF-{}-{:04}", timestamp, random)
    });

    // 状态：未提交时默认「在库」，提交时必须是本列取值域内的中文 token
    let status = match req.status {
        Some(s) => {
            validate_greige_status(&s)?;
            s
        }
        None => greige_fabric_status::IN_STOCK.to_string(),
    };

    let fabric = greige_fabric::ActiveModel {
        // id 交由 SERIAL 序列生成；显式 Set(0) 会写入主键 0 并在第二次插入时主键冲突
        id: NotSet,
        fabric_no: Set(fabric_no),
        fabric_name: Set(req.fabric_name.unwrap_or_else(|| "未命名坯布".to_string())),
        product_id: Set(req.product_id),
        supplier_id: Set(req.supplier_id),
        composition: Set(req.composition),
        yarn_count: Set(req.yarn_count),
        density: Set(req.density),
        width: Set(req.width.and_then(Decimal::from_f64_retain)),
        gram_weight: Set(req.gram_weight.and_then(Decimal::from_f64_retain)),
        structure: Set(req.structure),
        production_date: Set(req.production_date),
        batch_no: Set(req.batch_no),
        quantity_meters: Set(req.quantity_meters.and_then(Decimal::from_f64_retain)),
        quantity_kg: Set(req.quantity_kg.and_then(Decimal::from_f64_retain)),
        warehouse_id: Set(req.warehouse_id),
        status: Set(Some(status)),
        is_deleted: Set(Some(false)),
        fabric_type: Set(fabric_type),
        color_code: Set(req.color_code),
        width_cm: Set(req.width_cm.and_then(Decimal::from_f64_retain)),
        weight_kg: Set(req.weight_kg.and_then(Decimal::from_f64_retain)),
        length_m: Set(req.length_m.and_then(Decimal::from_f64_retain)),
        location: Set(req.location),
        quality_grade: Set(req.quality_grade),
        purchase_date: Set(req.purchase_date),
        remarks: Set(req.remarks),
        created_by: Set(req.created_by),
        purchase_order_id: Set(req.purchase_order_id),
        purchase_receipt_id: Set(req.purchase_receipt_id),
        safety_stock: Set(req.safety_stock.and_then(Decimal::from_f64_retain)),
        reorder_point: Set(req.reorder_point.and_then(Decimal::from_f64_retain)),
        max_stock_point: Set(req.max_stock_point.and_then(Decimal::from_f64_retain)),
        reorder_quantity: Set(req.reorder_quantity.and_then(Decimal::from_f64_retain)),
        // V15 P2 21.3：缸号/色号追溯字段
        dye_lot_no: Set(req.dye_lot_no),
        color_no: Set(req.color_no),
        created_at: Set(crate::utils::date_utils::utc_now_fixed()),
        updated_at: Set(crate::utils::date_utils::utc_now_fixed()),
    };

    let created = fabric.insert(&*state.db).await?;
    Ok(Json(ApiResponse::success_with_message(
        created,
        "坯布创建成功",
    )))
}

pub async fn update_greige_fabric(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
    Json(req): Json<UpdateGreigeFabricRequest>,
) -> Result<Json<ApiResponse<greige_fabric::Model>>, AppError> {
    let mut fabric: greige_fabric::ActiveModel = greige_fabric::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("坯布不存在"))?
        .into();

    if let Some(fabric_name) = req.fabric_name {
        fabric.fabric_name = Set(fabric_name);
    }
    if let Some(fabric_type) = req.fabric_type {
        let fabric_type = fabric_type.trim().to_string();
        if fabric_type.is_empty() {
            return Err(AppError::validation("坯布类型(fabric_type)不能为空"));
        }
        fabric.fabric_type = Set(fabric_type);
    }
    if let Some(color_code) = req.color_code {
        fabric.color_code = Set(Some(color_code));
    }
    if let Some(width_cm) = req.width_cm {
        fabric.width_cm = Set(Decimal::from_f64_retain(width_cm));
    }
    if let Some(weight_kg) = req.weight_kg {
        fabric.weight_kg = Set(Decimal::from_f64_retain(weight_kg));
    }
    if let Some(length_m) = req.length_m {
        fabric.length_m = Set(Decimal::from_f64_retain(length_m));
    }
    if let Some(supplier_id) = req.supplier_id {
        fabric.supplier_id = Set(Some(supplier_id));
    }
    if let Some(batch_no) = req.batch_no {
        fabric.batch_no = Set(Some(batch_no));
    }
    if let Some(warehouse_id) = req.warehouse_id {
        fabric.warehouse_id = Set(Some(warehouse_id));
    }
    if let Some(location) = req.location {
        fabric.location = Set(Some(location));
    }
    if let Some(status) = req.status {
        validate_greige_status(&status)?;
        fabric.status = Set(Some(status));
    }
    if let Some(quality_grade) = req.quality_grade {
        fabric.quality_grade = Set(Some(quality_grade));
    }
    if let Some(remarks) = req.remarks {
        fabric.remarks = Set(Some(remarks));
    }
    // 缺陷 1.2：安全库存配置更新
    if let Some(safety) = req.safety_stock {
        fabric.safety_stock = Set(Decimal::from_f64_retain(safety));
    }
    if let Some(reorder) = req.reorder_point {
        fabric.reorder_point = Set(Decimal::from_f64_retain(reorder));
    }
    if let Some(max_stock) = req.max_stock_point {
        fabric.max_stock_point = Set(Decimal::from_f64_retain(max_stock));
    }
    if let Some(reorder_qty) = req.reorder_quantity {
        fabric.reorder_quantity = Set(Decimal::from_f64_retain(reorder_qty));
    }
    // V15 P2 21.3：缸号/色号追溯字段更新
    if let Some(dye_lot_no) = req.dye_lot_no {
        fabric.dye_lot_no = Set(Some(dye_lot_no));
    }
    if let Some(color_no) = req.color_no {
        fabric.color_no = Set(Some(color_no));
    }

    fabric.updated_at = Set(crate::utils::date_utils::utc_now_fixed());

    let updated = fabric.update(&*state.db).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "坯布更新成功",
    )))
}

pub async fn delete_greige_fabric(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let fabric = greige_fabric::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("坯布不存在"))?;

    if fabric.status.as_deref() == Some(greige_fabric_status::IN_STOCK) {
        return Err(AppError::business("在库坯布不允许删除，请先完成出库"));
    }

    // 软删除
    let mut active: greige_fabric::ActiveModel = fabric.into();
    active.is_deleted = Set(Some(true));
    active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());

    active.update(&*state.db).await?;
    Ok(Json(ApiResponse::success_with_message((), "坯布删除成功")))
}

pub async fn stock_in(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
    Json(req): Json<StockInRequest>,
) -> Result<Json<ApiResponse<greige_fabric::Model>>, AppError> {
    let mut fabric: greige_fabric::ActiveModel = greige_fabric::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("坯布不存在"))?
        .into();

    // 累加库存而不是覆盖
    let current_weight = fabric
        .weight_kg
        .as_ref()
        .and_then(|w| w.to_string().parse::<f64>().ok())
        .unwrap_or(0.0);
    let current_length = fabric
        .length_m
        .as_ref()
        .and_then(|l| l.to_string().parse::<f64>().ok())
        .unwrap_or(0.0);
    let current_qty_kg = fabric
        .quantity_kg
        .as_ref()
        .and_then(|w| w.to_string().parse::<f64>().ok())
        .unwrap_or(0.0);
    let current_qty_meters = fabric
        .quantity_meters
        .as_ref()
        .and_then(|l| l.to_string().parse::<f64>().ok())
        .unwrap_or(0.0);

    let new_weight = current_weight + req.weight_kg;
    let new_length = current_length + req.length_m;

    fabric.warehouse_id = Set(Some(req.warehouse_id));
    fabric.location = Set(req.location);
    fabric.weight_kg = Set(Decimal::from_f64_retain(new_weight));
    fabric.length_m = Set(Decimal::from_f64_retain(new_length));
    fabric.quantity_kg = Set(Decimal::from_f64_retain(current_qty_kg + req.weight_kg));
    fabric.quantity_meters = Set(Decimal::from_f64_retain(current_qty_meters + req.length_m));
    fabric.status = Set(Some(greige_fabric_status::IN_STOCK.to_string()));
    if let Some(grade) = req.quality_grade {
        fabric.quality_grade = Set(Some(grade));
    }
    if let Some(remarks) = req.remarks {
        fabric.remarks = Set(Some(remarks));
    }
    // 缺陷 1.1：入库时关联采购入库单
    if let Some(receipt_id) = req.purchase_receipt_id {
        fabric.purchase_receipt_id = Set(Some(receipt_id));
    }
    fabric.updated_at = Set(crate::utils::date_utils::utc_now_fixed());

    let updated = fabric.update(&*state.db).await?;

    // 缺陷 1.2：入库后检查安全库存预警
    let current_weight_decimal = updated.weight_kg.unwrap_or(Decimal::ZERO);
    let reorder_point = updated.reorder_point.unwrap_or(Decimal::ZERO);
    let safety_stock = updated.safety_stock.unwrap_or(Decimal::ZERO);
    let max_stock = updated.max_stock_point.unwrap_or(Decimal::ZERO);
    if reorder_point > Decimal::ZERO && current_weight_decimal <= reorder_point {
        tracing::warn!(
            fabric_id = updated.id,
            current_weight = %current_weight_decimal,
            reorder_point = %reorder_point,
            safety_stock = %safety_stock,
            "胚布库存低于订货点，触发补货建议"
        );
    }
    if max_stock > Decimal::ZERO && current_weight_decimal > max_stock {
        tracing::warn!(
            fabric_id = updated.id,
            current_weight = %current_weight_decimal,
            max_stock = %max_stock,
            "胚布库存超过最大库存点"
        );
    }

    Ok(Json(ApiResponse::success_with_message(
        updated,
        "坯布入库成功",
    )))
}

pub async fn stock_out(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
    Json(req): Json<StockOutRequest>,
) -> Result<Json<ApiResponse<greige_fabric::Model>>, AppError> {
    let fabric = greige_fabric::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("坯布不存在"))?;

    let mut update_fabric: greige_fabric::ActiveModel = fabric.clone().into();

    if let Some(out_weight) = req.weight_kg {
        let current_weight = fabric.weight_kg.unwrap_or(Decimal::ZERO);
        let new_weight =
            current_weight - Decimal::from_f64_retain(out_weight).unwrap_or(Decimal::ZERO);
        if new_weight < Decimal::ZERO {
            return Err(AppError::business("出库重量不能大于现有重量"));
        }
        update_fabric.weight_kg = Set(Some(new_weight));
        // 同步更新 quantity_kg
        let current_qty_kg = fabric.quantity_kg.unwrap_or(Decimal::ZERO);
        update_fabric.quantity_kg = Set(Some(
            current_qty_kg - Decimal::from_f64_retain(out_weight).unwrap_or(Decimal::ZERO),
        ));
    }

    if let Some(out_length) = req.length_m {
        let current_length = fabric.length_m.unwrap_or(Decimal::ZERO);
        let new_length =
            current_length - Decimal::from_f64_retain(out_length).unwrap_or(Decimal::ZERO);
        if new_length < Decimal::ZERO {
            return Err(AppError::business("出库长度不能大于现有长度"));
        }
        update_fabric.length_m = Set(Some(new_length));
        // 同步更新 quantity_meters
        let current_qty_meters = fabric.quantity_meters.unwrap_or(Decimal::ZERO);
        update_fabric.quantity_meters = Set(Some(
            current_qty_meters - Decimal::from_f64_retain(out_length).unwrap_or(Decimal::ZERO),
        ));
    }

    // 根据剩余库存决定状态
    let final_weight = update_fabric
        .weight_kg
        .as_ref()
        .and_then(|w| w.to_string().parse::<f64>().ok())
        .unwrap_or(0.0);
    let final_length = update_fabric
        .length_m
        .as_ref()
        .and_then(|l| l.to_string().parse::<f64>().ok())
        .unwrap_or(0.0);

    let new_status = if final_weight <= 0.0 && final_length <= 0.0 {
        greige_fabric_status::STOCKED_OUT.to_string()
    } else {
        greige_fabric_status::IN_STOCK.to_string()
    };

    update_fabric.status = Set(Some(new_status));
    update_fabric.updated_at = Set(crate::utils::date_utils::utc_now_fixed());

    // 同步更新备注（若请求提供则覆盖，否则保留原值）
    if let Some(remarks) = req.remarks {
        update_fabric.remarks = Set(Some(remarks));
    }

    let updated = update_fabric.update(&*state.db).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "坯布出库成功",
    )))
}

pub async fn get_greige_by_supplier(
    State(state): State<AppState>,
    Path(supplier_id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<greige_fabric::Model>>>, AppError> {
    let fabrics = greige_fabric::Entity::find()
        .filter(greige_fabric::Column::SupplierId.eq(supplier_id))
        .filter(greige_fabric::Column::IsDeleted.eq(false))
        .order_by_desc(greige_fabric::Column::CreatedAt)
        .all(&*state.db)
        .await?;
    Ok(Json(ApiResponse::success(fabrics)))
}
