use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use std::sync::Arc;
use chrono::Utc;

use crate::models::inventory_reservation::{self, Entity as InventoryReservationEntity};

/// 库存预留服务
#[allow(dead_code)]
pub struct InventoryReservationService {
    db: Arc<DatabaseConnection>,
}

#[allow(dead_code)]
impl InventoryReservationService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建库存预留
    pub async fn create_reservation(
        &self,
        order_id: i32,
        product_id: i32,
        warehouse_id: i32,
        quantity: rust_decimal::Decimal,
        created_by: Option<i32>,
        notes: Option<String>,
    ) -> Result<inventory_reservation::Model, sea_orm::DbErr> {
        let reservation = inventory_reservation::ActiveModel {
            id: Default::default(),
            order_id: sea_orm::ActiveValue::Set(order_id),
            product_id: sea_orm::ActiveValue::Set(product_id),
            warehouse_id: sea_orm::ActiveValue::Set(warehouse_id),
            quantity: sea_orm::ActiveValue::Set(quantity),
            status: sea_orm::ActiveValue::Set("pending".to_string()),
            reserved_at: sea_orm::ActiveValue::Set(Utc::now()),
            released_at: sea_orm::ActiveValue::NotSet,
            notes: sea_orm::ActiveValue::Set(notes),
            created_by: sea_orm::ActiveValue::Set(created_by),
            created_at: sea_orm::ActiveValue::Set(Utc::now()),
            updated_at: sea_orm::ActiveValue::Set(Utc::now()),
        };

        reservation.insert(&*self.db).await
    }

    /// 锁定预留（从 pending 到 locked）
    pub async fn lock_reservation(
        &self,
        reservation_id: i32,
    ) -> Result<inventory_reservation::Model, sea_orm::DbErr> {
        let reservation = InventoryReservationEntity::find_by_id(reservation_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                sea_orm::DbErr::RecordNotFound(format!("库存预留 {} 未找到", reservation_id))
            })?;

        if reservation.status != "pending" {
            return Err(sea_orm::DbErr::Custom(
                format!("预留状态为{}，只有待处理状态的预留可以锁定", reservation.status)
            ));
        }

        let mut reservation_update: inventory_reservation::ActiveModel = reservation.into();
        reservation_update.status = sea_orm::ActiveValue::Set("locked".to_string());
        reservation_update.updated_at = sea_orm::ActiveValue::Set(Utc::now());

        reservation_update.update(&*self.db).await
    }

    /// 释放预留（从 locked 到 released）
    pub async fn release_reservation(
        &self,
        reservation_id: i32,
    ) -> Result<inventory_reservation::Model, sea_orm::DbErr> {
        let reservation = InventoryReservationEntity::find_by_id(reservation_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                sea_orm::DbErr::RecordNotFound(format!("库存预留 {} 未找到", reservation_id))
            })?;

        if reservation.status != "locked" && reservation.status != "pending" {
            return Err(sea_orm::DbErr::Custom(
                format!("预留状态为{}，只有已锁定或待处理状态的预留可以释放", reservation.status)
            ));
        }

        let mut reservation_update: inventory_reservation::ActiveModel = reservation.into();
        reservation_update.status = sea_orm::ActiveValue::Set("released".to_string());
        reservation_update.released_at = sea_orm::ActiveValue::Set(Some(Utc::now()));
        reservation_update.updated_at = sea_orm::ActiveValue::Set(Utc::now());

        reservation_update.update(&*self.db).await
    }

    /// 使用预留（从 locked 到 used）- 通常在发货时调用
    pub async fn use_reservation(
        &self,
        reservation_id: i32,
    ) -> Result<inventory_reservation::Model, sea_orm::DbErr> {
        let reservation = InventoryReservationEntity::find_by_id(reservation_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                sea_orm::DbErr::RecordNotFound(format!("库存预留 {} 未找到", reservation_id))
            })?;

        if reservation.status != "locked" {
            return Err(sea_orm::DbErr::Custom(
                format!("预留状态为{}，只有已锁定状态的预留可以使用", reservation.status)
            ));
        }

        let mut reservation_update: inventory_reservation::ActiveModel = reservation.into();
        reservation_update.status = sea_orm::ActiveValue::Set("used".to_string());
        reservation_update.updated_at = sea_orm::ActiveValue::Set(Utc::now());

        reservation_update.update(&*self.db).await
    }

    /// 根据订单 ID 获取所有预留
    pub async fn get_reservations_by_order(
        &self,
        order_id: i32,
    ) -> Result<Vec<inventory_reservation::Model>, sea_orm::DbErr> {
        InventoryReservationEntity::find()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .all(&*self.db)
            .await
    }

    /// 根据订单 ID 获取所有已锁定的预留
    pub async fn get_locked_reservations_by_order(
        &self,
        order_id: i32,
    ) -> Result<Vec<inventory_reservation::Model>, sea_orm::DbErr> {
        InventoryReservationEntity::find()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(inventory_reservation::Column::Status.eq("locked"))
            .all(&*self.db)
            .await
    }

    /// 批量创建预留（用于订单审核时）
    pub async fn batch_create_reservations(
        &self,
        order_id: i32,
        items: Vec<(i32, i32, rust_decimal::Decimal)>, // (product_id, warehouse_id, quantity)
        created_by: Option<i32>,
    ) -> Result<Vec<inventory_reservation::Model>, sea_orm::DbErr> {
        let mut reservations = Vec::new();

        for (product_id, warehouse_id, quantity) in items {
            let reservation = self
                .create_reservation(order_id, product_id, warehouse_id, quantity, created_by, None)
                .await?;
            reservations.push(reservation);
        }

        Ok(reservations)
    }

    /// 批量锁定预留
    pub async fn batch_lock_reservations(
        &self,
        reservation_ids: Vec<i32>,
    ) -> Result<Vec<inventory_reservation::Model>, sea_orm::DbErr> {
        let mut locked = Vec::new();

        for id in reservation_ids {
            match self.lock_reservation(id).await {
                Ok(reservation) => locked.push(reservation),
                Err(e) => return Err(e),
            }
        }

        Ok(locked)
    }

    /// 批量释放预留
    pub async fn batch_release_reservations(
        &self,
        reservation_ids: Vec<i32>,
    ) -> Result<Vec<inventory_reservation::Model>, sea_orm::DbErr> {
        let mut released = Vec::new();

        for id in reservation_ids {
            match self.release_reservation(id).await {
                Ok(reservation) => released.push(reservation),
                Err(e) => return Err(e),
            }
        }

        Ok(released)
    }
}
