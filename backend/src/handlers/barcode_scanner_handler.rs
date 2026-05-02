use axum::{
    extract::State,
    Json,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::Deserialize;

use crate::models::inventory_piece;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

#[derive(Deserialize)]
pub struct ScanToShipRequest {
    pub barcode: String,
    pub order_id: i32,
}

pub async fn scan_to_ship(
    State(state): State<AppState>,
    Json(req): Json<ScanToShipRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let txn = state.db.begin().await?;

    // 1. 通过条码查询布卷
    let piece = inventory_piece::Entity::find()
        .filter(inventory_piece::Column::Barcode.eq(&req.barcode))
        .one(&txn)
        .await?
        .ok_or_else(|| AppError::NotFound("未找到该条码对应的布卷".to_string()))?;

    if piece.status == "SHIPPED" {
        return Err(AppError::BadRequest("该布卷已发货".to_string()));
    }

    // 2. 更新布卷状态为已发货
    let mut active_piece: inventory_piece::ActiveModel = piece.clone().into();
    active_piece.status = Set("SHIPPED".to_string());
    active_piece.updated_at = Set(Utc::now());
    active_piece.update(&txn).await?;

    txn.commit().await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "布卷扫码出库成功",
        "barcode": req.barcode,
        "piece_no": piece.piece_no
    }))))
}
