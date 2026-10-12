//! 生产订单-审批子模块（production_order_ops/approval）
//!
//! 批次 488 D10-2 拆分：从原 `production_order_service.rs` L1250-1501 迁移。
//! 包含 7 个审批相关方法：
//! - submit_for_approval（公开 API，提交审批 + 启动 BPM 流程）
//! - approve_order（公开 API，审批通过/拒绝 + 回调 BPM 任务）
//! - lock_and_validate_order_for_approval_txn（私有 &self，行锁查询 + 状态校验）
//! - build_approval_active_model（私有 associated fn，构造审批状态 ActiveModel）
//! - handle_bpm_approval_after_commit（私有 &self，commit 后完成 BPM 任务）
//! - approve_order_via_bpm（公开 API，BPM 回写专用审批通过，不回调 BPM 避免循环）
//! - reject_order_via_bpm（公开 API，BPM 回写专用审批拒绝，不回调 BPM 避免循环）
//!
//! 业务规则：
//! - 事务包裹"查询 + 状态校验 + update"，加 lock_exclusive 防止并发审批同一订单
//! - BPM 启动/任务审批保留事务外（失败 warn 不阻断已提交状态），避免 BPM 调用持有数据库锁
//! - approve_order_via_bpm / reject_order_via_bpm 不回调 BPM（避免 BPM → 事件 → approve_order → BPM 死循环）
//! - BPM 回写方法走 update_with_audit 保留审计追溯
//! - validate_status_transition 跨 impl 块调用（定义在 crud 子模块，pub(crate) 可见性）

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};

use crate::models::bpm_task;
use crate::models::production_order::{
    ActiveModel, Entity as ProductionOrderEntity, Model as ProductionOrderModel,
};
use crate::models::status::bpm_task as task_status;
use crate::utils::error::AppError;

use crate::services::bpm_ops::task::{APPROVE_ACTION, REJECT_ACTION};
use crate::services::production_order_service::ProductionOrderService;

impl ProductionOrderService {
    /// 提交生产订单审批
    /// 批次 15（2026-06-28）：事务包裹"查询 + 状态校验 + update"，；加 lock_exclusive 防止并发提交同一订单导致状态不一致；BPM 启动保留事务外（失败 warn 不阻断已提交状态），避免 BPM 调用持有数据库锁。
    pub async fn submit_for_approval(
        &self,
        id: i32,
        user_id: i32,
        user_name: &str,
    ) -> Result<ProductionOrderModel, AppError> {
        let txn = (*self.db).begin().await?;

        let model = ProductionOrderEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("生产订单不存在"))?;

        // 验证状态转换是否合法
        Self::validate_status_transition(
            &model.status,
            crate::models::status::production::PRODUCTION_PENDING_APPROVAL,
        )?;

        // 更新状态为审批中
        let mut active_model: ActiveModel = model.into();
        active_model.status =
            Set(crate::models::status::production::PRODUCTION_PENDING_APPROVAL.to_string());
        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&txn).await?;

        txn.commit().await?;

        // 启动BPM审批流程（事务外，失败不阻断已提交状态）
        let bpm_service = crate::services::bpm_service::BpmService::new(self.db.clone());
        let req = crate::models::dto::bpm_dto::StartProcessRequest {
            process_key: "production_order_approval".to_string(),
            business_type: "production_order".to_string(),
            business_id: i64::from(id),
            title: format!("生产订单审批 - {}", updated.order_no),
            initiator_id: user_id,
            initiator_name: user_name.to_string(),
            initiator_department_id: None,
            priority: Some("HIGH".to_string()),
            form_data: Some(serde_json::json!({
                "order_no": updated.order_no,
                "product_id": updated.product_id,
                "planned_quantity": updated.planned_quantity,
                "work_center_id": updated.work_center_id,
            })),
            variables: None,
        };

        // P0 修复（批次 4，2026-06-27）：原 `let _ = ...` 静默吞掉 BPM 启动错误，
        // 导致模板缺失/DB 异常时无任何日志可追溯。改为 warn 日志记录，保留兼容性
        // （不向上传播错误，避免阻断主流程），但确保运维可观测。
        if let Err(e) = bpm_service.start_process(req).await {
            tracing::warn!(
                error = %e,
                order_id = id,
                "BPM 启动生产订单审批流程失败（兼容旧数据，不阻断主流程）"
            );
        }

        Ok(updated)
    }

    /// 审批生产订单
    /// 批次 15（2026-06-28）：事务包裹"查询 + 状态校验 + update"，；加 lock_exclusive 防止并发审批同一订单导致重复审批或状态覆盖；BPM 任务审批保留事务外（失败 warn 不阻断已提交状态），避免 BPM 调用持有数据库锁。
    pub async fn approve_order(
        &self,
        id: i32,
        user_id: i32,
        user_name: &str,
        approved: bool,
        opinion: Option<String>,
    ) -> Result<ProductionOrderModel, AppError> {
        let txn = (*self.db).begin().await?;

        let model = self
            .lock_and_validate_order_for_approval_txn(id, approved, &txn)
            .await?;
        let active_model = Self::build_approval_active_model(model, approved);
        let updated = active_model.update(&txn).await?;
        txn.commit().await?;

        // BPM 任务审批在事务外执行，失败仅 warn 不阻断已提交状态
        self.handle_bpm_approval_after_commit(id, user_id, user_name, approved, opinion)
            .await;

        Ok(updated)
    }

    /// 行锁查询订单并校验审批状态转换合法性
    async fn lock_and_validate_order_for_approval_txn(
        &self,
        id: i32,
        approved: bool,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<ProductionOrderModel, AppError> {
        let model = ProductionOrderEntity::find_by_id(id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found("生产订单不存在"))?;
        let new_status = if approved {
            crate::models::status::common::STATUS_APPROVED
        } else {
            crate::models::status::production::PRODUCTION_REJECTED
        };
        Self::validate_status_transition(&model.status, new_status)?;
        Ok(model)
    }

    /// 构造审批状态 ActiveModel（写入新状态与更新时间）
    fn build_approval_active_model(model: ProductionOrderModel, approved: bool) -> ActiveModel {
        let new_status = if approved {
            crate::models::status::common::STATUS_APPROVED
        } else {
            crate::models::status::production::PRODUCTION_REJECTED
        };
        let mut active_model: ActiveModel = model.into();
        active_model.status = Set(new_status.to_string());
        active_model.updated_at = Set(Utc::now());
        active_model
    }

    /// 事务提交后完成 BPM 任务（容错：失败只记日志，不回改业务侧已提交的裁决结果）
    ///
    /// 定位口径：先按 business_type+business_id 定位流程实例，再按
    /// `instance_id + 写入方同源常量 bpm_task::PENDING` 定位**本工单**的待办任务，
    /// 只回写这些任务。旧形态用 `common::STATUS_PENDING`（大写 `"PENDING"`，属
    /// 其他单据域词表）过滤任务状态，与写入方小写值（bpm_ops 写 `bpm_task::PENDING=
    /// "pending"`，DB CHECK 钉小写集）永不相等 ⇒ 回写循环恒空转，`opinion` 既不进
    /// 业务表（无列）也不进 `bpm_task`，理由彻底丢失；办理人过滤+分页上限 10 还会
    /// 在待办较多时漏查本工单任务，故一并弃用。
    async fn handle_bpm_approval_after_commit(
        &self,
        id: i32,
        user_id: i32,
        user_name: &str,
        approved: bool,
        opinion: Option<String>,
    ) {
        let bpm_service = crate::services::bpm_service::BpmService::new(self.db.clone());
        let action = if approved {
            APPROVE_ACTION.to_string()
        } else {
            REJECT_ACTION.to_string()
        };
        let instance = match bpm_service
            .get_process_by_business("production_order", i64::from(id))
            .await
        {
            Ok(Some(instance)) => instance,
            Ok(None) => {
                tracing::warn!(
                    business_type = "production_order",
                    business_id = id,
                    expected_action = %action,
                    "生产工单审批 BPM 回写：未找到该工单的流程实例，待办任务无法闭环，需人工对账"
                );
                return;
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    business_type = "production_order",
                    business_id = id,
                    "生产工单审批 BPM 回写：查询流程实例失败，任务未闭环"
                );
                return;
            }
        };
        let tasks = match bpm_task::Entity::find()
            .filter(bpm_task::Column::InstanceId.eq(instance.id))
            .filter(bpm_task::Column::Status.eq(task_status::PENDING))
            .all(&*self.db)
            .await
        {
            Ok(tasks) => tasks,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    business_type = "production_order",
                    business_id = id,
                    instance_id = instance.id,
                    "生产工单审批 BPM 回写：查询待办任务失败，任务未闭环"
                );
                return;
            }
        };
        if tasks.is_empty() {
            tracing::warn!(
                business_type = "production_order",
                business_id = id,
                instance_id = instance.id,
                expected_task_status = task_status::PENDING,
                "生产工单审批 BPM 回写：该实例下无状态为待处理的 bpm_task，裁决与理由未进 BPM，需人工对账"
            );
            return;
        }
        for task in tasks {
            // approve_task 的处理人取已认证操作人（user_id 参数，见 bpm_ops/task.rs），
            // req.handler_id/handler_name 在当前签名下不被消费，仅按 DTO 形状填真实入参。
            if let Err(e) = bpm_service
                .approve_task(
                    crate::models::dto::bpm_dto::ApproveTaskRequest {
                        task_id: task.id,
                        handler_id: user_id,
                        handler_name: user_name.to_string(),
                        action: action.clone(),
                        approval_opinion: opinion.clone(),
                        attachment_urls: None,
                    },
                    Some(user_id),
                )
                .await
            {
                tracing::warn!(
                    error = %e,
                    task_id = task.id,
                    business_type = "production_order",
                    business_id = id,
                    "BPM 生产订单任务审批失败（不阻断已提交的业务状态）"
                );
            } else {
                tracing::info!(
                    task_id = task.id,
                    business_id = id,
                    approved,
                    "生产工单审批 BPM 回写完成"
                );
            }
        }
    }

    /// B-P1-9 修复（批次 360 v13 复审）：BPM 回写专用审批通过方法
    /// 与 `approve_order` 的区别：不回调 BPM（避免 BPM → 事件 → approve_order → BPM 死循环）。；仅更新生产订单状态 PENDING_APPROVAL → APPROVED，走 update_with_audit 保留审计追溯。；由 event_bus.rs 的 BpmProcessFinished 事件监听器调用。
    pub async fn approve_order_via_bpm(
        &self,
        order_id: i32,
        approver_id: i32,
    ) -> Result<ProductionOrderModel, AppError> {
        let txn = (*self.db).begin().await?;

        let model = ProductionOrderEntity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("生产订单不存在"))?;

        let new_status = crate::models::status::common::STATUS_APPROVED;
        Self::validate_status_transition(&model.status, new_status)?;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set(new_status.to_string());
        active_model.updated_at = Set(Utc::now());

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            super::types::AUDIT_RESOURCE_TYPE,
            active_model,
            Some(approver_id),
        )
        .await?;

        txn.commit().await?;
        Ok(updated)
    }

    /// B-P1-9 修复（批次 360 v13 复审）：BPM 回写专用审批拒绝方法
    /// 与 `approve_order(approved=false)` 的区别：不回调 BPM（避免循环）。；仅更新生产订单状态 PENDING_APPROVAL → REJECTED，走 update_with_audit 保留审计追溯。
    pub async fn reject_order_via_bpm(
        &self,
        order_id: i32,
        reason: String,
        approver_id: i32,
    ) -> Result<ProductionOrderModel, AppError> {
        let txn = (*self.db).begin().await?;

        let model = ProductionOrderEntity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("生产订单不存在"))?;

        let new_status = crate::models::status::production::PRODUCTION_REJECTED;
        Self::validate_status_transition(&model.status, new_status)?;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set(new_status.to_string());
        active_model.updated_at = Set(Utc::now());

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            super::types::AUDIT_RESOURCE_TYPE,
            active_model,
            Some(approver_id),
        )
        .await?;

        txn.commit().await?;
        tracing::info!(
            order_id = order_id,
            approver_id = approver_id,
            reason = %reason,
            "生产订单 BPM 审批拒绝回写完成"
        );
        Ok(updated)
    }
}
