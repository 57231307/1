use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::Deserialize;

use crate::models::logistics_waybill;
use crate::models::sales_order;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

#[derive(Deserialize)]
pub struct CreateWaybillRequest {
    pub order_id: i32,
    pub logistics_company: String,
    pub tracking_number: String,
    pub driver_name: Option<String>,
    pub driver_phone: Option<String>,
    pub freight_fee: Option<f64>,
    pub expected_arrival: Option<chrono::DateTime<Utc>>,
    pub notes: Option<String>,
}

pub async fn create_waybill(
    State(state): State<AppState>,
    Json(req): Json<CreateWaybillRequest>,
) -> Result<Json<ApiResponse<logistics_waybill::Model>>, AppError> {
    let txn = state.db.begin().await?;

    // Verify order exists
    let order = sales_order::Entity::find_by_id(req.order_id)
        .one(&txn)
        .await?
        .ok_or_else(|| AppError::NotFound("订单不存在".to_string()))?;

    // Create waybill
    let freight = req.freight_fee.map(|f| Decimal::from_f64_retain(f).unwrap_or(Decimal::ZERO));
    
    let new_waybill = logistics_waybill::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        order_id: Set(req.order_id),
        logistics_company: Set(req.logistics_company),
        tracking_number: Set(req.tracking_number),
        driver_name: Set(req.driver_name),
        driver_phone: Set(req.driver_phone),
        freight_fee: Set(freight),
        status: Set(Some("IN_TRANSIT".to_string())),
        expected_arrival: Set(req.expected_arrival),
        actual_arrival: Set(None),
        notes: Set(req.notes),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
    };

    let inserted = new_waybill.insert(&txn).await?;

    // Update order status to DELIVERING / SHIPPED if not already
    if order.status != "SHIPPED" {
        let mut active_order: sales_order::ActiveModel = order.into();
        active_order.status = Set("SHIPPED".to_string());
        active_order.update(&txn).await?;
    }

    txn.commit().await?;

    Ok(Json(ApiResponse::success(inserted)))
}

pub async fn list_waybills(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<logistics_waybill::Model>>>, AppError> {
    let waybills = logistics_waybill::Entity::find()
        .order_by_desc(logistics_waybill::Column::CreatedAt)
        .all(&*state.db)
        .await?;

    Ok(Json(ApiResponse::success(waybills)))
}

#[derive(Deserialize)]
pub struct UpdateWaybillStatusReq {
    pub status: String,
}

pub async fn update_waybill_status(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateWaybillStatusReq>,
) -> Result<Json<ApiResponse<logistics_waybill::Model>>, AppError> {
    let waybill = logistics_waybill::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("运单不存在".to_string()))?;

    let mut active_waybill: logistics_waybill::ActiveModel = waybill.into();
    active_waybill.status = Set(Some(req.status.clone()));
    
    if req.status == "DELIVERED" {
        active_waybill.actual_arrival = Set(Some(Utc::now()));
    }
    
    active_waybill.updated_at = Set(Utc::now());
    let updated = active_waybill.update(&*state.db).await?;

    Ok(Json(ApiResponse::success(updated)))
}
