//! 任务审批流子模块（bpm_ops/task）
//!
//! 从原 `bpm_service.rs` 迁移 11 个方法 + `ApproveContext` 辅助结构：
//! - `approve_task`：审批任务（approve/reject 主入口，事务内收集事件，commit 后发布）
//! - `load_approve_context`：加载审批上下文（行锁 + 校验 pending + 加载 instance/definition）
//! - `update_task_status`：更新任务状态 + 审计
//! - `handle_task_reject`：拒绝任务（终止 instance + 收集事件）
//! - `handle_task_approve`：批准任务（推进下一节点或完成 instance）
//! - `try_advance_to_next_node`：尝试推进下一节点（条件评估 + 创建下一 user_task）
//! - `create_next_user_task`：创建下一 user_task 节点
//! - `complete_process_instance`：完成 instance + 收集事件
//! - `query_user_tasks`：分页查询用户任务
//! - `transfer_task`：转办任务
//! - `urge_task`：催办任务（发送通知）
//!
//! 跨 facade 依赖：
//! - `try_advance_to_next_node` 调用 facade 的 `evaluate_bpm_condition`（`pub(crate)`）

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect, Set,
    TransactionTrait,
};

use crate::models::dto::bpm_dto::{ApproveTaskRequest, TaskQuery};
use crate::models::dto::PageResponse;
use crate::models::status::bpm_instance as instance_status;
use crate::models::status::bpm_task as task_status;
use crate::models::{bpm_process_definition, bpm_process_instance, bpm_task};
use crate::services::bpm_service::{evaluate_bpm_condition, BpmService};
use crate::utils::error::AppError;

/// approve_task 上下文：封装 task/instance/definition
struct ApproveContext {
    task: bpm_task::Model,
    instance: bpm_process_instance::Model,
    definition: bpm_process_definition::Model,
}

impl BpmService {
    pub async fn approve_task(
        &self,
        req: ApproveTaskRequest,
        user_id: Option<i32>,
    ) -> Result<(), AppError> {
        let txn = self.db.begin().await?;
        // P0 5-3 修复：事务内仅收集待发事件，commit 成功后再 publish，避免 commit 失败产生幻事件
        let mut pending_event: Option<crate::services::event_bus::BusinessEvent> = None;

        let ctx = self.load_approve_context(&req, &txn).await?;
        self.update_task_status(&req, &ctx.task, user_id, &txn)
            .await?;

        if req.action == "reject" {
            self.handle_task_reject(&ctx.instance, user_id, &txn, &mut pending_event)
                .await?;
        } else {
            self.handle_task_approve(&ctx, user_id, &txn, &mut pending_event)
                .await?;
        }

        txn.commit().await?;

        // P0 5-3 修复：commit 成功后发布 BPM 流程结束事件
        if let Some(ev) = pending_event {
            crate::services::event_bus::EVENT_BUS.publish(ev);
        }
        Ok(())
    }

    /// 加载审批上下文：锁定 task + 校验 pending + 加载 instance/definition
    async fn load_approve_context(
        &self,
        req: &ApproveTaskRequest,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<ApproveContext, AppError> {
        // P1 3-5 修复（批次 61）：task 查询加 lock_exclusive，串行化并发审批同一任务
        // 原实现仅 txn 无行锁，两并发 approve_task 同时读到 pending 状态，竞态后双写。
        let task = bpm_task::Entity::find_by_id(req.task_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found("Task not found"))?;

        if task.status.as_deref() != Some(task_status::PENDING) {
            return Err(AppError::validation("Task is not pending"));
        }

        let process_instance_id = task.instance_id;
        let instance = bpm_process_instance::Entity::find_by_id(process_instance_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found("Process instance not found"))?;

        let definition = bpm_process_definition::Entity::find_by_id(instance.process_definition_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found("Definition not found"))?;

        Ok(ApproveContext {
            task,
            instance,
            definition,
        })
    }

    /// 更新当前任务状态（COMPLETED/REJECTED）+ 审计
    async fn update_task_status(
        &self,
        req: &ApproveTaskRequest,
        task: &bpm_task::Model,
        user_id: Option<i32>,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let mut task_active: bpm_task::ActiveModel = task.clone().into();
        task_active.status = Set(Some(if req.action == "approve" {
            task_status::COMPLETED.to_string()
        } else {
            task_status::REJECTED.to_string()
        }));
        task_active.actual_handler_id = Set(Some(req.handler_id));
        task_active.approval_opinion = Set(req.approval_opinion.clone());
        task_active.handled_at = Set(Some(chrono::Utc::now()));
        task_active.updated_at = Set(Some(chrono::Utc::now()));
        // P0 8-4 修复：task 状态变更纳入审计（update_with_audit 在事务内同步写审计日志，
        // 记录真实操作者 user_id，而非前端传入的 handler_id，防止代审追溯丢失）
        // P2-3 修复（批次 84 v1 复审）：有意忽略返回的 ActiveModel（字段已通过 Set 表达更新意图），仅传播错误
        // 批次 94 P2-11：审计日志为关键路径，错误已通过 ? 传播；去掉 let _ = 直接丢弃 ActiveModel 返回值
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "bpm_task",
            task_active,
            user_id,
        )
        .await?;
        Ok(())
    }

    /// 拒绝任务：终止 instance + 收集 BpmProcessFinished 事件
    async fn handle_task_reject(
        &self,
        instance: &bpm_process_instance::Model,
        user_id: Option<i32>,
        txn: &sea_orm::DatabaseTransaction,
        pending_event: &mut Option<crate::services::event_bus::BusinessEvent>,
    ) -> Result<(), AppError> {
        // End instance if rejected
        let mut instance_active: bpm_process_instance::ActiveModel = instance.clone().into();
        instance_active.status = Set(Some(instance_status::TERMINATED.to_string()));
        instance_active.completed_at = Set(Some(chrono::Utc::now()));
        instance_active.updated_at = Set(Some(chrono::Utc::now()));
        // P0 8-4 修复：instance 终止状态变更纳入审计
        // P2-3 修复（批次 84 v1 复审）：有意忽略返回的 ActiveModel（字段已通过 Set 表达更新意图），仅传播错误
        // 批次 94 P2-11：审计日志为关键路径，错误已通过 ? 传播；去掉 let _ = 直接丢弃 ActiveModel 返回值
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "bpm_process_instance",
            instance_active,
            user_id,
        )
        .await?;

        if let (Some(b_type), Some(b_id)) = (
            Some(instance.business_type.clone()),
            Some(instance.business_id),
        ) {
            // P0 5-3 修复：事务内仅收集事件，commit 后再 publish
            // P2 5-18 修复：携带 approver_id（拒绝操作的实际审批人）
            *pending_event = Some(
                crate::services::event_bus::BusinessEvent::BpmProcessFinished {
                    business_type: b_type,
                    business_id: b_id,
                    approved: false,
                    approver_id: user_id.unwrap_or(0),
                },
            );
        }
        Ok(())
    }

    /// 批准任务：尝试推进下一节点，否则完成 instance
    async fn handle_task_approve(
        &self,
        ctx: &ApproveContext,
        user_id: Option<i32>,
        txn: &sea_orm::DatabaseTransaction,
        pending_event: &mut Option<crate::services::event_bus::BusinessEvent>,
    ) -> Result<(), AppError> {
        let advanced = self.try_advance_to_next_node(ctx, txn).await?;
        if !advanced {
            self.complete_process_instance(&ctx.instance, user_id, txn, pending_event)
                .await?;
        }
        Ok(())
    }

    /// 尝试推进下一节点：查找匹配边 + 创建下一 user_task
    async fn try_advance_to_next_node(
        &self,
        ctx: &ApproveContext,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<bool, AppError> {
        // Approve -> Find next node
        if let Some(flow_def) = &ctx.definition.config {
            if let (Some(nodes), Some(edges)) = (
                flow_def.get("nodes").and_then(|n| n.as_array()),
                flow_def.get("edges").and_then(|e| e.as_array()),
            ) {
                // 查找从当前任务节点出发的边，支持条件评估
                let matching_edge = edges.iter().find(|e| {
                    let source_match =
                        e.get("source").and_then(|s| s.as_str()) == Some(&ctx.task.node_id);
                    if !source_match {
                        return false;
                    }

                    // 检查边条件
                    if let Some(condition) = e.get("condition").and_then(|c| c.as_str()) {
                        evaluate_bpm_condition(condition, &ctx.instance.variables)
                    } else {
                        true // 无条件默认匹配
                    }
                });

                if let Some(edge) = matching_edge {
                    let target_id = edge.get("target").and_then(|t| t.as_str()).unwrap_or("");
                    let target_node = nodes
                        .iter()
                        .find(|n| n.get("id").and_then(|i| i.as_str()) == Some(target_id));

                    if let Some(next_node) = target_node {
                        let node_type =
                            next_node.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        if node_type == "user_task" {
                            self.create_next_user_task(ctx, next_node, txn).await?;
                            return Ok(true);
                        } else if node_type == "end_event" {
                            // 结束事件，在下面处理
                        }
                    }
                }
            }
        }
        Ok(false)
    }

    /// 创建下一 user_task 节点（生成 TSK 单号 + 初始化为 PENDING）
    async fn create_next_user_task(
        &self,
        ctx: &ApproveContext,
        next_node: &serde_json::Value,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let new_task = bpm_task::ActiveModel {
            instance_id: Set(ctx.instance.id),
            process_definition_id: Set(ctx.definition.id),
            // P1 3-6 修复（批次 60）：改用 DocumentNumberGenerator 保证并发唯一性
            task_no: Set(
                crate::utils::number_generator::DocumentNumberGenerator::generate_no_with_txn(
                    txn,
                    "TSK",
                    bpm_task::Entity,
                    bpm_task::Column::TaskNo,
                )
                .await?
            ),
            node_id: Set(next_node
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or("unknown")
                .to_string()),
            node_name: Set(next_node
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("Task")
                .to_string()),
            node_type: Set("user_task".to_string()),
            task_type: Set(Some("user_task".to_string())),
            actual_handler_id: Set(next_node
                .get("assignee_value")
                .and_then(|a| a.as_str())
                .and_then(|s| s.parse::<i32>().ok())),
            status: Set(Some(task_status::PENDING.to_string())),
            created_at: Set(Some(chrono::Utc::now())),
            updated_at: Set(Some(chrono::Utc::now())),
            ..Default::default()
        };
        new_task.insert(txn).await?;
        Ok(())
    }

    /// 完成 instance + 收集 BpmProcessFinished 事件
    async fn complete_process_instance(
        &self,
        instance: &bpm_process_instance::Model,
        user_id: Option<i32>,
        txn: &sea_orm::DatabaseTransaction,
        pending_event: &mut Option<crate::services::event_bus::BusinessEvent>,
    ) -> Result<(), AppError> {
        // No more user tasks, instance is completed
        let mut instance_active: bpm_process_instance::ActiveModel = instance.clone().into();
        instance_active.status = Set(Some(instance_status::COMPLETED.to_string()));
        instance_active.completed_at = Set(Some(chrono::Utc::now()));
        // P0 8-4 修复：instance 完成状态变更纳入审计
        // P2-3 修复（批次 84 v1 复审）：有意忽略返回的 ActiveModel（字段已通过 Set 表达更新意图），仅传播错误
        // 批次 94 P2-11：审计日志为关键路径，错误已通过 ? 传播；去掉 let _ = 直接丢弃 ActiveModel 返回值
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "bpm_process_instance",
            instance_active,
            user_id,
        )
        .await?;

        if let (Some(b_type), Some(b_id)) = (
            Some(instance.business_type.clone()),
            Some(instance.business_id),
        ) {
            // P0 5-3 修复：事务内仅收集事件，commit 后再 publish
            // P2 5-18 修复：携带 approver_id（最后节点审批通过的实际审批人）
            *pending_event = Some(
                crate::services::event_bus::BusinessEvent::BpmProcessFinished {
                    business_type: b_type,
                    business_id: b_id,
                    approved: true,
                    approver_id: user_id.unwrap_or(0),
                },
            );
        }
        Ok(())
    }

    pub async fn query_user_tasks(
        &self,
        query: TaskQuery,
    ) -> Result<PageResponse<bpm_task::Model>, AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(10).clamp(1, 100); // v10 P1-1 修复：page_size clamp(1,100) 防 DoS

        let mut stmt = bpm_task::Entity::find();

        if let Some(user_id) = query.user_id {
            stmt = stmt.filter(bpm_task::Column::ActualHandlerId.eq(user_id));
        }

        if let Some(status) = query.status {
            stmt = stmt.filter(bpm_task::Column::Status.eq(status));
        }

        let paginator = stmt.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;

        let total_pages = if total == 0 {
            0
        } else {
            total.div_ceil(page_size)
        };
        Ok(PageResponse {
            data: items,
            total,
            page,
            page_size,
            total_pages,
        })
    }

    /// 转办任务
    pub async fn transfer_task(
        &self,
        task_id: i32,
        new_assignee_id: i32,
        transfer_reason: &str,
    ) -> Result<(), AppError> {
        let txn = self.db.begin().await?;

        let task = bpm_task::Entity::find_by_id(task_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("任务不存在"))?;

        if task.status.as_deref() != Some(task_status::PENDING) {
            return Err(AppError::validation("只能转办待处理任务"));
        }

        let mut task_active: bpm_task::ActiveModel = task.into();
        task_active.actual_handler_id = Set(Some(new_assignee_id));
        task_active.approval_opinion = Set(Some(format!("[转办] {}", transfer_reason)));
        task_active.updated_at = Set(Some(chrono::Utc::now()));
        task_active.update(&txn).await?;

        txn.commit().await?;
        Ok(())
    }

    /// 催办任务
    pub async fn urge_task(&self, task_id: i32, urge_message: &str) -> Result<(), AppError> {
        let task = bpm_task::Entity::find_by_id(task_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("任务不存在"))?;

        if task.status.as_deref() != Some(task_status::PENDING) {
            return Err(AppError::validation("只能催办待处理任务"));
        }

        // 记录催办日志，可以通过事件总线发送通知
        tracing::info!(
            "催办任务 {}: {}, 处理人: {:?}, 消息: {}",
            task_id,
            task.task_no,
            task.actual_handler_id,
            urge_message
        );

        // 发送催办通知给处理人
        if let Some(assignee_id) = task.actual_handler_id {
            let notification_service =
                crate::services::notification_service::NotificationService::new(self.db.clone());
            // P1-4c 修复（批次 80 v1 复审）：原 let _ = 静默吞错，
            // 通知创建失败时无任何日志，催办通知丢失难以排查。
            // 改为 if let Err(e) = ... { tracing::warn!(...); }
            if let Err(e) = notification_service
                .create_notification(
                    crate::services::notification_service::CreateNotificationRequest {
                        user_id: assignee_id,
                        notification_type: crate::models::notification::NotificationType::Internal,
                        title: "任务催办".to_string(),
                        content: format!(
                            "任务 '{}' 需要您尽快处理。催办消息: {}",
                            task.task_no, urge_message
                        ),
                        priority: crate::models::notification::NotificationPriority::High,
                        business_type: Some("BPM".to_string()),
                        business_id: Some(task.instance_id),
                        action_url: Some(format!("/bpm/tasks/{}", task_id)),
                        sender_id: None,
                        sender_name: Some("系统".to_string()),
                    },
                )
                .await
            {
                tracing::warn!(
                    task_id = task_id,
                    assignee_id = assignee_id,
                    error = %e,
                    "BPM 任务催办通知发送失败（best-effort，不影响主业务流）"
                );
            }
        }

        Ok(())
    }
}
