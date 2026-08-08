use axum::{extract::State, Json};
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

    validate_parent_piece(&parent, req.cut_length)?;

    // V15 P2 缺陷 3.2：确定原始长度（首次拆分时记录，后续复用）
    let original_length = parent.original_length.unwrap_or(parent.length + req.cut_length);
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

/// 校验母卷状态和剪裁长度
fn validate_parent_piece(
    parent: &inventory_piece::Model,
    cut_length: Decimal,
) -> Result<(), AppError> {
    if parent.status == piece_status::SHIPPED || parent.status == piece_status::UNAVAILABLE {
        return Err(AppError::bad_request(
            "当前布卷已发货或不可用，无法进行剪裁拆分".to_string(),
        ));
    }
    if parent.length < cut_length {
        return Err(AppError::bad_request(format!(
            "剪裁长度 ({}) 超过母卷可用长度 ({})",
            cut_length, parent.length
        )));
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

    // 如果母卷原本有重量，且输入了剪裁重量，则按比例或直接扣减
    if let (Some(pw), Some(cw)) = (parent.weight, cut_weight) {
        if pw >= cw {
            active_parent.weight = Set(Some(pw - cw));
        } else {
            return Err(AppError::bad_request(
                "剪裁重量不能大于母卷总重量".to_string(),
            ));
        }
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
fn generate_piece_no(parent: &inventory_piece::Model, new_barcode: &Option<String>) -> String {
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
        // v14 批次 416：dye_lot_id 为 NOT NULL 字段，拆分产生的新布卷继承母卷的缸号
        dye_lot_id: Set(parent.dye_lot_id),
        batch_no: Set(parent.batch_no.clone()),
        product_id: Set(parent.product_id),
        warehouse_id: Set(parent.warehouse_id),
        location_id: Set(parent.location_id),
        piece_no: Set(new_piece_no.clone()),
        barcode: Set(Some(new_piece_no)),
        parent_piece_id: Set(Some(parent.id)), // 关联母卷
        length: Set(cut_length),
        weight: Set(cut_weight),
        status: Set(piece_status::AVAILABLE.to_string()),
        remarks: Set(Some(format!("从布卷 {} 剪裁拆分而来", parent.piece_no))),
        scan_type: Set(None), // v11 批次 153 P2-A：拆分产生的新布卷无扫码类型
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        // v14 批次 416：新增的 nullable 字段，拆分产生的新布卷不设置这些字段
        supplier_piece_no: sea_orm::ActiveValue::NotSet,
        width: sea_orm::ActiveValue::NotSet,
        gram_weight: sea_orm::ActiveValue::NotSet,
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
