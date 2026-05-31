#![allow(dead_code)]
use crate::models::dto::bpm_dto::{
    ApproveTaskRequest, CreateProcessDefinitionRequest, CreateVersionRequest,
    ProcessDefinitionQuery, StartProcessRequest, StartProcessResponse, TaskQuery, TemplateQuery,
    UpdateProcessDefinitionRequest,
};
use crate::models::dto::PageResponse;
use crate::models::{bpm_process_definition, bpm_process_instance, bpm_task};
use crate::utils::error::AppError;
use sea_orm::*;
use std::sync::Arc;

/// 评估 BPM 边条件表达式
/// 支持的条件格式:
/// - `${amount} > 10000` - 变量数值比较
/// - `${status} == 'APPROVED'` - 变量字符串比较
/// - `${level} >= 3` - 变量数值比较
fn evaluate_bpm_condition(condition: &str, variables: &Option<serde_json::Value>) -> bool {
    let vars = match variables {
        Some(v) => v,
        None => return false,
    };

    let condition = condition.trim();
    if condition.is_empty() {
        return true; // 无条件默认通过
    }

    // 提取变量名和比较操作: ${var_name} operator value
    let re = regex::Regex::new(r"\$\{(\w+)\}\s*(==|!=|>|<|>=|<=)\s*(.+)").unwrap();

    if let Some(caps) = re.captures(condition) {
        let var_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let operator = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let expected_value = caps.get(3).map(|m| m.as_str()).unwrap_or("").trim();

        // 获取实际变量值
        let actual_value = vars.get(var_name).and_then(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .or_else(|| v.as_i64().map(|i| i.to_string()))
                .or_else(|| v.as_f64().map(|f| f.to_string()))
        });

        match actual_value {
            Some(actual) => {
                // 尝试数值比较
                if let (Ok(actual_num), Ok(expected_num)) =
                    (actual.parse::<f64>(), expected_value.parse::<f64>())
                {
                    match operator {
                        ">" => actual_num > expected_num,
                        "<" => actual_num < expected_num,
                        ">=" => actual_num >= expected_num,
                        "<=" => actual_num <= expected_num,
                        "==" => (actual_num - expected_num).abs() < f64::EPSILON,
                        "!=" => (actual_num - expected_num).abs() >= f64::EPSILON,
                        _ => false,
                    }
                } else {
                    // 字符串比较
                    let expected = expected_value.trim_matches('\'').trim_matches('"');
                    match operator {
                        "==" => actual == expected,
                        "!=" => actual != expected,
                        _ => false,
                    }
                }
            }
            None => false,
        }
    } else {
        // 无法解析的条件，默认通过并记录警告
        tracing::warn!("无法解析 BPM 条件表达式: {}", condition);
        true
    }
}

pub struct BpmService {
    db: Arc<DatabaseConnection>,
}

impl BpmService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn start_process(
        &self,
        req: StartProcessRequest,
    ) -> Result<StartProcessResponse, AppError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let definition = bpm_process_definition::Entity::find()
            .filter(bpm_process_definition::Column::Code.eq(&req.process_key))
            .filter(bpm_process_definition::Column::Status.eq("ACTIVE"))
            .one(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| {
                AppError::NotFound("Process definition not found or inactive".to_string())
            })?;

        let instance_no = format!(
            "BPM{}{}",
            chrono::Local::now().format("%Y%m%d%H%M%S"),
            req.business_id
        );
        let instance_model = bpm_process_instance::ActiveModel {
            process_definition_id: Set(definition.id),
            instance_no: Set(instance_no.clone()),
            business_type: Set(req.business_type.clone()),
            business_id: Set(req.business_id),
            title: Set(format!("流程审批-{}", req.business_id)),
            initiator_id: Set(req.initiator_id),
            initiator_name: Set("".to_string()),
            status: Set(Some("PROCESSING".to_string())),
            variables: Set(req.variables),
            started_at: Set(Some(chrono::Utc::now())),
            ..Default::default()
        };

        let instance = instance_model
            .insert(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut task_ids = vec![];
        if let Some(flow_def) = definition.config {
            if let Some(nodes) = flow_def.get("nodes").and_then(|n| n.as_array()) {
                // Find start_event or first user_task
                let start_node = nodes
                    .iter()
                    .find(|n| n.get("type").and_then(|t| t.as_str()) == Some("start_event"));

                let mut first_task_node = None;

                if let Some(start) = start_node {
                    let start_id = start.get("id").and_then(|i| i.as_str()).unwrap_or("");
                    if let Some(edges) = flow_def.get("edges").and_then(|e| e.as_array()) {
                        if let Some(edge) = edges
                            .iter()
                            .find(|e| e.get("source").and_then(|s| s.as_str()) == Some(start_id))
                        {
                            let target_id =
                                edge.get("target").and_then(|t| t.as_str()).unwrap_or("");
                            first_task_node = nodes
                                .iter()
                                .find(|n| n.get("id").and_then(|i| i.as_str()) == Some(target_id));
                        }
                    }
                }

                if first_task_node.is_none() {
                    first_task_node = nodes
                        .iter()
                        .find(|n| n.get("type").and_then(|t| t.as_str()) == Some("user_task"));
                }

                if let Some(first_task) = first_task_node {
                    let task_model = bpm_task::ActiveModel {
                        instance_id: Set(instance.id),
                        process_definition_id: Set(definition.id),
                        task_no: Set(format!(
                            "TSK{}{}",
                            chrono::Local::now().format("%Y%m%d%H%M%S"),
                            instance.id
                        )),
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
                        status: Set(Some("pending".to_string())),
                        created_at: Set(Some(chrono::Utc::now())),
                        updated_at: Set(Some(chrono::Utc::now())),
                        ..Default::default()
                    };
                    let task = task_model
                        .insert(&txn)
                        .await
                        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
                    task_ids.push(task.id);
                } else {
                    // No task found, auto complete
                    let mut instance_active: bpm_process_instance::ActiveModel =
                        instance.clone().into();
                    instance_active.status = Set(Some("COMPLETED".to_string()));
                    instance_active.completed_at = Set(Some(chrono::Utc::now()));
                    instance_active
                        .update(&txn)
                        .await
                        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

                    crate::services::event_bus::EVENT_BUS.publish(
                        crate::services::event_bus::BusinessEvent::BpmProcessFinished {
                            business_type: req.business_type.clone(),
                            business_id: req.business_id,
                            approved: true,
                        },
                    );
                }
            }
        }

        txn.commit()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(StartProcessResponse {
            instance_id: instance.id,
            instance_no,
            task_ids,
        })
    }

    pub async fn approve_task(&self, req: ApproveTaskRequest) -> Result<(), AppError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let task = bpm_task::Entity::find_by_id(req.task_id)
            .one(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Task not found".to_string()))?;

        if task.status.as_deref() != Some("pending") {
            return Err(AppError::ValidationError("Task is not pending".to_string()));
        }

        let process_instance_id = task.instance_id;
        let instance = bpm_process_instance::Entity::find_by_id(process_instance_id)
            .one(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Process instance not found".into()))?;

        let definition = bpm_process_definition::Entity::find_by_id(instance.process_definition_id)
            .one(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Definition not found".into()))?;

        // 1. Update current task status
        let mut task_active: bpm_task::ActiveModel = task.clone().into();
        task_active.status = Set(Some(if req.action == "approve" {
            "completed".to_string()
        } else {
            "rejected".to_string()
        }));
        task_active.actual_handler_id = Set(Some(req.handler_id));
        task_active.approval_opinion = Set(req.approval_opinion);
        task_active.handled_at = Set(Some(chrono::Utc::now()));
        task_active.updated_at = Set(Some(chrono::Utc::now()));
        task_active
            .update(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if req.action == "reject" {
            // End instance if rejected
            let mut instance_active: bpm_process_instance::ActiveModel = instance.clone().into();
            instance_active.status = Set(Some("TERMINATED".to_string()));
            instance_active.completed_at = Set(Some(chrono::Utc::now()));
            instance_active.updated_at = Set(Some(chrono::Utc::now()));
            instance_active
                .update(&txn)
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;

            if let (Some(b_type), Some(b_id)) = (
                Some(instance.business_type.clone()),
                Some(instance.business_id),
            ) {
                crate::services::event_bus::EVENT_BUS.publish(
                    crate::services::event_bus::BusinessEvent::BpmProcessFinished {
                        business_type: b_type,
                        business_id: b_id,
                        approved: false,
                    },
                );
            }
        } else {
            // Approve -> Find next node
            let mut next_task_created = false;

            if let Some(flow_def) = definition.config {
                if let (Some(nodes), Some(edges)) = (
                    flow_def.get("nodes").and_then(|n| n.as_array()),
                    flow_def.get("edges").and_then(|e| e.as_array()),
                ) {
                    // 查找从当前任务节点出发的边，支持条件评估
                    let matching_edge = edges.iter().find(|e| {
                        let source_match =
                            e.get("source").and_then(|s| s.as_str()) == Some(&task.node_id);
                        if !source_match {
                            return false;
                        }

                        // 检查边条件
                        if let Some(condition) = e.get("condition").and_then(|c| c.as_str()) {
                            evaluate_bpm_condition(condition, &instance.variables)
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
                                let new_task = bpm_task::ActiveModel {
                                    instance_id: Set(instance.id),
                                    process_definition_id: Set(definition.id),
                                    task_no: Set(format!(
                                        "TSK{}{}",
                                        chrono::Local::now().format("%Y%m%d%H%M%S"),
                                        instance.id
                                    )),
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
                                    status: Set(Some("pending".to_string())),
                                    created_at: Set(Some(chrono::Utc::now())),
                                    updated_at: Set(Some(chrono::Utc::now())),
                                    ..Default::default()
                                };
                                new_task
                                    .insert(&txn)
                                    .await
                                    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
                                next_task_created = true;
                            } else if node_type == "end_event" {
                                // 结束事件，在下面处理
                            }
                        }
                    }
                }
            }

            if !next_task_created {
                // No more user tasks, instance is completed
                let mut instance_active: bpm_process_instance::ActiveModel =
                    instance.clone().into();
                instance_active.status = Set(Some("COMPLETED".to_string()));
                instance_active.completed_at = Set(Some(chrono::Utc::now()));
                instance_active
                    .update(&txn)
                    .await
                    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

                if let (Some(b_type), Some(b_id)) = (
                    Some(instance.business_type.clone()),
                    Some(instance.business_id),
                ) {
                    crate::services::event_bus::EVENT_BUS.publish(
                        crate::services::event_bus::BusinessEvent::BpmProcessFinished {
                            business_type: b_type,
                            business_id: b_id,
                            approved: true,
                        },
                    );
                }
            }
        }

        txn.commit()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    pub async fn query_user_tasks(
        &self,
        query: TaskQuery,
    ) -> Result<PageResponse<bpm_task::Model>, AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(10);

        let mut stmt = bpm_task::Entity::find();

        if let Some(user_id) = query.user_id {
            stmt = stmt.filter(bpm_task::Column::ActualHandlerId.eq(user_id));
        }

        if let Some(status) = query.status {
            stmt = stmt.filter(bpm_task::Column::Status.eq(status));
        }

        let paginator = stmt.paginate(&*self.db, page_size);
        let total = paginator
            .num_items()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let items = paginator
            .fetch_page(page - 1)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

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
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if let Some(inst) = instance {
            let tasks = bpm_task::Entity::find()
                .filter(bpm_task::Column::InstanceId.eq(inst.id))
                .all(&*self.db)
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;

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
                    .filter(|t| t.status.as_deref() == Some("completed"))
                    .count() as i32,
                pending_tasks: tasks
                    .iter()
                    .filter(|t| t.status.as_deref() == Some("pending"))
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
            .map_err(|e| AppError::DatabaseError(e.to_string()))
    }

    // ========== 审批链功能 ==========

    /// 获取流程实例的审批链
    pub async fn get_approval_chain(
        &self,
        instance_id: i32,
    ) -> Result<Vec<ApprovalChainNode>, AppError> {
        let instance = bpm_process_instance::Entity::find_by_id(instance_id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("流程实例不存在".to_string()))?;

        let definition = bpm_process_definition::Entity::find_by_id(instance.process_definition_id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("流程定义不存在".to_string()))?;

        let tasks = bpm_task::Entity::find()
            .filter(bpm_task::Column::InstanceId.eq(instance_id))
            .order_by_asc(bpm_task::Column::CreatedAt)
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

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
                            .unwrap_or_else(|| Some("pending".to_string()))
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
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("流程实例不存在".to_string()))?;

        let definition = bpm_process_definition::Entity::find_by_id(instance.process_definition_id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let tasks = bpm_task::Entity::find()
            .filter(bpm_task::Column::InstanceId.eq(instance_id))
            .order_by_asc(bpm_task::Column::CreatedAt)
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let approval_chain = self.get_approval_chain(instance_id).await?;

        Ok(ProcessInstanceDetail {
            instance: instance.clone(),
            definition_name: definition.map(|d| d.name).unwrap_or_default(),
            tasks,
            approval_chain,
        })
    }

    // ========== 流程监控功能 ==========

    /// 获取流程监控统计
    pub async fn get_monitor_stats(&self) -> Result<ProcessMonitorStats, AppError> {
        use sea_orm::QuerySelect;

        let total_instances = bpm_process_instance::Entity::find()
            .count(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let processing_instances = bpm_process_instance::Entity::find()
            .filter(bpm_process_instance::Column::Status.eq("PROCESSING"))
            .count(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let completed_instances = bpm_process_instance::Entity::find()
            .filter(bpm_process_instance::Column::Status.eq("COMPLETED"))
            .count(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let terminated_instances = bpm_process_instance::Entity::find()
            .filter(bpm_process_instance::Column::Status.eq("TERMINATED"))
            .count(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let total_tasks = bpm_task::Entity::find()
            .count(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let pending_tasks = bpm_task::Entity::find()
            .filter(bpm_task::Column::Status.eq("PENDING"))
            .count(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let completed_tasks = bpm_task::Entity::find()
            .filter(bpm_task::Column::Status.eq("COMPLETED"))
            .count(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let rejected_tasks = bpm_task::Entity::find()
            .filter(bpm_task::Column::Status.eq("REJECTED"))
            .count(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 计算平均流程处理时长（分钟）
        let avg_duration = bpm_process_instance::Entity::find()
            .filter(bpm_process_instance::Column::Status.eq("COMPLETED"))
            .filter(bpm_process_instance::Column::CompletedAt.is_not_null())
            .select_only()
            .column_as(
                sea_orm::sea_query::Expr::cust(
                    "AVG(EXTRACT(EPOCH FROM (completed_at - started_at)) / 60)",
                ),
                "avg_duration",
            )
            .into_tuple::<Option<f64>>()
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .flatten();

        Ok(ProcessMonitorStats {
            total_instances: total_instances as i64,
            processing_instances: processing_instances as i64,
            completed_instances: completed_instances as i64,
            terminated_instances: terminated_instances as i64,
            total_tasks: total_tasks as i64,
            pending_tasks: pending_tasks as i64,
            completed_tasks: completed_tasks as i64,
            rejected_tasks: rejected_tasks as i64,
            avg_process_duration_minutes: avg_duration,
        })
    }

    /// 获取待处理任务列表（用于监控）
    pub async fn get_pending_tasks_for_monitor(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<PageResponse<bpm_task::Model>, AppError> {
        let stmt = bpm_task::Entity::find().filter(bpm_task::Column::Status.eq("PENDING"));

        let paginator = stmt.paginate(&*self.db, page_size);
        let total = paginator
            .num_items()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let items = paginator
            .fetch_page(page - 1)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

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

    /// 获取流程实例列表（用于监控）
    pub async fn list_instances_for_monitor(
        &self,
        status: Option<String>,
        page: u64,
        page_size: u64,
    ) -> Result<PageResponse<bpm_process_instance::Model>, AppError> {
        let mut stmt = bpm_process_instance::Entity::find();

        if let Some(s) = status {
            stmt = stmt.filter(bpm_process_instance::Column::Status.eq(s));
        }

        stmt = stmt.order_by_desc(bpm_process_instance::Column::CreatedAt);

        let paginator = stmt.paginate(&*self.db, page_size);
        let total = paginator
            .num_items()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let items = paginator
            .fetch_page(page - 1)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

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
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let task = bpm_task::Entity::find_by_id(task_id)
            .one(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("任务不存在".to_string()))?;

        if task.status.as_deref() != Some("pending") {
            return Err(AppError::ValidationError("只能转办待处理任务".to_string()));
        }

        let mut task_active: bpm_task::ActiveModel = task.into();
        task_active.actual_handler_id = Set(Some(new_assignee_id));
        task_active.approval_opinion = Set(Some(format!("[转办] {}", transfer_reason)));
        task_active.updated_at = Set(Some(chrono::Utc::now()));
        task_active
            .update(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        txn.commit()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    /// 催办任务
    pub async fn urge_task(&self, task_id: i32, urge_message: &str) -> Result<(), AppError> {
        let task = bpm_task::Entity::find_by_id(task_id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("任务不存在".to_string()))?;

        if task.status.as_deref() != Some("pending") {
            return Err(AppError::ValidationError("只能催办待处理任务".to_string()));
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
            let _ = notification_service
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
                        sender_id: Some(0),
                        sender_name: Some("系统".to_string()),
                    },
                )
                .await;
        }

        Ok(())
    }
}

/// BPM business relation info
#[derive(Debug, serde::Serialize)]
pub struct BpmBusinessRelation {
    pub has_process: bool,
    pub instance_id: i32,
    pub instance_no: String,
    pub process_status: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub task_count: i32,
    pub completed_tasks: i32,
    pub pending_tasks: i32,
}

/// 审批链节点信息
#[derive(Debug, serde::Serialize)]
pub struct ApprovalChainNode {
    pub node_id: String,
    pub node_name: String,
    pub node_type: String,
    pub assignee_id: Option<i32>,
    pub assignee_name: Option<String>,
    pub status: String,
    pub comment: Option<String>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub due_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// 流程监控统计
#[derive(Debug, serde::Serialize)]
pub struct ProcessMonitorStats {
    pub total_instances: i64,
    pub processing_instances: i64,
    pub completed_instances: i64,
    pub terminated_instances: i64,
    pub total_tasks: i64,
    pub pending_tasks: i64,
    pub completed_tasks: i64,
    pub rejected_tasks: i64,
    pub avg_process_duration_minutes: Option<f64>,
}

/// 流程实例详情
#[derive(Debug, serde::Serialize)]
pub struct ProcessInstanceDetail {
    pub instance: bpm_process_instance::Model,
    pub definition_name: String,
    pub tasks: Vec<bpm_task::Model>,
    pub approval_chain: Vec<ApprovalChainNode>,
}

impl BpmService {
    pub async fn create_process_definition(
        &self,
        _req: CreateProcessDefinitionRequest,
    ) -> Result<bpm_process_definition::Model, AppError> {
        Err(AppError::BadRequest("Not implemented".to_string()))
    }
    pub async fn get_process_definition(
        &self,
        _id: i32,
    ) -> Result<Option<bpm_process_definition::Model>, AppError> {
        Err(AppError::NotFound(format!(
            "Process definition not found: {}",
            _id
        )))
    }
    pub async fn update_process_definition(
        &self,
        _id: i32,
        _req: UpdateProcessDefinitionRequest,
    ) -> Result<bpm_process_definition::Model, AppError> {
        Err(AppError::BadRequest("Not implemented".to_string()))
    }
    pub async fn delete_process_definition(&self, _id: i32) -> Result<(), AppError> {
        Err(AppError::BadRequest("Not implemented".to_string()))
    }
    pub async fn list_process_definitions(
        &self,
        _query: ProcessDefinitionQuery,
    ) -> Result<PageResponse<bpm_process_definition::Model>, AppError> {
        Ok(PageResponse {
            data: vec![],
            total: 0,
            page: 1,
            page_size: 10,
            total_pages: 0,
        })
    }
    pub async fn create_process_version(
        &self,
        _req: CreateVersionRequest,
    ) -> Result<bpm_process_definition::Model, AppError> {
        Err(AppError::BadRequest("Not implemented".to_string()))
    }
    pub async fn list_process_versions(
        &self,
        _definition_id: i32,
    ) -> Result<Vec<bpm_process_definition::Model>, AppError> {
        Ok(vec![])
    }
    pub async fn activate_process_version(
        &self,
        _id: i32,
    ) -> Result<bpm_process_definition::Model, AppError> {
        Err(AppError::BadRequest("Not implemented".to_string()))
    }
    pub async fn save_as_template(&self, _id: i32, _name: String) -> Result<(), AppError> {
        Err(AppError::BadRequest("Not implemented".to_string()))
    }
    pub async fn list_templates(
        &self,
        _query: TemplateQuery,
    ) -> Result<PageResponse<bpm_process_definition::Model>, AppError> {
        Ok(PageResponse {
            data: vec![],
            total: 0,
            page: 1,
            page_size: 10,
            total_pages: 0,
        })
    }
    pub async fn create_from_template(
        &self,
        _template_id: i32,
    ) -> Result<bpm_process_definition::Model, AppError> {
        Err(AppError::BadRequest("Not implemented".to_string()))
    }
}
