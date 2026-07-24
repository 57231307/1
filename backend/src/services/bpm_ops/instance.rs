//! 流程实例生命周期子模块（bpm_ops/instance）
//!
//! 从原 `bpm_service.rs` 迁移 7 个方法：
//! - `start_process`：发起流程（创建实例 + 首任务/自动完成 + 事务提交后发布事件）
//! - `create_first_task_or_complete`：创建首个任务或自动完成流程（私有，仅 start_process 调用）
//! - `cancel_instance`：撤回流程实例（终止实例 + 取消待处理任务 + 发布拒绝事件）
//! - `get_process_by_business`：按业务类型 + 业务 ID 查询流程实例
//! - `get_instance_detail`：获取流程实例详情（含审批链）
//! - `get_approval_chain`：获取流程实例审批链（pub，被 get_instance_detail 调用）
//! - `get_business_relation`：获取 BPM 业务关联信息
//!
//! 跨 facade 依赖：
//! - `create_first_task_or_complete` 调用 facade 的 `BpmService::resolve_first_task_node`（`pub(crate)`）

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait,
};

use crate::models::dto::bpm_dto::{StartProcessRequest, StartProcessResponse};

use crate::models::status::bpm_instance as instance_status;
use crate::models::status::bpm_task as task_status;
use crate::models::{bpm_process_definition, bpm_process_instance, bpm_task};
use crate::services::bpm_service::BpmService;
use crate::services::bpm_service_dto::{
    ApprovalChainNode, BpmBusinessRelation, ProcessInstanceDetail,
};
use crate::utils::error::AppError;

impl BpmService {
    /// 创建首个任务或自动完成流程
    async fn create_first_task_or_complete(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        definition: &bpm_process_definition::Model,
        instance: &bpm_process_instance::Model,
        req: &StartProcessRequest,
    ) -> Result<(Vec<i32>, Option<crate::services::event_bus::BusinessEvent>), AppError> {
        let mut task_ids = vec![];
        let mut pending_event: Option<crate::services::event_bus::BusinessEvent> = None;

        if let Some(flow_def) = &definition.config {
            let first_task_node = Self::resolve_first_task_node(flow_def);

            if let Some(first_task) = first_task_node {
                // 创建首个用户任务
                let task_model = bpm_task::ActiveModel {
                    instance_id: Set(instance.id),
                    process_definition_id: Set(definition.id),
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
                    node_id: Set(first_task
                        .get("id")
                        .and_then(|i| i.as_str())
                        .unwrap_or("unknown")
                        .to_string()),
                    node_name: Set(first_task
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("Task")
                        .to_string()),
                    node_type: Set("user_task".to_string()),
                    task_type: Set(Some("user_task".to_string())),
                    actual_handler_id: Set(first_task
                        .get("assignee_value")
                        .and_then(|a| a.as_str())
                        .and_then(|s| s.parse::<i32>().ok())),
                    status: Set(Some(task_status::PENDING.to_string())),
                    created_at: Set(Some(chrono::Utc::now())),
                    updated_at: Set(Some(chrono::Utc::now())),
                    ..Default::default()
                };
                let task = task_model.insert(txn).await?;
                task_ids.push(task.id);
            } else {
                // 无任务节点，自动完成流程
                let mut instance_active: bpm_process_instance::ActiveModel =
                    instance.clone().into();
                instance_active.status = Set(Some(instance_status::COMPLETED.to_string()));
                instance_active.completed_at = Set(Some(chrono::Utc::now()));
                instance_active.update(txn).await?;

                // P0 5-3 修复：事务内仅收集事件，commit 后再 publish
                // P2 5-18 修复：携带 initiator_id 作为 approver_id（start_process 自动完成时审批人=发起人）
                pending_event = Some(
                    crate::services::event_bus::BusinessEvent::BpmProcessFinished {
                        business_type: req.business_type.clone(),
                        business_id: req.business_id,
                        approved: true,
                        approver_id: req.initiator_id,
                    },
                );
            }
        }

        Ok((task_ids, pending_event))
    }

    pub async fn start_process(
        &self,
        req: StartProcessRequest,
    ) -> Result<StartProcessResponse, AppError> {
        let txn = self.db.begin().await?;

        let definition = bpm_process_definition::Entity::find()
            .filter(bpm_process_definition::Column::Code.eq(&req.process_key))
            .filter(bpm_process_definition::Column::Status.eq("ACTIVE"))
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("Process definition not found or inactive"))?;

        // P1 3-6 修复（批次 60）：改用 DocumentNumberGenerator 保证并发唯一性
        // 原实现基于时间戳 + business_id，同秒并发 + 同 business_id 会产生重复单号
        let instance_no = crate::utils::number_generator::DocumentNumberGenerator::generate_no_with_txn(
            &txn,
            "BPM",
            bpm_process_instance::Entity,
            bpm_process_instance::Column::InstanceNo,
        )
        .await?;
        let instance_model = bpm_process_instance::ActiveModel {
            process_definition_id: Set(definition.id),
            instance_no: Set(instance_no.clone()),
            business_type: Set(req.business_type.clone()),
            business_id: Set(req.business_id),
            title: Set(format!("流程审批-{}", req.business_id)),
            initiator_id: Set(req.initiator_id),
            initiator_name: Set("".to_string()),
            status: Set(Some(instance_status::PROCESSING.to_string())),
            variables: Set(req.variables.clone()),
            started_at: Set(Some(chrono::Utc::now())),
            ..Default::default()
        };

        let instance = instance_model.insert(&txn).await?;

        // 创建首个任务或自动完成流程
        let (task_ids, pending_event) = self
            .create_first_task_or_complete(&txn, &definition, &instance, &req)
            .await?;

        txn.commit().await?;

        // P0 5-3 修复：commit 成功后发布 BPM 流程结束事件
        if let Some(ev) = pending_event {
            crate::services::event_bus::EVENT_BUS.publish(ev);
        }

        Ok(StartProcessResponse {
            instance_id: instance.id,
            instance_no,
            task_ids,
        })
    }

    /// 撤回流程实例（批次 157d-3 新增）：终止实例并取消所有待处理任务
    pub async fn cancel_instance(
        &self,
        instance_id: i32,
        user_id: Option<i32>,
        cancel_reason: Option<String>,
    ) -> Result<(), AppError> {
        let txn = self.db.begin().await?;

        // 加行锁串行化并发撤回
        let instance = bpm_process_instance::Entity::find_by_id(instance_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("流程实例不存在"))?;

        // 仅允许未结束的实例撤回
        let cur_status = instance.status.as_deref().unwrap_or("");
        if cur_status == instance_status::COMPLETED || cur_status == instance_status::TERMINATED || cur_status == instance_status::CANCELLED {
            return Err(AppError::validation("流程已结束，无法撤回"));
        }

        // 先捕获事件所需字段，避免后续 instance 被 move 后无法引用
        let business_type = instance.business_type.clone();
        let business_id = instance.business_id;

        // 取消所有待处理任务（每次循环 clone reason 以移交所有权给 Set）
        let pending_tasks = bpm_task::Entity::find()
            .filter(bpm_task::Column::InstanceId.eq(instance_id))
            .filter(bpm_task::Column::Status.eq(task_status::PENDING))
            .all(&txn)
            .await?;

        for task in pending_tasks {
            let mut task_active: bpm_task::ActiveModel = task.into();
            task_active.status = Set(Some(task_status::CANCELLED.to_string()));
            task_active.approval_opinion = Set(cancel_reason.clone());
            task_active.handled_at = Set(Some(chrono::Utc::now()));
            task_active.updated_at = Set(Some(chrono::Utc::now()));
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "bpm_task",
                task_active,
                user_id,
            )
            .await?;
        }

        // 更新实例状态为 CANCELLED
        let mut instance_active: bpm_process_instance::ActiveModel = instance.into();
        instance_active.status = Set(Some(instance_status::CANCELLED.to_string()));
        instance_active.completed_at = Set(Some(chrono::Utc::now()));
        instance_active.updated_at = Set(Some(chrono::Utc::now()));
        instance_active.remarks = Set(cancel_reason);
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "bpm_process_instance",
            instance_active,
            user_id,
        )
        .await?;

        // 收集事件，commit 后再 publish（撤回必然发布流程结束事件，无需 Option 包装）
        let pending_event = crate::services::event_bus::BusinessEvent::BpmProcessFinished {
            business_type,
            business_id,
            approved: false,
            approver_id: user_id.unwrap_or_default(),
        };

        txn.commit().await?;

        crate::services::event_bus::EVENT_BUS.publish(pending_event);
        Ok(())
    }

    /// Get BPM business relation info
    pub async fn get_business_relation(
        &self,
        business_type: &str,
        business_id: i32,
    ) -> Result<BpmBusinessRelation, AppError> {
        let instance = bpm_process_instance::Entity::find()
            .filter(bpm_process_instance::Column::BusinessType.eq(business_type))
            .filter(bpm_process_instance::Column::BusinessId.eq(business_id))
            .order_by_desc(bpm_process_instance::Column::CreatedAt)
            .one(&*self.db)
            .await?;

        if let Some(inst) = instance {
            let tasks = bpm_task::Entity::find()
                .filter(bpm_task::Column::InstanceId.eq(inst.id))
                .all(&*self.db)
                .await?;

            Ok(BpmBusinessRelation {
                has_process: true,
                instance_id: inst.id,
                instance_no: inst.instance_no,
                process_status: inst.status.unwrap_or_default(),
                started_at: inst.started_at.unwrap_or_default(),
                completed_at: inst.completed_at,
                task_count: tasks.len() as i32,
                completed_tasks: tasks
                    .iter()
                    .filter(|t| t.status.as_deref() == Some(task_status::COMPLETED))
                    .count() as i32,
                pending_tasks: tasks
                    .iter()
                    .filter(|t| t.status.as_deref() == Some(task_status::PENDING))
                    .count() as i32,
            })
        } else {
            Ok(BpmBusinessRelation {
                has_process: false,
                instance_id: 0,
                instance_no: String::new(),
                process_status: "NONE".to_string(),
                started_at: chrono::Utc::now(),
                completed_at: None,
                task_count: 0,
                completed_tasks: 0,
                pending_tasks: 0,
            })
        }
    }

    /// Get process instance by business
    pub async fn get_process_by_business(
        &self,
        business_type: &str,
        business_id: i32,
    ) -> Result<Option<bpm_process_instance::Model>, AppError> {
        bpm_process_instance::Entity::find()
            .filter(bpm_process_instance::Column::BusinessType.eq(business_type))
            .filter(bpm_process_instance::Column::BusinessId.eq(business_id))
            .order_by_desc(bpm_process_instance::Column::CreatedAt)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::database(e.to_string()))
    }

    // ========== 审批链功能 ==========

    /// 获取流程实例的审批链
    pub async fn get_approval_chain(
        &self,
        instance_id: i32,
    ) -> Result<Vec<ApprovalChainNode>, AppError> {
        let instance = bpm_process_instance::Entity::find_by_id(instance_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("流程实例不存在"))?;

        let definition = bpm_process_definition::Entity::find_by_id(instance.process_definition_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("流程定义不存在"))?;

        let tasks = bpm_task::Entity::find()
            .filter(bpm_task::Column::InstanceId.eq(instance_id))
            .order_by_asc(bpm_task::Column::CreatedAt)
            .all(&*self.db)
            .await?;

        let mut chain = Vec::new();

        if let Some(flow_def) = definition.config {
            if let Some(nodes) = flow_def.get("nodes").and_then(|n| n.as_array()) {
                for node in nodes {
                    let node_id = node
                        .get("id")
                        .and_then(|i| i.as_str())
                        .unwrap_or("")
                        .to_string();
                    let node_name = node
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();
                    let node_type = node
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("")
                        .to_string();

                    // 查找对应的任务
                    let task = tasks.iter().find(|t| t.node_id == node_id);

                    chain.push(ApprovalChainNode {
                        node_id: node_id.clone(),
                        node_name,
                        node_type,
                        assignee_id: task.and_then(|t| t.actual_handler_id),
                        assignee_name: None, // 可以通过关联查询获取用户名
                        status: task
                            .map(|t| t.status.clone())
                            .unwrap_or_else(|| Some(task_status::PENDING.to_string()))
                            .unwrap_or_default(),
                        comment: task.and_then(|t| t.approval_opinion.clone()),
                        completed_at: task.and_then(|t| t.handled_at),
                        due_time: task.and_then(|t| t.due_date),
                    });
                }
            }
        }

        Ok(chain)
    }

    /// 获取流程实例详情（包含审批链）
    pub async fn get_instance_detail(
        &self,
        instance_id: i32,
    ) -> Result<ProcessInstanceDetail, AppError> {
        let instance = bpm_process_instance::Entity::find_by_id(instance_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("流程实例不存在"))?;

        let definition = bpm_process_definition::Entity::find_by_id(instance.process_definition_id)
            .one(&*self.db)
            .await?;

        let tasks = bpm_task::Entity::find()
            .filter(bpm_task::Column::InstanceId.eq(instance_id))
            .order_by_asc(bpm_task::Column::CreatedAt)
            .all(&*self.db)
            .await?;

        let approval_chain = self.get_approval_chain(instance_id).await?;

        Ok(ProcessInstanceDetail {
            instance: instance.clone(),
            definition_name: definition.map(|d| d.name).unwrap_or_default(),
            tasks,
            approval_chain,
        })
    }
}
