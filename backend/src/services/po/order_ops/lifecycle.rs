//! 采购订单-生命周期子模块（order_ops/lifecycle）
//!
//! 拆分自原 `po/order.rs` 的 `impl PurchaseOrderService` 块。
//! 包含订单状态机驱动的生命周期方法：
//! - `close_order` 关闭采购订单（仅 COMPLETED / PARTIAL_RECEIVED 可关闭）
//!
//! 业务规则：
//! - 关闭操作走事务边界 + lock_exclusive 串行化并发关闭（批次 17 修复）
//! - update_with_audit 传 &txn 纳入事务，保证原子性
//! - 审计操作人使用真实 user_id（P1 1-1 修复，批次 59b）

use sea_orm::{EntityTrait, QuerySelect, Set, TransactionTrait};

use crate::models::{purchase_order, status};
use crate::services::po::order::PurchaseOrderService;
use crate::utils::error::AppError;

impl PurchaseOrderService {
    /// 关闭采购订单
    pub async fn close_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        // 批次 17（2026-06-28）：补全事务边界，原实现无事务且 update_with_audit 传 &*self.db 非原子
        let txn = (*self.db).begin().await?;

        // 1. 查询订单（加 lock_exclusive 串行化并发关闭）
        let order = purchase_order::Entity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态（已完成或部分入库的订单才能关闭）
        if ![
            status::purchase_order::COMPLETED,
            status::purchase_order::PARTIAL_RECEIVED,
        ]
        .contains(&order.order_status.as_str())
        {
            return Err(AppError::business(format!(
                "订单状态不允许关闭，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 更新状态（update_with_audit 传 &txn 纳入事务，保证原子性）
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set(status::purchase_order::CLOSED.to_string());
        order_active.updated_by = Set(Some(user_id));

        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(order)
    }
}
