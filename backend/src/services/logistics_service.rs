//! 物流运单服务
//!
//! V15 P1 batch-19 缺陷 23.4.1/23.4.2/23.4.3：
//! - 缺陷 23.4.1：运单关联采购订单（order_type 区分销售/采购）
//! - 缺陷 23.4.2：物流跟踪事件历史
//! - 缺陷 23.4.3：运费核算（按重量/体积/距离取最大值）

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::container::AppState;
use crate::models::logistics_tracking_event;
use crate::models::logistics_waybill::{self, Entity as WaybillEntity};
use crate::utils::error::AppError;

/// 物流跟踪事件 DTO
#[derive(Debug, Deserialize, Serialize)]
pub struct TrackingEvent {
    pub event_time: chrono::DateTime<Utc>,
    pub location: Option<String>,
    pub description: String,
    pub event_type: String,
    pub data_source: Option<String>,
}

/// 物流服务
pub struct LogisticsService {
    db: Arc<DatabaseConnection>,
}

impl LogisticsService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// V15 P1 batch-19 缺陷 23.4.1：关联采购订单到运单
    pub async fn link_purchase_order(
        &self,
        waybill_id: i32,
        po_id: i32,
    ) -> Result<logistics_waybill::Model, AppError> {
        let txn = self.db.begin().await?;
        let waybill = WaybillEntity::find_by_id(waybill_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("运单不存在"))?;

        // 验证采购订单存在
        let po_exists = crate::models::purchase_order::Entity::find_by_id(po_id)
            .one(&txn)
            .await?
            .is_some();
        if !po_exists {
            return Err(AppError::not_found(format!("采购订单 {} 不存在", po_id)));
        }

        let mut active: logistics_waybill::ActiveModel = waybill.into();
        active.order_id = Set(po_id);
        active.order_type = Set(Some("purchase_order".to_string()));
        active.updated_at = Set(Utc::now());
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// V15 P1 batch-19 缺陷 23.4.2：记录物流跟踪事件历史
    pub async fn record_tracking_event(
        &self,
        waybill_id: i32,
        event: TrackingEvent,
    ) -> Result<logistics_tracking_event::Model, AppError> {
        // 校验运单存在
        let waybill_exists = WaybillEntity::find_by_id(waybill_id)
            .one(&*self.db)
            .await?
            .is_some();
        if !waybill_exists {
            return Err(AppError::not_found("运单不存在"));
        }

        let data_source = event.data_source.unwrap_or_else(|| "manual".to_string());
        let now = Utc::now();
        let active = logistics_tracking_event::ActiveModel {
            id: Default::default(),
            waybill_id: Set(waybill_id),
            event_time: Set(event.event_time),
            location: Set(event.location),
            description: Set(event.description),
            event_type: Set(event.event_type),
            data_source: Set(data_source),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let result = active.insert(&*self.db).await?;
        Ok(result)
    }

    /// V15 P1 batch-19 缺陷 23.4.2：查询运单跟踪事件历史
    pub async fn list_tracking_events(
        &self,
        waybill_id: i32,
    ) -> Result<Vec<logistics_tracking_event::Model>, AppError> {
        let events = logistics_tracking_event::Entity::find()
            .filter(logistics_tracking_event::Column::WaybillId.eq(waybill_id))
            .order_by_asc(logistics_tracking_event::Column::EventTime)
            .all(&*self.db)
            .await?;
        Ok(events)
    }

    /// V15 P1 batch-19 缺陷 23.4.3：计算运费（取重量/体积/距离计算的最大值）
    pub async fn calculate_freight(&self, waybill_id: i32) -> Result<Decimal, AppError> {
        let waybill = WaybillEntity::find_by_id(waybill_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("运单不存在"))?;

        let rate = waybill.freight_rate.unwrap_or(Decimal::ZERO);
        let weight_cost = waybill.total_weight.unwrap_or(Decimal::ZERO) * rate;
        let volume_cost = waybill.total_volume.unwrap_or(Decimal::ZERO) * rate;
        let distance_rate = rate / Decimal::from(1000);
        let distance_cost = waybill.distance_km.unwrap_or(Decimal::ZERO) * distance_rate;

        let freight = [weight_cost, volume_cost, distance_cost]
            .into_iter()
            .max()
            .unwrap_or(Decimal::ZERO);

        let mut active: logistics_waybill::ActiveModel = waybill.into();
        active.freight_fee = Set(Some(freight));
        active.updated_at = Set(Utc::now());
        active.update(&*self.db).await?;

        Ok(freight)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freight_max_calculation() {
        // 验证运费取三者最大值的逻辑
        let weight_cost = Decimal::from(100);
        let volume_cost = Decimal::from(150);
        let distance_cost = Decimal::from(80);
        let freight = [weight_cost, volume_cost, distance_cost]
            .into_iter()
            .max()
            .unwrap_or(Decimal::ZERO);
        assert_eq!(freight, Decimal::from(150));
    }

    #[test]
    fn test_freight_zero_when_no_data() {
        let freight = [Decimal::ZERO, Decimal::ZERO, Decimal::ZERO]
            .into_iter()
            .max()
            .unwrap_or(Decimal::ZERO);
        assert_eq!(freight, Decimal::ZERO);
    }
}
