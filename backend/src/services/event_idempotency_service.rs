//! 事件幂等服务（B-P1-8 修复，批次 365 v13 复审）
//!
//! 提供 BusinessEvent 重复消费防护：消费者处理事件前在同一事务内尝试插入
//! processed_events 记录，主键冲突表示已处理，直接跳过。
//!
//! 幂等键设计：
//! - InventoryTransactionCreated → `inventory_txn:{transaction_id}`
//! - PaymentCompleted → `ap_paid:{invoice_id}`
//! - CollectionCompleted → `ar_paid:{invoice_id}`
//! - BpmProcessFinished → `bpm:{business_type}:{business_id}`
//! - LowStockAlert → `low_stock:{product_id}:{warehouse_id}:{date}`
//! - MaterialShortageAlert → `material_shortage:{material_id}:{date}`
//! - SalesOrderShipped → `sales_shipped:{order_id}`

use crate::models::processed_event;
use crate::utils::error::AppError;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait, QueryFilter, Set};
use std::sync::Arc;

/// 事件幂等服务
pub struct EventIdempotencyService {
    db: Arc<DatabaseConnection>,
}

impl EventIdempotencyService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 在指定事务内检查并标记事件为已处理。
    ///
    /// 若 (consumer_id, event_key) 已存在（主键冲突），返回 `Ok(false)` 表示已处理，
    /// 消费者应跳过本次处理；若插入成功返回 `Ok(true)`，消费者继续执行业务逻辑。
    ///
    /// 关键：必须在业务副作用的同一数据库事务内调用，保证原子性。
    pub async fn try_mark_processed_txn(
        &self,
        txn: &DatabaseTransaction,
        consumer_id: &str,
        event_key: &str,
        event_type: &str,
    ) -> Result<bool, AppError> {
        // 先查询是否已处理（避免依赖唯一约束报错，便于日志排查）
        let existing = processed_event::Entity::find()
            .filter(processed_event::Column::ConsumerId.eq(consumer_id))
            .filter(processed_event::Column::EventKey.eq(event_key))
            .one(txn)
            .await?;
        if existing.is_some() {
            tracing::info!(
                consumer_id = consumer_id,
                event_key = event_key,
                event_type = event_type,
                "事件已处理，幂等跳过"
            );
            return Ok(false);
        }

        let active_model = processed_event::ActiveModel {
            consumer_id: Set(consumer_id.to_string()),
            event_key: Set(event_key.to_string()),
            event_type: Set(event_type.to_string()),
            processed_at: Set(chrono::Utc::now()),
        };
        active_model.insert(txn).await?;
        Ok(true)
    }

    /// 无事务版本：用于无法在同一事务内完成幂等校验的场景（不推荐，存在并发窗口）。
    /// 推荐使用 `try_mark_processed_txn`。
    pub async fn try_mark_processed(
        &self,
        consumer_id: &str,
        event_key: &str,
        event_type: &str,
    ) -> Result<bool, AppError> {
        let txn = self.db.begin().await?;
        let result = self
            .try_mark_processed_txn(&txn, consumer_id, event_key, event_type)
            .await?;
        txn.commit().await?;
        Ok(result)
    }
}
