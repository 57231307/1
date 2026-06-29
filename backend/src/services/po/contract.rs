//! 采购合同/审批工作流服务（po/contract）
//!
//! 包含采购订单的提交、审批、拒绝等审批工作流方法。
//! 拆分自原 `purchase_order_service.rs`。

use crate::models::{purchase_order, purchase_order_item, status};
use crate::utils::error::AppError;
use chrono::Utc;
use sea_orm::{
    ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};

use super::order::PurchaseOrderService;

impl PurchaseOrderService {
    /// 提交采购订单
    ///
    /// 批次 22（2026-06-28 v5 P0-5）：补全事务边界 + lock_exclusive + 真实 user_id
    /// 原 `submit_order` 在 `&*self.db` 上裸查询 + 裸更新，无事务边界也无行锁，
    /// 并发提交同一订单可能基于过期快照导致状态覆盖；
    /// 同时 `update_with_audit` 的 user_id 传入 `Some(0)` 导致审计日志用户缺失。
    /// 改为：begin txn + lock_exclusive 查询 + 状态/权限/明细校验 + update_with_audit(&txn, Some(user_id)) + commit；
    /// BPM 启动保留事务外（与批次 12 一致：失败 warn 不阻断已提交状态），避免 BPM 调用持有数据库锁。
    pub async fn submit_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询订单（加 lock_exclusive 串行化并发提交）
        let order = purchase_order::Entity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态
        if order.order_status != status::purchase_order::DRAFT
            && order.order_status != status::purchase_order::REJECTED
        {
            return Err(AppError::business(format!(
                "订单状态不允许提交，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 检查权限
        if order.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能提交自己创建的订单".to_string(),
            ));
        }

        // 4. 检查是否有明细（事务内查询以保证快照一致）
        let item_count = purchase_order_item::Entity::find()
            .filter(purchase_order_item::Column::OrderId.eq(order_id))
            .count(&txn)
            .await?;

        if item_count == 0 {
            return Err(AppError::business("订单至少需要一行明细"));
        }

        // 5. 更新状态为 PENDING_APPROVAL（走 update_with_audit 保留审计追溯，使用真实 user_id）
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set(status::purchase_order::PENDING_APPROVAL.to_string());
        order_active.updated_at = Set(Utc::now());
        order_active.updated_by = Set(Some(user_id));

        let updated_order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        // 6. 挂载 BPM 引擎（事务外，失败不阻断已提交状态）
        let bpm_service = crate::services::bpm_service::BpmService::new(self.db.clone());
        let req = crate::models::dto::bpm_dto::StartProcessRequest {
            process_key: "purchase_order_approval".to_string(),
            business_type: "purchase_order".to_string(),
            business_id: order_id,
            title: format!("采购订单审批 - {}", updated_order.order_no),
            initiator_id: user_id,
            initiator_name: String::new(),
            initiator_department_id: None,
            priority: None,
            form_data: None,
            variables: None,
        };
        // P0 修复（批次 4，2026-06-27）：原 `let _ = ...` 静默吞掉 BPM 启动错误，
        // 改为 warn 日志记录，保留兼容性（不阻断主流程），确保运维可观测。
        if let Err(e) = bpm_service.start_process(req).await {
            tracing::warn!(
                error = %e,
                order_id = order_id,
                "BPM 启动采购订单审批流程失败（兼容旧数据，不阻断主流程）"
            );
        }

        Ok(updated_order)
    }

    /// 审批采购订单
    ///
    /// 批次 22（2026-06-28 v5 P0-5）：补全事务边界 + lock_exclusive + 真实 user_id
    /// 原 `approve_order` 在 `&*self.db` 上裸查询 + 裸更新，无事务边界也无行锁，
    /// 并发审批同一订单可能基于过期快照导致重复审批或状态覆盖；
    /// 同时 `update_with_audit` 的 user_id 传入 `Some(0)` 导致审计日志用户缺失。
    /// 改为：begin txn + lock_exclusive 查询 + 状态校验 + update_with_audit(&txn, Some(user_id)) + commit。
    pub async fn approve_order(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询订单（加 lock_exclusive 串行化并发审批）
        let order = purchase_order::Entity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态 - 只有待审批状态的订单才能审批
        if order.order_status != status::purchase_order::PENDING_APPROVAL {
            return Err(AppError::business(format!(
                "订单状态不允许审批，当前状态：{}，需要状态：PENDING_APPROVAL",
                order.order_status
            )));
        }

        // 3. 更新状态（走 update_with_audit 保留审计追溯，使用真实 user_id）
        let now = chrono::Utc::now();
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set(status::purchase_order::APPROVED.to_string());
        order_active.approved_by = Set(Some(user_id));
        order_active.approved_at = Set(Some(now));
        order_active.updated_by = Set(Some(user_id));
        order_active.updated_at = Set(now);

        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(order)
    }

    /// 拒绝采购订单
    ///
    /// 批次 22（2026-06-28 v5 P0-5）：补全事务边界 + lock_exclusive + 真实 user_id
    /// 原 `reject_order` 在 `&*self.db` 上裸查询 + 裸更新，无事务边界也无行锁，
    /// 并发拒绝同一订单可能基于过期快照导致重复拒绝或状态覆盖；
    /// 同时 `update_with_audit` 的 user_id 传入 `Some(0)` 导致审计日志用户缺失。
    /// 改为：begin txn + lock_exclusive 查询 + 状态校验 + update_with_audit(&txn, Some(user_id)) + commit。
    pub async fn reject_order(
        &self,
        order_id: i32,
        reason: String,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询订单（加 lock_exclusive 串行化并发拒绝）
        let order = purchase_order::Entity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态 - 只有待审批状态的订单才能拒绝
        if order.order_status != status::purchase_order::PENDING_APPROVAL {
            return Err(AppError::business(format!(
                "订单状态不允许拒绝，当前状态：{}，需要状态：PENDING_APPROVAL",
                order.order_status
            )));
        }

        // 3. 更新状态（走 update_with_audit 保留审计追溯，使用真实 user_id）
        let now = chrono::Utc::now();
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set(status::purchase_order::REJECTED.to_string());
        order_active.rejected_reason = Set(Some(reason));
        order_active.updated_by = Set(Some(user_id));
        order_active.updated_at = Set(now);

        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(order)
    }
}
