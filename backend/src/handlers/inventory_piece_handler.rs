use crate::utils::app_state::AppState;
use axum::{extract::{State, Path}, Json};
use crate::models::inventory_piece;
use crate::utils::error::AppError;
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set, TransactionTrait};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use chrono::Utc;


#[derive(Debug, Clone, serde::Serialize)]
pub struct RollResponse {
    pub roll_no: String,
    pub batch_no: String,
    pub length: f64,
    pub defect_points: f64,
}

pub async fn list_pieces(
    State(state): State<AppState>,
) -> Result<Json<Vec<RollResponse>>, AppError> {
    let pieces = inventory_piece::Entity::find().all(&*state.db).await?;
    
    let rolls = pieces.into_iter().map(|p| RollResponse {
        roll_no: p.piece_no,
        batch_no: p.batch_no,
        length: p.length.to_string().parse().unwrap_or(0.0),
        defect_points: 0.0,
    }).collect();
    
    Ok(Json(rolls))
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SplitPieceRequest {
    pub split_length: f64, // The length to cut off
}

pub async fn split_piece(
    State(state): State<AppState>,
    Path(piece_no): Path<String>,
    Json(req): Json<SplitPieceRequest>,
) -> Result<Json<RollResponse>, AppError> {
    let txn = state.db.begin().await?;
    
    // 1. Find original piece
    let original_piece = inventory_piece::Entity::find()
        .filter(inventory_piece::Column::PieceNo.eq(piece_no.clone()))
        .one(&txn)
        .await?;
        
    let original_piece = match original_piece {
        Some(p) => p,
        None => return Err(AppError::NotFound(format!("找不到匹号: {}", piece_no))),
    };
    
    let original_len_f64 = original_piece.length.to_string().parse::<f64>().unwrap_or(0.0);
    if original_len_f64 <= req.split_length {
        return Err(AppError::BadRequest("拆分长度必须小于原卷总长度".to_string()));
    }
    
    let new_len_f64 = original_len_f64 - req.split_length;
    let original_weight = original_piece.weight.unwrap_or(Decimal::ZERO).to_string().parse::<f64>().unwrap_or(0.0);
    
    // Calculate proportional weight
    let split_weight = if original_len_f64 > 0.0 {
        (req.split_length / original_len_f64) * original_weight
    } else {
        0.0
    };
    let new_weight = original_weight - split_weight;
    
    // 2. Update original piece
    let mut active_original: inventory_piece::ActiveModel = original_piece.clone().into();
    active_original.length = Set(Decimal::from_f64(new_len_f64).unwrap_or(Decimal::ZERO));
    active_original.weight = Set(Some(Decimal::from_f64(new_weight).unwrap_or(Decimal::ZERO)));
    active_original.updated_at = Set(Utc::now());
    active_original.update(&txn).await?;
    
    // 3. Create new piece
    let timestamp = Utc::now().timestamp_subsec_millis();
    let new_piece_no = format!("{}-S{}", piece_no, timestamp);
    
    let new_piece = inventory_piece::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        batch_no: Set(original_piece.batch_no.clone()),
        product_id: Set(original_piece.product_id),
        warehouse_id: Set(original_piece.warehouse_id),
        location_id: Set(original_piece.location_id),
        piece_no: Set(new_piece_no.clone()),
        length: Set(Decimal::from_f64(req.split_length).unwrap_or(Decimal::ZERO)),
        weight: Set(Some(Decimal::from_f64(split_weight).unwrap_or(Decimal::ZERO))),
        status: Set("AVAILABLE".to_string()),
        remarks: Set(Some(format!("从 {} 拆分/剪样产生", piece_no))),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
    };
    
    new_piece.insert(&txn).await?;
    
    txn.commit().await?;
    
    Ok(Json(RollResponse {
        roll_no: new_piece_no,
        batch_no: original_piece.batch_no,
        length: req.split_length,
        defect_points: 0.0,
    }))
}
