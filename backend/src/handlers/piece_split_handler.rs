use axum::{
    extract::State,
    Json,
};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, EntityTrait, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};

use crate::models::inventory_piece;
use crate::utils::app_state::AppState;
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
        .ok_or_else(|| AppError::NotFound("未找到母卷(原始布卷)".to_string()))?;

    // 状态检查
    if parent.status == "SHIPPED" || parent.status == "UNAVAILABLE" {
        return Err(AppError::BadRequest("当前布卷已发货或不可用，无法进行剪裁拆分".to_string()));
    }

    // 长度校验
    if parent.length < req.cut_length {
        return Err(AppError::BadRequest(format!(
            "剪裁长度 ({}) 超过母卷可用长度 ({})",
            req.cut_length, parent.length
        )));
    }

    // 2. 更新母卷剩余长度与重量
    let mut active_parent: inventory_piece::ActiveModel = parent.clone().into();
    let remaining_length = parent.length - req.cut_length;
    active_parent.length = Set(remaining_length);
    
    // 如果母卷原本有重量，且输入了剪裁重量，则按比例或直接扣减
    if let (Some(pw), Some(cw)) = (parent.weight, req.cut_weight) {
        if pw >= cw {
            active_parent.weight = Set(Some(pw - cw));
        } else {
            return Err(AppError::BadRequest("剪裁重量不能大于母卷总重量".to_string()));
        }
    }
    active_parent.updated_at = Set(Utc::now());
    let updated_parent = active_parent.update(&txn).await?;

    // 3. 生成新布卷 (子卷)
    let new_piece_no = req.new_barcode.unwrap_or_else(|| format!("{}-CUT-{}", parent.piece_no, Utc::now().timestamp_subsec_millis()));
    
    let new_piece = inventory_piece::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        batch_no: Set(parent.batch_no.clone()),
        product_id: Set(parent.product_id),
        warehouse_id: Set(parent.warehouse_id),
        location_id: Set(parent.location_id),
        piece_no: Set(new_piece_no.clone()),
        barcode: Set(Some(new_piece_no)),
        parent_piece_id: Set(Some(parent.id)), // 关联母卷
        length: Set(req.cut_length),
        weight: Set(req.cut_weight),
        status: Set("AVAILABLE".to_string()),
        remarks: Set(Some(format!("从布卷 {} 剪裁拆分而来", parent.piece_no))),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
    };

    let inserted_piece = new_piece.insert(&txn).await?;

    txn.commit().await?;

    Ok(Json(ApiResponse::success(SplitPieceResponse {
        message: "布卷剪裁拆分成功".to_string(),
        parent_piece: updated_parent,
        new_piece: inserted_piece,
    })))
}
