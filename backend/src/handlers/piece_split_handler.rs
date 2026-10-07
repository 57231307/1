use axum::{Json, extract::State};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::{Deserialize, Serialize};

use crate::container::AppState;
use crate::models::inventory_piece;
// 批次 236 v13 P1-1：库存裁片状态常量接入（规则 0）
use crate::models::status::inventory_piece as piece_status;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct SplitPieceRequest {
    /// 母卷/原始布卷 ID
    pub parent_piece_id: i32,
    /// 剪裁下来的新卷长度（米）
    pub cut_length: Decimal,
    /// 剪裁下来的新卷重量（公斤） - 选填
    pub cut_weight: Option<Decimal>,
    /// 新布卷条形码/编号 (如果为空则系统自动生成)
    pub new_barcode: Option<String>,
}

#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Serialize)]
pub struct SplitPieceResponse {
    pub message: String,
    pub parent_piece: inventory_piece::Model,
    pub new_piece: inventory_piece::Model,
}

pub async fn split_fabric_piece(
    State(state): State<AppState>,
    Json(req): Json<SplitPieceRequest>,
) -> Result<Json<ApiResponse<SplitPieceResponse>>, AppError> {
    let txn = state.db.begin().await?;

    // 1. 查询母卷
    let parent = inventory_piece::Entity::find_by_id(req.parent_piece_id)
        .one(&txn)
        .await?
        .ok_or_else(|| AppError::not_found("未找到母卷(原始布卷)"))?;

    validate_parent_piece(&parent, req.cut_length, req.cut_weight)?;

    // V15 P2 缺陷 3.2：确定原始长度（首次拆分时记录，后续复用）
    // original_length 为空即该匹从未拆过（生产报工建匹即为 None），拆分前母卷长度就是全长：
    // 剩余(parent.length - cut) + 子卷(cut) ≡ parent.length，与 validate_split_consistency
    // 的「剩余 + Σ子 == original」恒等式同式（旧式 length + cut 与之恒矛盾，成功路径不可达）。
    let original_length = parent.original_length.unwrap_or(parent.length);
    let original_weight = match (parent.original_weight, parent.weight, req.cut_weight) {
        (Some(ow), _, _) => Some(ow),
        (None, Some(pw), Some(cw)) => Some(pw + cw),
        _ => None,
    };

    // 2. 更新母卷剩余长度与重量 + 记录原始值
    let updated_parent = update_parent_piece(
        &parent,
        req.cut_length,
        req.cut_weight,
        original_length,
        original_weight,
        &txn,
    )
    .await?;

    // 3. 生成新布卷 (子卷)
    let new_piece_no = generate_piece_no(&parent, &req.new_barcode);
    let new_piece = build_new_piece(
        &parent,
        req.cut_length,
        req.cut_weight,
        new_piece_no,
        original_length,
        original_weight,
    );
    let inserted_piece = new_piece.insert(&txn).await?;

    // V15 P2 缺陷 3.2：校验 remaining + sum(children) = original
    validate_split_consistency(&updated_parent, original_length, &txn).await?;

    txn.commit().await?;

    Ok(Json(ApiResponse::success(SplitPieceResponse {
        message: "布卷剪裁拆分成功".to_string(),
        parent_piece: updated_parent,
        new_piece: inserted_piece,
    })))
}

/// 校验母卷状态、剪裁长度与剪裁重量额度
fn validate_parent_piece(
    parent: &inventory_piece::Model,
    cut_length: Decimal,
    cut_weight: Option<Decimal>,
) -> Result<(), AppError> {
    if parent.status == piece_status::SHIPPED || parent.status == piece_status::UNAVAILABLE {
        // 状态门：母卷已处于发货/不可用态，剪裁前置未满足，归业务族；
        // 文案只述公开规则、不含状态 token 与库存数字，可外显。
        return Err(AppError::business_displayable(
            "当前布卷已发货或不可用，无法进行剪裁拆分".to_string(),
        ));
    }
    if parent.length < cut_length {
        // 额度门：剪裁量受母卷可用长度约束，归业务族；文案含查询所得库存长度，
        // 按 error.rs 安全边界保持脱敏 business。
        return Err(AppError::business(format!(
            "剪裁长度 ({}) 超过母卷可用长度 ({})",
            cut_length, parent.length
        )));
    }
    // 实测重量取值域门（与 migration m0076 的 chk_inventory_piece_weight_positive、
    // 打卷门 fabric_inspection_service.rs:601-617、收回门 outsourcing_ops/receipt.rs:65-87
    // 逐字符同口径 "IS NULL OR > 0"）：0/负数属伪造实测值，落库后会被原样印上标签。
    crate::services::piece_domain_service::validate_piece_measured_value(
        "剪裁",
        "重量(weight)",
        Some(parent.piece_no.as_str()),
        cut_weight,
    )?;
    if let (Some(pw), Some(cw)) = (parent.weight, cut_weight)
        && cw >= pw
    {
        // 额度门（自 `update_parent_piece` 上提到入口，同一规则只此一处判定）：
        // 剪裁重量必须**小于**母卷实测重量。相等意味着母卷剩余实测重量被写成 0——
        // 那不是"称出来的 0 公斤"，而是"整卷拆尽后仍留一行布卷记录"的空壳形态，
        // 正是 m0076 值域要拦的伪实测值（且长度同理会留下 0 米母卷）。
        // 整卷流转应直接对母卷发货/出库，不需要先拆一条等量子卷再留一条空壳。
        // 文案不含查询所得公斤数（按 error.rs 安全边界不外显）。
        return Err(AppError::business_displayable(
            "剪裁重量必须小于该布卷的实测重量：整卷无需拆分，请直接按原卷出库或发货",
        ));
    }
    Ok(())
}

/// 更新母卷剩余长度与重量，返回更新后的母卷
async fn update_parent_piece(
    parent: &inventory_piece::Model,
    cut_length: Decimal,
    cut_weight: Option<Decimal>,
    original_length: Decimal,
    original_weight: Option<Decimal>,
    txn: &sea_orm::DatabaseTransaction,
) -> Result<inventory_piece::Model, AppError> {
    let mut active_parent: inventory_piece::ActiveModel = parent.clone().into();
    let remaining_length = parent.length - cut_length;
    active_parent.length = Set(remaining_length);

    // 母卷剩余实测重量 = 原值 - 本次剪裁值。额度与取值域已由 `validate_parent_piece`
    // 在任何写入前判定（含"剪裁重量必须小于母卷实测重量"，故此处差值必为正值，
    // 不会写出被 m0076 值域禁止的 0/负数）；本函数只负责如实落值，不再重复判规则。
    // 母卷无实测重量（None）或本次未提交剪裁重量时保持原值不动（不猜、不塞 0）。
    if let (Some(pw), Some(cw)) = (parent.weight, cut_weight) {
        active_parent.weight = Set(Some(pw - cw));
    }
    // V15 P2 缺陷 3.2：记录原始长度/重量（首次拆分时写入）
    active_parent.original_length = Set(Some(original_length));
    active_parent.original_weight = Set(original_weight);
    active_parent.updated_at = Set(Utc::now());
    Ok(active_parent.update(txn).await?)
}

/// V15 P2 缺陷 3.2：校验拆匹一致性
/// 母卷剩余长度 + 所有子卷长度之和 = 原始长度
async fn validate_split_consistency(
    parent: &inventory_piece::Model,
    original_length: Decimal,
    txn: &sea_orm::DatabaseTransaction,
) -> Result<(), AppError> {
    let children: Vec<inventory_piece::Model> = inventory_piece::Entity::find()
        .filter(inventory_piece::Column::ParentPieceId.eq(parent.id))
        .all(txn)
        .await?;

    let children_total: Decimal = children.iter().map(|c| c.length).sum();
    let computed_original = parent.length + children_total;

    if computed_original != original_length {
        return Err(AppError::bad_request(format!(
            "拆匹一致性校验失败：母卷剩余 ({}) + 子卷总长 ({}) = {}，原始长度为 {}",
            parent.length, children_total, computed_original, original_length
        )));
    }
    Ok(())
}

/// 生成新布卷编号（优先使用请求中的条码，否则自动生成）
/// batch-18 P3：改进匹号生成逻辑，使用日期+序列号格式
fn generate_piece_no(_parent: &inventory_piece::Model, new_barcode: &Option<String>) -> String {
    if let Some(barcode) = new_barcode {
        barcode.clone()
    } else {
        // 使用日期+毫秒时间戳格式，确保唯一性
        let now = Utc::now();
        let date_part = now.format("%Y%m%d");
        let time_part = now.timestamp_millis() % 100000; // 取后5位
        format!("P{}{:05}", date_part, time_part)
    }
}

/// 构建新布卷 ActiveModel（继承母卷属性）
fn build_new_piece(
    parent: &inventory_piece::Model,
    cut_length: Decimal,
    cut_weight: Option<Decimal>,
    new_piece_no: String,
    original_length: Decimal,
    original_weight: Option<Decimal>,
) -> inventory_piece::ActiveModel {
    inventory_piece::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        // v14 批次 416：拆分产生的新布卷继承母卷的缸号；m0051 起 dye_lot_id 可空（生产匹无缸号）
        dye_lot_id: Set(parent.dye_lot_id),
        // m0051 匹类型：拆分产生的子卷继承母卷类型（greige/dyed），机台号仅生产匹继承
        piece_type: Set(parent.piece_type.clone()),
        machine_no: Set(parent.machine_no.clone()),
        machine_operator: Set(parent.machine_operator.clone()),
        warehouse_in_at: Set(parent.warehouse_in_at),
        batch_no: Set(parent.batch_no.clone()),
        product_id: Set(parent.product_id),
        warehouse_id: Set(parent.warehouse_id),
        location_id: Set(parent.location_id),
        piece_no: Set(new_piece_no.clone()),
        barcode: Set(Some(new_piece_no)),
        parent_piece_id: Set(Some(parent.id)), // 关联母卷
        length: Set(cut_length),
        // 子卷重量 = 本次剪裁提交的实测重量（cut_weight），维持既有语义：剪裁量是按秤入的，
        // 不继承母卷整卷重量（那会把母卷剩余量重复计入）；未提交则保持 NULL（标签点名拒绝）。
        weight: Set(cut_weight),
        status: Set(piece_status::AVAILABLE.to_string()),
        remarks: Set(Some(format!("从布卷 {} 剪裁拆分而来", parent.piece_no))),
        scan_type: Set(None), // v11 批次 153 P2-A：拆分产生的新布卷无扫码类型
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        // 幅宽/克重：子卷是**同一母卷上剪下的另一段同一卷布**，横向裁切不改变幅宽与克重，
        // 因此按母卷实测值继承属物理同源（同一实测事实的延续记录），不是"缺值兜底"——
        // 母卷本身为 NULL（未补录）时子卷同样是 NULL，绝不猜值、不回落 products 标称值。
        // 与 length/weight 的处理不矛盾：那两个量随剪裁被物理分割，这两个量不被分割。
        width: Set(parent.width),
        gram_weight: Set(parent.gram_weight),
        // 其余 nullable 字段拆分产生的新布卷不设置（保持 NULL）
        supplier_piece_no: sea_orm::ActiveValue::NotSet,
        position_no: sea_orm::ActiveValue::NotSet,
        package_no: sea_orm::ActiveValue::NotSet,
        production_date: sea_orm::ActiveValue::NotSet,
        shelf_life: sea_orm::ActiveValue::NotSet,
        quality_status: sea_orm::ActiveValue::NotSet,
        inventory_status: sea_orm::ActiveValue::NotSet,
        created_by: sea_orm::ActiveValue::NotSet,
        updated_by: sea_orm::ActiveValue::NotSet,
        // 缺陷 3.1 修复：拆匹后子匹必须继承母卷的 color_no/dye_lot_no 字符串字段
        // 禁止 NotSet 导致子卷 dye_lot_no 为 NULL，破坏缸号字符串维度追溯
        color_no: Set(parent.color_no.clone()),
        dye_lot_no: Set(parent.dye_lot_no.clone()),
        // v14 批次 426：新增的 nullable 字段，拆分产生的新布卷不设置验布关联字段
        inspection_id: sea_orm::ActiveValue::NotSet,
        piece_seq: sea_orm::ActiveValue::NotSet,
        // V15 P2 缺陷 3.2：子卷继承母卷的原始长度/重量
        original_length: Set(Some(original_length)),
        original_weight: Set(original_weight),
    }
}
