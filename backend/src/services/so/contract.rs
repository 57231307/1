//! 销售合同/审批工作流服务（so/contract）
//!
//! 包含销售订单的拒绝等审批工作流方法。
//! submit_order / approve_order / complete_order 已在 so/order.rs 中实现，
//! 这里仅补充合同专属业务（驳回、释放库存预留等）。
//!
//! 拆分自原 `sales_service.rs`。

use crate::models::{customer, sales_order};
use crate::services::so::order::SalesService;
use crate::utils::error::AppError;
use sea_orm::{EntityTrait, Set, TransactionTrait};

impl SalesService {
    /// 拒绝销售订单
    pub async fn reject_order(
        &self,
        order_id: i32,
        reason: String,
        _user_id: i32,
    ) -> Result<(), AppError> {
        let order = sales_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("订单不存在"))?;

        if order.status != "pending" {
            return Err(AppError::business(format!(
                "订单状态为{}，不允许拒绝",
                order.status
            )));
        }

        // 校验客户存在（保持与原业务一致）
        let _customer = customer::Entity::find_by_id(order.customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("客户不存在"))?;

        let txn = (*self.db).begin().await?;

        // 释放库存预留
        self.release_reservations(order_id, &txn).await?;

        let mut order_update: sales_order::ActiveModel = order.into();
        order_update.status = Set("rejected".to_string());
        order_update.notes = Set(Some(reason));
        order_update.updated_at = Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_update,
            Some(0),
        )
        .await?;

        txn.commit().await?;
        Ok(())
    }
}
