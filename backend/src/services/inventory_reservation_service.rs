use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect,
    TransactionTrait,
};
use std::sync::Arc;

use crate::models::inventory_reservation::{self, Entity as InventoryReservationEntity};
use crate::utils::error::AppError;

/// 库存预留服务
pub struct InventoryReservationService {
    db: Arc<DatabaseConnection>,
}

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
    ) -> Result<inventory_reservation::Model, AppError> {
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

        reservation.insert(&*self.db).await.map_err(AppError::from)
    }

    /// 锁定预留（从 pending 到 locked）
    pub async fn lock_reservation(
        &self,
        reservation_id: i32,
    ) -> Result<inventory_reservation::Model, AppError> {
        // P1-8 修复（批次 79 v1 复审）：状态门 + update 移入单一事务，加 lock_exclusive 串行化
        // 原实现状态门查询在 self.db 上、update 也在 self.db 上，无事务边界，
        // 并发场景下可能在状态检查通过后、update 前发生状态变更，导致已锁定/已释放预留被重复锁定。
        let txn = (*self.db).begin().await?;

        let reservation = InventoryReservationEntity::find_by_id(reservation_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存预留 {} 未找到", reservation_id)))?;

        if reservation.status != "pending" {
            return Err(AppError::business(format!(
                "预留状态为{}，只有待处理状态的预留可以锁定",
                reservation.status
            )));
        }

        let mut reservation_update: inventory_reservation::ActiveModel = reservation.into();
        reservation_update.status = sea_orm::ActiveValue::Set("locked".to_string());
        reservation_update.updated_at = sea_orm::ActiveValue::Set(Utc::now());

        let result = reservation_update
            .update(&txn)
            .await
            .map_err(AppError::from)?;

        txn.commit().await?;
        Ok(result)
    }

    /// 释放预留（从 locked 到 released）
    pub async fn release_reservation(
        &self,
        reservation_id: i32,
    ) -> Result<inventory_reservation::Model, AppError> {
        // P1-9 修复（批次 79 v1 复审）：状态门 + update 移入单一事务，加 lock_exclusive 串行化
        // 原实现状态门查询在 self.db 上、update 也在 self.db 上，无事务边界，
        // 并发场景下可能在状态检查通过后、update 前发生状态变更，导致已释放预留被重复释放。
        let txn = (*self.db).begin().await?;

        let reservation = InventoryReservationEntity::find_by_id(reservation_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存预留 {} 未找到", reservation_id)))?;

        if reservation.status != "locked" && reservation.status != "pending" {
            return Err(AppError::business(format!(
                "预留状态为{}，只有已锁定或待处理状态的预留可以释放",
                reservation.status
            )));
        }

        let mut reservation_update: inventory_reservation::ActiveModel = reservation.into();
        reservation_update.status = sea_orm::ActiveValue::Set("released".to_string());
        reservation_update.released_at = sea_orm::ActiveValue::Set(Some(Utc::now()));
        reservation_update.updated_at = sea_orm::ActiveValue::Set(Utc::now());

        let result = reservation_update
            .update(&txn)
            .await
            .map_err(AppError::from)?;

        txn.commit().await?;
        Ok(result)
    }

    /// 获取预留列表
    pub async fn list_reservations(
        &self,
        page: u64,
        page_size: u64,
        product_id: Option<i32>,
        warehouse_id: Option<i32>,
        status: Option<String>,
    ) -> Result<(Vec<inventory_reservation::Model>, i64), AppError> {
        use sea_orm::PaginatorTrait;

        let mut query = InventoryReservationEntity::find();

        if let Some(pid) = product_id {
            query = query.filter(inventory_reservation::Column::ProductId.eq(pid));
        }
        if let Some(wid) = warehouse_id {
            query = query.filter(inventory_reservation::Column::WarehouseId.eq(wid));
        }
        if let Some(s) = status {
            query = query.filter(inventory_reservation::Column::Status.eq(s));
        }

        let paginator = query.paginate(&*self.db, page_size);
        let total = paginator.num_items().await? as i64;
        let reservations = paginator.fetch_page(page).await?;

        Ok((reservations, total))
    }

    /// 删除预留
    pub async fn delete_reservation(
        &self,
        reservation_id: i32,
        user_id: i32,
    ) -> Result<(), AppError> {
        // P1-10 修复（批次 79 v1 复审）：状态门 + delete_with_audit 移入单一事务，加 lock_exclusive 串行化
        // 原实现状态门查询在 self.db 上、delete_with_audit 也在 self.db 上，无事务边界，
        // 并发场景下可能在状态检查通过后、delete 前发生状态变更，导致已锁定预留被误删。
        let txn = (*self.db).begin().await?;

        let reservation = InventoryReservationEntity::find_by_id(reservation_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存预留 {} 未找到", reservation_id)))?;

        // 只有 pending 状态的预留可以删除
        if reservation.status != "pending" {
            return Err(AppError::business(format!(
                "预留状态为{}，只有待处理状态的预留可以删除",
                reservation.status
            )));
        }

        // P0 8-3 修复：delete 操作补审计日志
        // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            InventoryReservationEntity,
            _,
        >(&txn, "inventory_reservation", reservation_id, Some(user_id))
        .await?;

        txn.commit().await?;
        Ok(())
    }
}
