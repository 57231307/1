use axum::{extract::State, Json};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set, QueryOrder};
use serde::{Deserialize, Serialize};

use crate::models::{color_card, color_card_record};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;

#[derive(Deserialize)]
pub struct CreateColorCardReq {
    pub card_no: String,
    pub product_id: i32,
    pub season: Option<String>,
    pub description: Option<String>,
    pub stock_quantity: Option<i32>,
}

pub async fn list_color_cards(
    State(state): State<AppState>,
) -> Result<Json<Vec<color_card::Model>>, AppError> {
    let cards = color_card::Entity::find()
        .order_by_desc(color_card::Column::CreatedAt)
        .all(&*state.db)
        .await?;
    Ok(Json(cards))
}

pub async fn create_color_card(
    State(state): State<AppState>,
    Json(req): Json<CreateColorCardReq>,
) -> Result<Json<color_card::Model>, AppError> {
    let now = Utc::now();
    let new_card = color_card::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        card_no: Set(req.card_no),
        product_id: Set(req.product_id),
        season: Set(req.season),
        description: Set(req.description),
        stock_quantity: Set(req.stock_quantity.or(Some(0))),
        created_at: Set(now),
        updated_at: Set(now),
    };
    
    let inserted = new_card.insert(&*state.db).await?;
    Ok(Json(inserted))
}

#[derive(Deserialize)]
pub struct IssueColorCardReq {
    pub color_card_id: i32,
    pub customer_id: i32,
    pub notes: Option<String>,
}

pub async fn issue_color_card(
    State(state): State<AppState>,
    Json(req): Json<IssueColorCardReq>,
) -> Result<Json<color_card_record::Model>, AppError> {
    let now = Utc::now();
    let record = color_card_record::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        color_card_id: Set(req.color_card_id),
        customer_id: Set(req.customer_id),
        issue_date: Set(now),
        return_date: Set(None),
        status: Set(Some("ISSUED".to_string())),
        notes: Set(req.notes),
        created_at: Set(now),
        updated_at: Set(now),
    };
    
    let inserted = record.insert(&*state.db).await?;
    Ok(Json(inserted))
}

pub async fn list_color_card_records(
    State(state): State<AppState>,
) -> Result<Json<Vec<color_card_record::Model>>, AppError> {
    let records = color_card_record::Entity::find()
        .order_by_desc(color_card_record::Column::CreatedAt)
        .all(&*state.db)
        .await?;
    Ok(Json(records))
}
