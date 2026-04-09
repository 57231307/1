use crate::utils::app_state::AppState;
use axum::{extract::State, Json};
use crate::models::inventory_piece;
use crate::utils::error::AppError;
use sea_orm::EntityTrait;

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
