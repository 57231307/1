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
// 批次 358 v13 复审 B-P1-5 修复：导入 BusinessEvent 和 EVENT_BUS，
// 在 approve_order commit 成功后发布 PurchaseOrderApproved 事件，
// 触发库存财务桥接等下游订阅方生成采购入库相关凭证
use crate::services::event_bus::{BusinessEvent, EVENT_BUS};

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

        // 批次 358 v13 复审 B-P1-5 修复：commit 成功后发布 PurchaseOrderApproved 事件
        // 原实现仅更新订单状态，未通知下游订阅方（库存财务桥接、BPM 流程等），
        // 导致采购审批 → 入库 → 凭证生成的业务闭环断裂。
        // 事件在 commit 后发布，避免事务回滚时已发布事件造成的幻事件。
        EVENT_BUS.publish(BusinessEvent::PurchaseOrderApproved {
            order_id: order.id,
            supplier_id: order.supplier_id,
        });

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

    /// 取消采购订单
    ///
    /// 批次 215 P2-1 修复（v12 复审）：实现采购订单 cancel_order 功能，
    /// 移除 purchase_order::CANCELLED 的 #[allow(dead_code)] 标注。
    ///
    /// 业务规则：
    /// - 允许取消状态：DRAFT / PENDING_APPROVAL / APPROVED / PARTIAL_RECEIVED
    ///   （已收货部分通过采购退货流程处理，取消仅作用于未收货部分）
    /// - 禁止取消状态：REJECTED（终态）/ CLOSED（终态）/ COMPLETED（终态）/ CANCELLED（终态）
    /// - 取消时释放已占用的预算（若创建时预算占用成功，插入反向冲销记录）
    /// - 取消原因记录到 rejected_reason 字段（语义扩展为"拒绝/取消原因"，避免新增字段）
    pub async fn cancel_order(
        &self,
        order_id: i32,
        reason: String,
        user_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询订单（加 lock_exclusive 串行化并发取消）
        let order = purchase_order::Entity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态 - 只有未完成终态的订单才能取消
        if ![
            status::purchase_order::DRAFT,
            status::purchase_order::PENDING_APPROVAL,
            status::purchase_order::APPROVED,
            status::purchase_order::PARTIAL_RECEIVED,
        ]
        .contains(&order.order_status.as_str())
        {
            return Err(AppError::business(format!(
                "订单状态不允许取消，当前状态：{}，允许取消状态：DRAFT/PENDING_APPROVAL/APPROVED/PARTIAL_RECEIVED",
                order.order_status
            )));
        }

        // 3. 释放已占用的预算（非阻断，失败仅 warn 不阻断取消主流程）
        //    查询 budget_execution 表中该订单的"使用"类型记录，若存在则插入反向"调整"冲销
        self.release_budget_occupation(&order, &txn).await;

        // 4. 更新状态为 CANCELLED，记录取消原因
        let now = chrono::Utc::now();
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set(status::purchase_order::CANCELLED.to_string());
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

    /// 释放采购订单已占用的预算（内部辅助方法，事务内原子操作）
    ///
    /// 查询 budget_execution 表中 related_document_type='purchase_order'
    /// 且 related_document_id=order_id 且 execution_type='使用' 的记录，
    /// 若存在则插入反向"调整"冲销记录（金额为负），抵消原占用。
    async fn release_budget_occupation(
        &self,
        order: &purchase_order::Model,
        txn: &sea_orm::DatabaseTransaction,
    ) {
        use crate::models::budget_execution;
        use sea_orm::ColumnTrait;
        use sea_orm::EntityTrait;
        use sea_orm::QueryFilter;

        // 查询该订单的预算占用记录
        let occupations = budget_execution::Entity::find()
            .filter(budget_execution::Column::RelatedDocumentType.eq("purchase_order"))
            .filter(budget_execution::Column::RelatedDocumentId.eq(order.id))
            .filter(budget_execution::Column::ExecutionType.eq("使用"))
            .all(txn)
            .await;

        let occupations = match occupations {
            Ok(records) => records,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    order_id = order.id,
                    "查询采购订单预算占用记录失败，跳过预算释放（不阻断取消主流程）"
                );
                return;
            }
        };

        if occupations.is_empty() {
            tracing::info!(
                order_id = order.id,
                "采购订单无预算占用记录，无需释放预算"
            );
            return;
        }

        // 在同一事务内插入反向"调整"冲销记录，保证原子性
        let now = chrono::Utc::now();
        for occupation in &occupations {
            let release_active = budget_execution::ActiveModel {
                plan_id: Set(occupation.plan_id),
                execution_type: Set("调整".to_string()),
                amount: Set(-occupation.amount),
                expense_type: Set(Some("采购订单取消释放".to_string())),
                expense_date: Set(now.date_naive()),
                related_document_type: Set(Some("purchase_order".to_string())),
                related_document_id: Set(Some(order.id)),
                remark: Set(Some(format!(
                    "采购订单取消冲销预算占用，订单 {}，原执行ID: {}",
                    order.order_no, occupation.id
                ))),
                created_by: Set(Some(order.created_by)),
                ..Default::default()
            };

            match budget_execution::Entity::insert(release_active)
                .exec(txn)
                .await
            {
                Ok(insert_result) => {
                    tracing::info!(
                        order_id = order.id,
                        original_execution_id = occupation.id,
                        release_execution_id = insert_result.last_insert_id,
                        amount = %occupation.amount,
                        "采购订单预算释放成功"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        order_id = order.id,
                        original_execution_id = occupation.id,
                        "采购订单预算释放记录插入失败（不阻断取消主流程）"
                    );
                }
            }
        }
    }
}
