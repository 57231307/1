//! 流程实例生命周期子模块（bpm_ops/instance）
//!
//! 从原 `bpm_service.rs` 迁移 7 个方法：
//! - start_process：发起流程（创建实例 + 创建首个任务 + 事务提交后通知处理人）
//! - create_first_task：创建首个任务（私有，仅 start_process 调用；解析不到可执行审批节点即拒绝发起）
//! - `cancel_instance`：撤回流程实例（终止实例 + 取消待处理任务 + 发布拒绝事件）
//! - `get_process_by_business`：按业务类型 + 业务 ID 查询流程实例
//! - `get_instance_detail`：获取流程实例详情（含审批链）
//! - `get_approval_chain`：获取流程实例审批链（pub，被 get_instance_detail 调用）
//! - `get_business_relation`：获取 BPM 业务关联信息
//!
//! 跨 facade 依赖：
//! - create_first_task 调用 facade 的 BpmService::resolve_first_task_node（pub(crate)）

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait,
};

use crate::models::department;
use crate::models::dto::bpm_dto::{StartProcessRequest, StartProcessResponse};

use crate::models::status::bpm_instance as instance_status;
use crate::models::status::bpm_task as task_status;
use crate::models::{bpm_process_definition, bpm_process_instance, bpm_task};
use crate::services::bpm_ops::{ApplicantDept, applicant_department};
use crate::services::bpm_service::BpmService;
use crate::services::bpm_service::evaluate_bpm_condition;
use crate::services::bpm_service_dto::{
    ApprovalChainNode, BpmBusinessRelation, ProcessInstanceDetail,
};
use crate::utils::error::AppError;

impl BpmService {
    /// 创建首个任务：发起流程必须解析出可执行的审批节点并落到真实审批人。
    ///
    /// 全部路径 fail-closed，四类形状都拒绝发起并由调用方回滚事务：
    /// ① 解析不到首个任务节点；② 解析到的节点类型不是审批节点（起始连线指向的
    /// 目标不经类型校验，可能带回引擎不建单的词表项）；③ 审批人解析不出
    /// （指定用户缺失，或按部门上级解析时部门链断裂）；④ 从首任务出发的审批链
    /// 不可闭合（节点无出边、条件分支与发起数据全部不匹配、连线悬空、目标类型
    /// 不可执行、链路成环）。
    /// 拒绝时绝不把实例置成已完成、不建工单、不发布批准事件——
    /// 未经人工审批的单据不得进入已批准状态。
    async fn create_first_task(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        definition: &bpm_process_definition::Model,
        instance: &bpm_process_instance::Model,
    ) -> Result<bpm_task::Model, AppError> {
        let flow_def = definition.config.as_ref().ok_or_else(|| {
            AppError::validation_displayable(format!(
                "流程定义 {} 未配置审批节点，无法发起审批，请先在流程定义中配置节点与审批人",
                definition.code
            ))
        })?;

        let first_task_node = Self::resolve_first_task_node(flow_def).ok_or_else(|| {
            tracing::warn!(
                definition_id = definition.id,
                instance_id = instance.id,
                process_key = %definition.code,
                "流程定义解析不到可执行的审批任务节点，已拒绝发起审批（内部标识仅随本条日志落盘）"
            );
            AppError::validation_displayable(format!(
                "流程定义 {} 没有可执行的审批节点，已拒绝发起审批。请在流程定义中配置审批节点与审批人，并确认节点类型与连线可被执行，然后重新提交",
                definition.code
            ))
        })?;

        // 起始连线指向的目标节点在解析时不经类型校验，可能带回通知、条件、结束事件等
        // 引擎不建单的词表项；这类形状若照常建单会产生无人可审或错挂处理人的孤儿工单，
        // 故首任务节点必须显式为审批节点（user_task），否则同样拒绝发起。
        let node_type = first_task_node
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("未配置");
        if node_type != "user_task" {
            let node_id = first_task_node
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or("unknown");
            let node_name = first_task_node
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("");
            tracing::warn!(
                definition_id = definition.id,
                instance_id = instance.id,
                process_key = %definition.code,
                node_id = %node_id,
                node_type = %node_type,
                "首个任务节点类型不是审批节点，已拒绝发起审批（内部标识仅随本条日志落盘）"
            );
            return Err(AppError::validation_displayable(format!(
                "流程定义 {} 的起始流程节点 {node_name}({node_id}) 类型为「{node_type}」，不是可执行的审批节点，审批引擎只对 user_task 节点建审批工单，已拒绝发起审批。请在流程定义中把起始连线指向配置了审批人的 user_task 节点，然后重新提交",
                definition.code
            )));
        }

        // 审批人解析失败属定义配置缺陷：拒绝发起、事务回滚，绝不建无人可审的孤儿工单。
        let assignee_id = super::resolve_node_assignee(first_task_node, instance, txn).await?;

        // 发起期链干跑（裁定口径：条件分支全部不匹配的分支语义缺陷必须在发起时给结论，
        // 不允许"实例已创建却在首个批准后自动完成"的形态存在）。
        validate_forward_path(
            &definition.code,
            first_task_node,
            flow_def,
            &instance.variables,
        )?;

        // 创建首个用户任务。审批人必须解析得出，否则工单落库后在按人过滤的待办里永不可见。
        let task_model = bpm_task::ActiveModel {
            instance_id: Set(instance.id),
            process_definition_id: Set(definition.id),
            // 任务号在同一事务内向取号器申请，保证并发下唯一
            task_no: Set(
                crate::utils::number_generator::DocumentNumberGenerator::generate_no_with_txn(
                    txn,
                    "TSK",
                    bpm_task::Entity,
                    bpm_task::Column::TaskNo,
                )
                .await?,
            ),
            node_id: Set(first_task_node
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or("unknown")
                .to_string()),
            node_name: Set(first_task_node
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("Task")
                .to_string()),
            node_type: Set("user_task".to_string()),
            task_type: Set(Some("user_task".to_string())),
            actual_handler_id: Set(assignee_id),
            status: Set(Some(task_status::PENDING.to_string())),
            created_at: Set(Some(chrono::Utc::now())),
            updated_at: Set(Some(chrono::Utc::now())),
            ..Default::default()
        };
        Ok(task_model.insert(txn).await?)
    }

    pub async fn start_process(
        &self,
        req: StartProcessRequest,
    ) -> Result<StartProcessResponse, AppError> {
        // 幂等预检：同一 (business_type, business_id) 已有在途实例时复用之，不再叠第二张工单。
        // 历史缺陷：无此检查，重复提交会产生并行实例，各实例独立审批 → 单据与工单进入分裂态。
        if let Some(existing) = bpm_process_instance::Entity::find()
            .filter(bpm_process_instance::Column::BusinessType.eq(&req.business_type))
            .filter(bpm_process_instance::Column::BusinessId.eq(req.business_id))
            .filter(bpm_process_instance::Column::Status.eq(instance_status::PROCESSING))
            .order_by_desc(bpm_process_instance::Column::Id)
            .one(&*self.db)
            .await?
        {
            let task_ids = bpm_task::Entity::find()
                .filter(bpm_task::Column::InstanceId.eq(existing.id))
                .all(&*self.db)
                .await?
                .into_iter()
                .map(|t| t.id)
                .collect();
            tracing::info!(
                instance_id = existing.id,
                business_type = %req.business_type,
                business_id = req.business_id,
                "该单据已有在途审批实例，复用既有实例而不重复建单"
            );
            return Ok(StartProcessResponse {
                instance_id: existing.id,
                instance_no: existing.instance_no,
                task_ids,
            });
        }

        let txn = self.db.begin().await?;

        let definition = bpm_process_definition::Entity::find()
            .filter(bpm_process_definition::Column::Code.eq(&req.process_key))
            .filter(bpm_process_definition::Column::Status.eq("ACTIVE"))
            .one(&txn)
            .await?
            .ok_or_else(|| {
                // 缺定义是"无法发起审批"的根因，点名 process_key 供管理员直接行动
                // （用户裁定）；文案经 bpm_ops 单源拼装，404/NOT_FOUND 族映射不变，
                // SQL/引擎细节仍只随 Display 进 tracing 日志（utils/error.rs 安全边界）。
                AppError::not_found_displayable(
                    crate::services::bpm_ops::bpm_start_failure_message(
                        &req.process_key,
                        "流程实例未创建，单据未进入审批",
                    ),
                )
            })?;

        // P1 3-6 修复（批次 60）：改用 DocumentNumberGenerator 保证并发唯一性
        // 原实现基于时间戳 + business_id，同秒并发 + 同 business_id 会产生重复单号
        let instance_no =
            crate::utils::number_generator::DocumentNumberGenerator::generate_no_with_txn(
                &txn,
                "BPM",
                bpm_process_instance::Entity,
                bpm_process_instance::Column::InstanceNo,
            )
            .await?;
        // 发起人部门落库（此前 ActiveModel 用 ..Default::default() 从未写入该列，
        // 属存量缺陷：按部门上级解析审批人依赖实例上的部门归属可信，此处补全）。
        // 申报值只做存在性核验（不存在即拒，防悬挂 ID 触发 FK 裸抛 500）；
        // 未申报时按关联表主部门优先、users.department_id 兼容现算；
        // 歧义与未关联不阻断发起（多数流程不用部门解析审批人），
        // 由 dept_superior 节点解析时按各自形状 fail-closed 并点名该配置什么。
        let initiator_department_id = match req.initiator_department_id {
            Some(d) => {
                if department::Entity::find_by_id(d).one(&txn).await?.is_none() {
                    tracing::warn!(
                        initiator_id = req.initiator_id,
                        declared_department_id = d,
                        "发起单申报的部门不存在，已拒绝发起（内部标识仅随本条日志落盘）"
                    );
                    return Err(AppError::validation_displayable(
                        "发起单中申报的申请人部门不存在或已被删除，流程实例未创建。请在 系统设置-部门管理 核对申请人的部门归属后重新提交".to_string(),
                    ));
                }
                Some(d)
            }
            None => match applicant_department(&txn, req.initiator_id).await? {
                ApplicantDept::Unique(d) => Some(d),
                ApplicantDept::Ambiguous | ApplicantDept::Unresolvable => None,
            },
        };
        let instance_model = bpm_process_instance::ActiveModel {
            process_definition_id: Set(definition.id),
            instance_no: Set(instance_no.clone()),
            business_type: Set(req.business_type.clone()),
            business_id: Set(Some(req.business_id)),
            // applicant_id 与 initiator 同义（发起人），列 NOT NULL 必须显式写入
            applicant_id: Set(req.initiator_id),
            title: Set(format!("流程审批-{}", req.business_id)),
            initiator_id: Set(req.initiator_id),
            initiator_name: Set("".to_string()),
            initiator_department_id: Set(initiator_department_id),
            status: Set(Some(instance_status::PROCESSING.to_string())),
            variables: Set(req.variables.clone()),
            started_at: Set(Some(chrono::Utc::now())),
            ..Default::default()
        };

        let instance = instance_model.insert(&txn).await?;

        // 创建首个任务：解析/落库失败在此直接返回错误，事务整体回滚，
        // 实例与任务都不落库，也不会产生任何流程结束事件。
        let first_task = self.create_first_task(&txn, &definition, &instance).await?;

        txn.commit().await?;

        // 工单已真实落库才通知处理人：审批任务生成即"有人被叫醒"，不再靠人工翻待办列表
        super::notify_task_pending(&self.db, &first_task, &instance, "新工单已生成").await;

        Ok(StartProcessResponse {
            instance_id: instance.id,
            instance_no,
            task_ids: vec![first_task.id],
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
        if cur_status == instance_status::COMPLETED
            || cur_status == instance_status::TERMINATED
            || cur_status == instance_status::CANCELLED
        {
            // 状态门：流程已处于终态，前置条件不满足，归业务族
            return Err(AppError::business_displayable("流程已结束，无法撤回"));
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

        // 收集事件，commit 后再 publish。business_id 为 None 时无业务单据可回调，跳过；
        // 有业务关联但拿不到操作人身份时显式报错并跳过，不用 0 顶替审批人（0 会被
        // 下游写进业务单据审计，等于伪造归属）。
        let pending_event = match (business_id, user_id) {
            (Some(bid), Some(uid)) => Some(
                crate::services::event_bus::BusinessEvent::BpmProcessFinished {
                    business_type,
                    business_id: bid,
                    approved: false,
                    approver_id: uid,
                },
            ),
            (None, _) => None,
            (Some(_), None) => {
                tracing::error!(
                    instance_id,
                    "BPM 撤回事件未发布：缺少操作人身份，拒绝以 0 号顶替审批人写入业务单据审计"
                );
                None
            }
        };

        txn.commit().await?;

        if let Some(ev) = pending_event {
            crate::services::event_bus::EVENT_BUS.publish(ev);
        }
        Ok(())
    }

    /// Get BPM business relation info
    pub async fn get_business_relation(
        &self,
        business_type: &str,
        business_id: i64,
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
        business_id: i64,
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

/// 发起期审批链干跑：从首个任务节点起沿出边预演推进，只判定不写库、无副作用。
///
/// 调用方：create_first_task（发起事务内）。入参：流程标识、首个任务节点、
/// 流程定义配置、实例变量。返回 Err 时发起事务整体回滚，实例与工单均不落库。
///
/// 判定可行的依据：条件表达式求值使用的实例变量在发起时落库后不可再变，
/// 因此出边匹配结果对该实例是确定的，能在发起时预演到位。分支语义按裁定
/// 从"条件全部不匹配即按完成收尾"（等于自动批准）改为发起即拒绝；
/// 存量定义若存在不覆盖的条件组合，会在下一次发起时当场暴露，属预期。
///
/// 边匹配顺序与推进路径同源（try_advance_to_next_node 对出边取第一条匹配边：
/// 无 condition 键或 condition 求值通过即匹配；键存在但非字符串按无边界处理，
/// 空串条件经 evaluate_bpm_condition 判通过——两侧逐字符同语义，不各写一套）。
/// 终止条件是走到 end_event；审批链必须闭合，节点无出边同样拒绝
/// （推进侧对该形状保留现行完成收尾仅服务于此规则生效前的历史在途实例，
/// 新建实例不可能带着无出边形态通过本校验）。
fn validate_forward_path(
    definition_code: &str,
    first_task_node: &serde_json::Value,
    flow_def: &serde_json::Value,
    variables: &Option<serde_json::Value>,
) -> Result<(), AppError> {
    let nodes = flow_def.get("nodes").and_then(|n| n.as_array());
    let edges = flow_def.get("edges").and_then(|e| e.as_array());
    let (Some(nodes), Some(edges)) = (nodes, edges) else {
        // 首个任务能解析出来说明 nodes 必在；缺 edges 数组即审批链无从闭合，
        // 按不可执行定义拒绝发起（缺 edges 的旧旁路：首节点批准后直接完成）。
        tracing::warn!(
            process_key = %definition_code,
            "流程定义缺少连线数组，审批链无法闭合，已拒绝发起（内部标识仅随本条日志落盘）"
        );
        return Err(AppError::validation_displayable(format!(
            "流程定义 {definition_code} 缺少节点连线配置，审批通过后流程无法闭合结束，已拒绝发起审批。请在流程定义中补全节点连线后重新提交"
        )));
    };
    let first_id = first_task_node
        .get("id")
        .and_then(|i| i.as_str())
        .unwrap_or("unknown")
        .to_string();
    let mut current_id = first_id;
    let mut visited = std::collections::HashSet::new();
    loop {
        if !visited.insert(current_id.clone()) {
            tracing::warn!(
                process_key = %definition_code,
                node_id = %current_id,
                "审批链成环，已拒绝发起（内部标识仅随本条日志落盘）"
            );
            return Err(AppError::validation_displayable(format!(
                "流程定义 {definition_code} 的审批链存在循环连线，审批通过后无法到达结束节点，已拒绝发起审批。请在流程定义中修正连线走向后重新提交"
            )));
        }
        let current_name = nodes
            .iter()
            .find(|n| n.get("id").and_then(|i| i.as_str()) == Some(current_id.as_str()))
            .and_then(|n| n.get("name").and_then(|v| v.as_str()))
            .unwrap_or("");
        let outgoing: Vec<&serde_json::Value> = edges
            .iter()
            .filter(|e| e.get("source").and_then(|s| s.as_str()) == Some(current_id.as_str()))
            .collect();
        if outgoing.is_empty() {
            tracing::warn!(
                process_key = %definition_code,
                node_id = %current_id,
                "审批节点没有出边，链不闭合，已拒绝发起（内部标识仅随本条日志落盘）"
            );
            return Err(AppError::validation_displayable(format!(
                "流程定义 {definition_code} 的审批节点 {current_name} 之后没有连线，审批通过后流程无法闭合结束，已拒绝发起审批。请在流程定义中为该节点补上后续审批节点，或连线到结束节点后重新提交"
            )));
        }
        let matched = outgoing
            .iter()
            .find(|e| match e.get("condition").and_then(|c| c.as_str()) {
                Some(condition) => evaluate_bpm_condition(condition, variables),
                None => true,
            });
        let Some(edge) = matched else {
            tracing::warn!(
                process_key = %definition_code,
                node_id = %current_id,
                outgoing_count = outgoing.len(),
                "审批节点出边的条件分支与发起数据全部不匹配，已拒绝发起（内部标识仅随本条日志落盘）"
            );
            return Err(AppError::validation_displayable(format!(
                "流程定义 {definition_code} 的审批节点 {current_name} 出发的条件分支与发起数据全部不匹配，审批通过后没有可执行的分支，已拒绝发起审批。请在流程定义中补全条件覆盖，或为该节点增加一条不带条件的默认分支连线后重新提交"
            )));
        };
        let target_id = edge.get("target").and_then(|t| t.as_str()).unwrap_or("");
        let Some(target) = nodes
            .iter()
            .find(|n| n.get("id").and_then(|i| i.as_str()) == Some(target_id))
        else {
            // 连线悬空指向不存在节点属流程定义配置错误，但不等于自动批准：
            // 推进路径（try_advance_to_next_node）对该形状已 fail-closed 拒绝本次审批。
            // 干跑职责收窄为只前置拦截两类"完成收尾"旁路（条件全不匹配、节点无出边），
            // 悬空与不支持类型交由推进侧处理，避免与推进侧既有拒绝契约重复并改变发起形状。
            tracing::debug!(
                process_key = %definition_code,
                node_id = %current_id,
                target_id = %target_id,
                "干跑遇悬空连线目标，交由推进侧拒绝（干跑不前置拦截该形状）"
            );
            return Ok(());
        };
        match target.get("type").and_then(|t| t.as_str()).unwrap_or("") {
            "end_event" => {
                tracing::info!(
                    process_key = %definition_code,
                    hops = visited.len(),
                    "审批链干跑校验通过：从首个审批节点可闭合到结束节点"
                );
                return Ok(());
            }
            "user_task" => {
                current_id = target_id.to_string();
            }
            other => {
                // 目标类型不在引擎词表：不前置拦截（推进侧已 fail-closed），干跑到此收敛。
                tracing::debug!(
                    process_key = %definition_code,
                    node_id = %current_id,
                    target_type = %other,
                    "干跑遇不可执行目标类型，交由推进侧拒绝（干跑不前置拦截该形状）"
                );
                return Ok(());
            }
        }
    }
}
