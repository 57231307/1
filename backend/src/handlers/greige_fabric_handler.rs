//! 坯布管理Handler（原料布匹管理）

use axum::{
    Json,
    extract::{Path, Query, State},
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
use crate::models::product;
use crate::models::purchase_order;
use crate::models::purchase_receipt;
use crate::models::status::purchase_inventory::greige_fabric_status;
use crate::models::supplier;
use crate::models::warehouse;
use crate::utils::data_scope;
use crate::utils::dual_unit_converter::DualUnitConverter;
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
    /// 以下可空列按三态语义接收：键缺席=保持原值、显式 `null`=清空为 NULL、有值=覆盖。
    /// serde_json 把请求体里的显式 `null` 折叠成外层 `None`，与「未提供该键」不可区分，
    /// 故逐个挂 `deserialize_with = "double_option"`；范式同 `asset_category_handler`。
    /// `status` 有意不开清空（见本结构体下方注释），保持裸 `Option<String>`。
    #[serde(default, deserialize_with = "double_option")]
    pub color_code: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub width_cm: Option<Option<f64>>,
    #[serde(default, deserialize_with = "double_option")]
    pub weight_kg: Option<Option<f64>>,
    #[serde(default, deserialize_with = "double_option")]
    pub length_m: Option<Option<f64>>,
    #[serde(default, deserialize_with = "double_option")]
    pub supplier_id: Option<Option<i32>>,
    #[serde(default, deserialize_with = "double_option")]
    pub batch_no: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub warehouse_id: Option<Option<i32>>,
    #[serde(default, deserialize_with = "double_option")]
    pub location: Option<Option<String>>,
    /// 状态只覆盖、不开清空：删除门控按 `status == 在库` 判在库不可删，出库回写也按该
    /// token 判定。把本列置 NULL 等于让物理在库的卷失去门控（历史 NULL 行可直删属另一
    /// 条待裁定的存量面，不在本端点里新增造 NULL 的入口）。
    pub status: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub quality_grade: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub remarks: Option<Option<String>>,
    /// 缺陷 1.2：安全库存配置（三态同上）
    #[serde(default, deserialize_with = "double_option")]
    pub safety_stock: Option<Option<f64>>,
    #[serde(default, deserialize_with = "double_option")]
    pub reorder_point: Option<Option<f64>>,
    #[serde(default, deserialize_with = "double_option")]
    pub max_stock_point: Option<Option<f64>>,
    #[serde(default, deserialize_with = "double_option")]
    pub reorder_quantity: Option<Option<f64>>,
    /// V15 P2 21.3：缸号（染色批次追溯）
    #[serde(default, deserialize_with = "double_option")]
    pub dye_lot_no: Option<Option<String>>,
    /// V15 P2 21.3：色号（颜色批次追溯）
    #[serde(default, deserialize_with = "double_option")]
    pub color_no: Option<Option<String>>,
}

/// 把 JSON 值解成「外层是否存在该键」与「内层是否为 null」两层可辨的形态。
///
/// 键缺席（配合 `serde(default)`）得 `None`，语义是保持原值；显式 `null` 得 `Some(None)`，
/// 语义是清空该列；有值得 `Some(Some(v))`，语义是覆盖。范式同
/// `asset_category_handler::double_option` 与 `batch_service::double_option`。
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

/// 可空文本入参归一：去首尾空白，空串一律折成 `None`（即落 NULL）。
///
/// 这几列库里不存在「空串」与 NULL 的语义差别（白坯即 `color_no` 为空），落 `""` 会让同
/// 一含义两种存法并存，并与出库侧按「是否为空」判白坯的口径漂移（见
/// `services/inv/fabric_class.rs::validate_fabric_trace`）。
fn blank_to_none(raw: Option<String>) -> Option<String> {
    raw.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// 可空文本列的三态落库：外层缺席由调用方跳过，`Some(None)` 清空为 NULL，
/// `Some(Some(v))` 归一后覆盖（空串折成清空）。
fn normalize_text(raw: Option<Option<String>>) -> Option<Option<String>> {
    raw.map(blank_to_none)
}

/// 库存与规格类数值入参转 DECIMAL，非有限值与负值一律显式拒绝。
///
/// `greige_fabric` 的 width_cm/weight_kg/length_m 与四个库存策略列都是「量」，库里无 CHECK
/// 兜负值：`stock_in` 按累加、`stock_out` 按减法维护重量与长度，负值一旦写进 weight_kg
/// 或 length_m，出库门 `new_weight < 0` 的判定与「在库不允许删除」门控全部失真。
/// 界面本次才把这些列第一次暴露成可录入项，故录入面必须自带拒绝，不用
/// `unwrap_or(Decimal::ZERO)` 之类的兜底把坏值静默写成 0。
fn stock_decimal(value: f64, field: &str) -> Result<Decimal, AppError> {
    if !value.is_finite() {
        return Err(AppError::validation_displayable(format!(
            "{field}不是有效数值，请重新填写"
        )));
    }
    if value < 0.0 {
        return Err(AppError::validation_displayable(format!(
            "{field}不能为负数"
        )));
    }
    Decimal::from_f64_retain(value).ok_or_else(|| {
        AppError::validation_displayable(format!("{field}超出可保存的数值范围"))
    })
}

/// 可空数值列的落库：`None` 保持/不写，`Some(v)` 经 `stock_decimal` 校验后覆盖。
/// 建单入参是裸 `Option<f64>`（无「保持原值」语义），直接复用本函数把坏值显式拒绝，
/// 不再用 `and_then(Decimal::from_f64_retain)` 把 NaN 静默折成 NULL。
fn stock_optional(value: Option<f64>, field: &str) -> Result<Option<Decimal>, AppError> {
    value.map(|v| stock_decimal(v, field)).transpose()
}

/// 出入库数量入参：必须是非零有限值，转 DECIMAL 后原样上抛错误。
///
/// 入库传 0 或负数会让「累计入库量」与「当前库存」反向漂移（`stock_in` 是累加），出库传
/// 负数等于凭空加库存；本函数把两类都拒成显式校验错误，不回退成默认数量。
fn positive_decimal(value: f64, field: &str) -> Result<Decimal, AppError> {
    let checked = stock_decimal(value, field)?;
    if checked.is_zero() {
        return Err(AppError::validation_displayable(format!(
            "{field}必须大于 0"
        )));
    }
    Ok(checked)
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
    Err(AppError::validation_displayable(format!(
        "无效的坯布状态：{}（允许值：{}）",
        status,
        greige_fabric_status::ALL.join("/")
    )))
}

/// 坯布列表 `status` 筛选入参校验（取值域与 create/update 同源 `greige_fabric_status::ALL`）。
///
/// 列表原先把该参数原样下推成 SQL 等值条件，越界/英文值恒零命中并静默返回空列表，前端假
/// 筛选永不显红。现按取值域拒绝并回显允许值；`None` 或去空白后的空串视为不加该筛选，交由
/// 前端 `serializeParams` 与后端 `normalize_empty_query_params` 双侧剔除，与 inventory 的
/// `validate_stock_status_param` 保持同一处理风格。
fn validate_greige_status_param(raw: Option<&str>) -> Result<(), AppError> {
    let Some(value) = raw.map(|s| s.trim()).filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    validate_greige_status(value)
}

/// 米数与公斤数双单位一致性校验，换算率与容差的唯一真源是
/// `utils/dual_unit_converter::DualUnitConverter`，此处不得再写第二套换算式。
///
/// 坯布同时存 length_m/weight_kg（当前库存）与 quantity_meters/quantity_kg（累计入库），
/// 入库按累加、出库按扣减，两栏若彼此矛盾（米数与公斤数对不上同一线密度），成本分摊分母
/// 与出库门控判定都会失真。当坯布已配克重(g/m²)与幅宽(cm)且均为正时，米↔公斤存在可换算
/// 口径，越界即显式拒绝，不静默写库；克重或幅宽未配（非正或缺席）时米与公斤是两个无从互推
/// 的独立量，跳过校验，不臆造线密度把正常录入判成非法。
fn ensure_dual_unit_consistent(
    length: Option<Decimal>,
    weight: Option<Decimal>,
    gram_weight: Option<Decimal>,
    width_cm: Option<Decimal>,
    label: &str,
) -> Result<(), AppError> {
    // 米数与公斤数都填了才构成一组可比的双单位；缺一项无从交叉核对
    let (Some(length), Some(weight)) = (length, weight) else {
        return Ok(());
    };
    // 克重与幅宽都配了正线密度才存在可换算口径，否则跳过（见函数头注释）
    let (Some(gram_weight), Some(width_cm)) = (gram_weight, width_cm) else {
        return Ok(());
    };
    if gram_weight <= Decimal::ZERO || width_cm <= Decimal::ZERO {
        return Ok(());
    }
    let consistent =
        DualUnitConverter::validate_dual_unit(length, weight, gram_weight, width_cm, None).map_err(
            |_| {
                AppError::validation_displayable(format!(
                    "{label}的米数与公斤数无法核对，请检查克重与门幅后重新填写"
                ))
            },
        )?;
    if !consistent {
        return Err(AppError::validation_displayable(format!(
            "{label}的米数与公斤数不一致，请按克重与门幅核对后重新填写"
        )));
    }
    Ok(())
}

/// 错误类型从 StatusCode 改为 AppError，并使用 `?` 运算符简化错误传播；
/// `AppError: From<sea_orm::DbErr>` 已实现自动转换。
pub async fn list_greige_fabrics(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<GreigeFabricListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<greige_fabric::Model>>>, AppError> {
    validate_greige_status_param(query.status.as_deref())?;

    let page = query.page.unwrap_or(1).clamp(1, 1000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let mut q = greige_fabric::Entity::find().filter(greige_fabric::Column::IsDeleted.eq(false));

    // 行级数据权限下推：greige_fabric 无 department_id 列，Dept 分支按
    // 「归属人 ∈ 可见部门成员集合」过滤（build_data_scope_condition），
    // owner_column 与 dept_column 均为 CreatedBy。
    let ctx = auth.to_data_scope_context();
    q = data_scope::apply_data_scope(
        q,
        &ctx,
        greige_fabric::Column::CreatedBy,
        greige_fabric::Column::CreatedBy,
    );

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
    let fabrics = paginator.fetch_page(page.saturating_sub(1)).await?;
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
    auth: AuthContext,
    Json(req): Json<CreateGreigeFabricRequest>,
) -> Result<Json<ApiResponse<greige_fabric::Model>>, AppError> {
    // fabric_type 为领域必填属性（DB NOT NULL），缺失时返回清晰校验错误而非裸 500
    let fabric_type = req
        .fabric_type
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| AppError::validation_displayable("坯布类型(fabric_type)不能为空"))?;

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

    // 外键预校验：supplier_id/warehouse_id 是裸 FK（REFERENCES suppliers/warehouses，无 ON DELETE 动作）。
    // 若不预检，脏 id 会在 insert 时抛 FK 约束被映射成 500 DATABASE_ERROR，调用方无法区分数据错误与系统故障。
    // 与产品建档 product_ops::crud 同口径，预检为 400 business。
    if let Some(sid) = req.supplier_id {
        supplier::Entity::find_by_id(sid)
            .one(&*state.db)
            .await?
            .ok_or_else(|| AppError::business("供应商不存在，请重新选择"))?;
    }
    if let Some(wid) = req.warehouse_id {
        warehouse::Entity::find_by_id(wid)
            .one(&*state.db)
            .await?
            .ok_or_else(|| AppError::business("仓库不存在，请重新选择"))?;
    }
    // product_id/purchase_order_id/purchase_receipt_id 三列库里没有外键约束（只有 supplier/warehouse
    // 两条，见 system 域建坯布表迁移的外键段），脏 id 不会撞约束、会静默存成
    // 悬空引用；实体侧却声明了 belongs_to（models/greige_fabric.rs 的 Relation 声明）。建单面按与
    // supplier/warehouse 同一口径预检为 400 business，缺外键约束本身另列数据层待办。
    if let Some(pid) = req.product_id {
        product::Entity::find_by_id(pid)
            .one(&*state.db)
            .await?
            .ok_or_else(|| AppError::business("产品不存在，请重新选择"))?;
    }
    if let Some(oid) = req.purchase_order_id {
        purchase_order::Entity::find_by_id(oid)
            .one(&*state.db)
            .await?
            .ok_or_else(|| AppError::business("采购订单不存在，请重新选择"))?;
    }
    if let Some(rid) = req.purchase_receipt_id {
        purchase_receipt::Entity::find_by_id(rid)
            .one(&*state.db)
            .await?
            .ok_or_else(|| AppError::business("采购入库单不存在，请重新选择"))?;
    }
    // 数值入参一律走 stock_optional：非有限值与负值显式拒绝，不再用
    // `and_then(Decimal::from_f64_retain)` 把坏值静默折成 NULL。
    let width = stock_optional(req.width, "门幅(m)")?;
    let gram_weight = stock_optional(req.gram_weight, "克重(g/m²)")?;
    let quantity_meters = stock_optional(req.quantity_meters, "累计米数(m)")?;
    let quantity_kg = stock_optional(req.quantity_kg, "累计重量(kg)")?;
    let width_cm = stock_optional(req.width_cm, "幅宽(cm)")?;
    let weight_kg = stock_optional(req.weight_kg, "重量(kg)")?;
    let length_m = stock_optional(req.length_m, "长度(m)")?;
    let safety_stock = stock_optional(req.safety_stock, "安全库存(kg)")?;
    let reorder_point = stock_optional(req.reorder_point, "订货点(kg)")?;
    let max_stock_point = stock_optional(req.max_stock_point, "最大库存(kg)")?;
    let reorder_quantity = stock_optional(req.reorder_quantity, "补货量(kg)")?;

    // 双单位一致性：累计入库量与当前存量两组，按已配克重与幅宽用换算率单源自检
    ensure_dual_unit_consistent(
        quantity_meters,
        quantity_kg,
        gram_weight,
        width_cm,
        "累计入库量",
    )?;
    ensure_dual_unit_consistent(length_m, weight_kg, gram_weight, width_cm, "当前库存")?;

    let fabric = greige_fabric::ActiveModel {
        // id 交由 SERIAL 序列生成；显式 Set(0) 会写入主键 0 并在第二次插入时主键冲突
        id: NotSet,
        fabric_no: Set(fabric_no),
        fabric_name: Set(req.fabric_name.unwrap_or_else(|| "未命名坯布".to_string())),
        product_id: Set(req.product_id),
        supplier_id: Set(req.supplier_id),
        composition: Set(blank_to_none(req.composition)),
        yarn_count: Set(blank_to_none(req.yarn_count)),
        density: Set(blank_to_none(req.density)),
        width: Set(width),
        gram_weight: Set(gram_weight),
        structure: Set(blank_to_none(req.structure)),
        production_date: Set(req.production_date),
        batch_no: Set(blank_to_none(req.batch_no)),
        quantity_meters: Set(quantity_meters),
        quantity_kg: Set(quantity_kg),
        warehouse_id: Set(req.warehouse_id),
        status: Set(Some(status)),
        is_deleted: Set(Some(false)),
        fabric_type: Set(fabric_type),
        color_code: Set(blank_to_none(req.color_code)),
        width_cm: Set(width_cm),
        weight_kg: Set(weight_kg),
        length_m: Set(length_m),
        location: Set(blank_to_none(req.location)),
        quality_grade: Set(blank_to_none(req.quality_grade)),
        purchase_date: Set(req.purchase_date),
        remarks: Set(blank_to_none(req.remarks)),
        // 建单人取服务端会话（AuthContext.user_id），请求体不承载身份
        created_by: Set(Some(auth.user_id)),
        purchase_order_id: Set(req.purchase_order_id),
        purchase_receipt_id: Set(req.purchase_receipt_id),
        safety_stock: Set(safety_stock),
        reorder_point: Set(reorder_point),
        max_stock_point: Set(max_stock_point),
        reorder_quantity: Set(reorder_quantity),
        // V15 P2 21.3：缸号/色号追溯字段（空串折成 NULL，NULL 色号即白坯）
        dye_lot_no: Set(blank_to_none(req.dye_lot_no)),
        color_no: Set(blank_to_none(req.color_no)),
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
    auth: AuthContext,
    Json(req): Json<UpdateGreigeFabricRequest>,
) -> Result<Json<ApiResponse<greige_fabric::Model>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let fabric_entity = greige_fabric::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("坯布不存在"))?;

    if !data_scope::check_resource_owner_by_member_scope(&ctx, fabric_entity.created_by) {
        return Err(AppError::permission_denied(
            "无权操作该坯布（数据范围限制）",
        ));
    }

    let mut fabric: greige_fabric::ActiveModel = fabric_entity.into();

    if let Some(fabric_name) = req.fabric_name {
        fabric.fabric_name = Set(fabric_name);
    }
    if let Some(fabric_type) = req.fabric_type {
        let fabric_type = fabric_type.trim().to_string();
        if fabric_type.is_empty() {
            return Err(AppError::validation_displayable(
                "坯布类型(fabric_type)不能为空",
            ));
        }
        fabric.fabric_type = Set(fabric_type);
    }
    if let Some(color_code) = normalize_text(req.color_code) {
        fabric.color_code = Set(color_code);
    }
    if let Some(width_cm) = req.width_cm {
        fabric.width_cm = Set(stock_optional(width_cm, "幅宽(cm)")?);
    }
    if let Some(weight_kg) = req.weight_kg {
        fabric.weight_kg = Set(stock_optional(weight_kg, "重量(kg)")?);
    }
    if let Some(length_m) = req.length_m {
        fabric.length_m = Set(stock_optional(length_m, "长度(m)")?);
    }
    if let Some(supplier_id) = req.supplier_id {
        // 与 create 同口径的外键预检：脏 id 裸抛 FK 违例会被映射成 500 DATABASE_ERROR，
        // 调用方分不清数据错误与系统故障；清空（None）不查库。
        if let Some(sid) = supplier_id {
            supplier::Entity::find_by_id(sid)
                .one(&*state.db)
                .await?
                .ok_or_else(|| AppError::business("供应商不存在，请重新选择"))?;
        }
        fabric.supplier_id = Set(supplier_id);
    }
    if let Some(batch_no) = normalize_text(req.batch_no) {
        fabric.batch_no = Set(batch_no);
    }
    if let Some(warehouse_id) = req.warehouse_id {
        if let Some(wid) = warehouse_id {
            warehouse::Entity::find_by_id(wid)
                .one(&*state.db)
                .await?
                .ok_or_else(|| AppError::business("仓库不存在，请重新选择"))?;
        }
        fabric.warehouse_id = Set(warehouse_id);
    }
    if let Some(location) = normalize_text(req.location) {
        fabric.location = Set(location);
    }
    if let Some(status) = req.status {
        validate_greige_status(&status)?;
        fabric.status = Set(Some(status));
    }
    if let Some(quality_grade) = normalize_text(req.quality_grade) {
        fabric.quality_grade = Set(quality_grade);
    }
    if let Some(remarks) = normalize_text(req.remarks) {
        fabric.remarks = Set(remarks);
    }
    // 缺陷 1.2：安全库存配置更新（三态；清空即未配置该阈值，读取侧按未配置不预警）
    if let Some(safety) = req.safety_stock {
        fabric.safety_stock = Set(stock_optional(safety, "安全库存(kg)")?);
    }
    if let Some(reorder) = req.reorder_point {
        fabric.reorder_point = Set(stock_optional(reorder, "订货点(kg)")?);
    }
    if let Some(max_stock) = req.max_stock_point {
        fabric.max_stock_point = Set(stock_optional(max_stock, "最大库存(kg)")?);
    }
    if let Some(reorder_qty) = req.reorder_quantity {
        fabric.reorder_quantity = Set(stock_optional(reorder_qty, "补货量(kg)")?);
    }
    // V15 P2 21.3：缸号/色号追溯字段更新（空串归一为 NULL=白坯，见 normalize_text）
    if let Some(dye_lot_no) = normalize_text(req.dye_lot_no) {
        fabric.dye_lot_no = Set(dye_lot_no);
    }
    if let Some(color_no) = normalize_text(req.color_no) {
        fabric.color_no = Set(color_no);
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
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let fabric = greige_fabric::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("坯布不存在"))?;

    if !data_scope::check_resource_owner_by_member_scope(&ctx, fabric.created_by) {
        return Err(AppError::permission_denied(
            "无权操作该坯布（数据范围限制）",
        ));
    }

    if fabric.status.as_deref() == Some(greige_fabric_status::IN_STOCK) {
        return Err(AppError::business_displayable(
            "在库坯布不允许删除，请先完成出库",
        ));
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
    auth: AuthContext,
    Json(req): Json<StockInRequest>,
) -> Result<Json<ApiResponse<greige_fabric::Model>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let fabric_entity = greige_fabric::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("坯布不存在"))?;

    if !data_scope::check_resource_owner_by_member_scope(&ctx, fabric_entity.created_by) {
        return Err(AppError::permission_denied(
            "无权操作该坯布（数据范围限制）",
        ));
    }

    // 归属门之后才做入参校验与外键预检：越权调用必须先拿 403 且零副作用。
    let in_weight = positive_decimal(req.weight_kg, "入库重量(kg)")?;
    let in_length = positive_decimal(req.length_m, "入库长度(m)")?;
    warehouse::Entity::find_by_id(req.warehouse_id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::business("仓库不存在，请重新选择"))?;
    if let Some(receipt_id) = req.purchase_receipt_id {
        // purchase_receipt_id 无库内外键（见 create 同位置注释），预检防悬空引用
        purchase_receipt::Entity::find_by_id(receipt_id)
            .one(&*state.db)
            .await?
            .ok_or_else(|| AppError::business("采购入库单不存在，请重新选择"))?;
    }

    // 本次入库的米数与公斤数按坯布档案上已配的克重与幅宽做双单位一致性自检（单源见
    // ensure_dual_unit_consistent）；档案未配线密度时跳过，不臆造换算把正常入库判成非法。
    ensure_dual_unit_consistent(
        Some(in_length),
        Some(in_weight),
        fabric_entity.gram_weight,
        fabric_entity.width_cm,
        "本次入库量",
    )?;

    // 累加库存而不是覆盖：全程 DECIMAL 相加。原实现把 DECIMAL 经 `to_string().parse::<f64>()`
    // 折成浮点再转回 DECIMAL，累计量大时会漂分位，且 parse 失败静默当 0。
    let current_weight = fabric_entity.weight_kg.unwrap_or(Decimal::ZERO);
    let current_length = fabric_entity.length_m.unwrap_or(Decimal::ZERO);
    let current_qty_kg = fabric_entity.quantity_kg.unwrap_or(Decimal::ZERO);
    let current_qty_meters = fabric_entity.quantity_meters.unwrap_or(Decimal::ZERO);

    let mut fabric: greige_fabric::ActiveModel = fabric_entity.into();
    fabric.warehouse_id = Set(Some(req.warehouse_id));
    fabric.location = Set(blank_to_none(req.location));
    fabric.weight_kg = Set(Some(current_weight + in_weight));
    fabric.length_m = Set(Some(current_length + in_length));
    fabric.quantity_kg = Set(Some(current_qty_kg + in_weight));
    fabric.quantity_meters = Set(Some(current_qty_meters + in_length));
    fabric.status = Set(Some(greige_fabric_status::IN_STOCK.to_string()));
    // 入库的等级与备注是单层 Option（StockInRequest 非三态键）：给值且非空白才覆盖，
    // 缺席或纯空白保持原值。blank_to_none 与 stock_out 的备注归一同源，不走 normalize_text
    // （那是给 Update 请求双层 Option 三态用的，类型不匹配）
    if let Some(grade) = blank_to_none(req.quality_grade) {
        fabric.quality_grade = Set(Some(grade));
    }
    if let Some(remarks) = blank_to_none(req.remarks) {
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
    auth: AuthContext,
    Json(req): Json<StockOutRequest>,
) -> Result<Json<ApiResponse<greige_fabric::Model>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let fabric = greige_fabric::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("坯布不存在"))?;

    if !data_scope::check_resource_owner_by_member_scope(&ctx, fabric.created_by) {
        return Err(AppError::permission_denied(
            "无权操作该坯布（数据范围限制）",
        ));
    }

    // 出库数量入参：至少一项、每项必须是正有限值。原实现两项全缺也照样走完并把状态
    // 回写一遍（空提交会把 0 库存行的状态重算落库），现显式拒绝，不让空调用冒充出库。
    if req.weight_kg.is_none() && req.length_m.is_none() {
        return Err(AppError::validation_displayable(
            "出库数量至少填写一项：重量(kg) 或 长度(m)",
        ));
    }
    let out_weight = req
        .weight_kg
        .map(|v| positive_decimal(v, "出库重量(kg)"))
        .transpose()?;
    let out_length = req
        .length_m
        .map(|v| positive_decimal(v, "出库长度(m)"))
        .transpose()?;

    let current_weight = fabric.weight_kg.unwrap_or(Decimal::ZERO);
    let current_length = fabric.length_m.unwrap_or(Decimal::ZERO);
    let current_qty_kg = fabric.quantity_kg.unwrap_or(Decimal::ZERO);
    let current_qty_meters = fabric.quantity_meters.unwrap_or(Decimal::ZERO);

    let mut update_fabric: greige_fabric::ActiveModel = fabric.clone().into();
    // 扣减后的余量就地跟踪，供状态判定使用（不回头读 ActiveModel 的值形态）
    let mut final_weight = current_weight;
    let mut final_length = current_length;

    if let Some(out_weight) = out_weight {
        let new_weight = current_weight - out_weight;
        if new_weight < Decimal::ZERO {
            return Err(AppError::business_displayable(format!(
                "出库重量不能大于现有重量（现有 {current_weight} kg，本次 {out_weight} kg）"
            )));
        }
        // 累计出库量同样不得被减成负数：quantity_kg 是入库累计口径，与当前库存同源扣减，
        // 只挡当前库存一处的话脏数据会把累计量永久带负，成本分摊分母随之失真。
        let new_qty_kg = current_qty_kg - out_weight;
        if new_qty_kg < Decimal::ZERO {
            return Err(AppError::business_displayable(format!(
                "出库重量不能大于累计入库重量（累计 {current_qty_kg} kg，本次 {out_weight} kg）"
            )));
        }
        update_fabric.weight_kg = Set(Some(new_weight));
        update_fabric.quantity_kg = Set(Some(new_qty_kg));
        final_weight = new_weight;
    }

    if let Some(out_length) = out_length {
        let new_length = current_length - out_length;
        if new_length < Decimal::ZERO {
            return Err(AppError::business_displayable(format!(
                "出库长度不能大于现有长度（现有 {current_length} m，本次 {out_length} m）"
            )));
        }
        let new_qty_meters = current_qty_meters - out_length;
        if new_qty_meters < Decimal::ZERO {
            return Err(AppError::business_displayable(format!(
                "出库长度不能大于累计入库米数（累计 {current_qty_meters} m，本次 {out_length} m）"
            )));
        }
        update_fabric.length_m = Set(Some(new_length));
        update_fabric.quantity_meters = Set(Some(new_qty_meters));
        final_length = new_length;
    }

    // 根据剩余库存决定状态：取扣减后的真实 DECIMAL 值比较，不再经 f64 字符串往返
    // （`to_string().parse::<f64>()` 在 parse 失败时按 0 处理，会把仍有库存的卷误判成已出库，
    // 而「已出库」正是删除门控放行的那一档）。
    let new_status = if final_weight <= Decimal::ZERO && final_length <= Decimal::ZERO {
        greige_fabric_status::STOCKED_OUT.to_string()
    } else {
        greige_fabric_status::IN_STOCK.to_string()
    };

    update_fabric.status = Set(Some(new_status));
    update_fabric.updated_at = Set(crate::utils::date_utils::utc_now_fixed());

    // 同步更新备注（提供则覆盖，去空白后为空即清空；未提供保留原值）
    if req.remarks.is_some() {
        update_fabric.remarks = Set(blank_to_none(req.remarks));
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
