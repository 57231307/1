//! 销售合同/审批工作流服务（so/contract）
//!
//! 包含销售订单的拒绝等审批工作流方法。
//! submit_order / approve_order / complete_order 已在 so/order.rs 中实现，
//! 这里仅补充合同专属业务（驳回、释放库存预留等）。
//!
//! 拆分自原 `sales_service.rs`。

use crate::models::{customer, sales_order};
use crate::models::status::sales_order as so_status;
use crate::services::so::order::SalesService;
use crate::utils::error::AppError;
use sea_orm::{EntityTrait, QuerySelect, Set, TransactionTrait};

impl SalesService {
    /// 拒绝销售订单
    pub async fn reject_order(
        &self,
        order_id: i32,
        reason: String,
        user_id: i32,
    ) -> Result<(), AppError> {
        // P1-13 修复（批次 79 v1 复审）：状态门 + 客户校验移入单一事务，加 lock_exclusive 串行化
        // 原实现状态门用 self.db 裸查询、客户校验也用 self.db，txn 只包裹 release + update，
        // 并发场景下可能在状态检查通过后、update 前发生状态变更（如已 approve），导致已审批单被拒绝。
        let txn = (*self.db).begin().await?;

        let order = sales_order::Entity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("订单不存在"))?;

        if order.status != so_status::PENDING {
            return Err(AppError::business(format!(
                "订单状态为{}，不允许拒绝",
                order.status
            )));
        }

        // 校验客户存在（保持与原业务一致，在事务内执行）
        let _customer = customer::Entity::find_by_id(order.customer_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("客户不存在"))?;

        // 释放库存预留
        self.release_reservations(order_id, &txn).await?;

        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = Set(so_status::REJECTED.to_string());
        order_update.notes = Set(Some(reason));
        order_update.updated_at = Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_update,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        txn.commit().await?;
        Ok(())
    }
}
