use sea_orm::*;
use std::sync::Arc;
use crate::models::{bpm_process_definition, bpm_process_instance, bpm_task};
use crate::models::dto::bpm_dto::{StartProcessRequest, StartProcessResponse, ApproveTaskRequest, TaskQuery};
use crate::models::dto::PageResponse;
use crate::utils::error::AppError;

pub struct BpmService {
    db: Arc<DatabaseConnection>,
}

impl BpmService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn start_process(&self, req: StartProcessRequest) -> Result<StartProcessResponse, AppError> {
        let txn = self.db.begin().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        let definition = bpm_process_definition::Entity::find()
            .filter(bpm_process_definition::Column::Code.eq(&req.process_key))
            .filter(bpm_process_definition::Column::Status.eq("ACTIVE"))
            .one(&txn).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Process definition not found or inactive".to_string()))?;

        let instance_no = format!("BPM{}{}", chrono::Local::now().format("%Y%m%d%H%M%S"), req.business_id);
        let instance_model = bpm_process_instance::ActiveModel {
            process_definition_id: Set(definition.id),
            instance_no: Set(instance_no.clone()),
            business_type: Set(Some(req.business_type.clone())),
            business_id: Set(Some(req.business_id)),
            business_no: Set(Some(req.business_id.to_string())),
            applicant_id: Set(req.initiator_id),
            status: Set("PROCESSING".to_string()),
            variables: Set(req.variables),
            start_time: Set(chrono::Utc::now()),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
            ..Default::default()
        };
        
        let instance = instance_model.insert(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut task_ids = vec![];
        if let Some(flow_def) = definition.config {
            if let Some(nodes) = flow_def.get("nodes").and_then(|n| n.as_array()) {
                // Find start_event or first user_task
                let start_node = nodes.iter().find(|n| n.get("type").and_then(|t| t.as_str()) == Some("start_event"));
                
                let mut first_task_node = None;
                
                if let Some(start) = start_node {
                    let start_id = start.get("id").and_then(|i| i.as_str()).unwrap_or("");
                    if let Some(edges) = flow_def.get("edges").and_then(|e| e.as_array()) {
                        if let Some(edge) = edges.iter().find(|e| e.get("source").and_then(|s| s.as_str()) == Some(start_id)) {
                            let target_id = edge.get("target").and_then(|t| t.as_str()).unwrap_or("");
                            first_task_node = nodes.iter().find(|n| n.get("id").and_then(|i| i.as_str()) == Some(target_id));
                        }
                    }
                }
                
                if first_task_node.is_none() {
                    first_task_node = nodes.iter().find(|n| n.get("type").and_then(|t| t.as_str()) == Some("user_task"));
                }

                if let Some(first_task) = first_task_node {
                    let task_model = bpm_task::ActiveModel {
                        process_instance_id: Set(instance.id),
                        task_no: Set(format!("TSK{}{}", chrono::Local::now().format("%Y%m%d%H%M%S"), instance.id)),
                        node_id: Set(first_task.get("id").and_then(|i| i.as_str()).unwrap_or("unknown").to_string()),
                        node_name: Set(first_task.get("name").and_then(|n| n.as_str()).unwrap_or("Task").to_string()),
                        name: Set(first_task.get("name").and_then(|n| n.as_str()).unwrap_or("Task").to_string()),
                        task_type: Set("user_task".to_string()),
                        assignee_id: Set(first_task.get("assignee_value").and_then(|a| a.as_str()).and_then(|s| s.parse::<i32>().ok())),
                        status: Set("PENDING".to_string()),
                        business_type: Set(Some(req.business_type.clone())),
                        business_id: Set(Some(req.business_id)),
                        created_at: Set(chrono::Utc::now()),
                        updated_at: Set(chrono::Utc::now()),
                        ..Default::default()
                    };
                    let task = task_model.insert(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
                    task_ids.push(task.id);
                } else {
                    // No task found, auto complete
                    let mut instance_active: bpm_process_instance::ActiveModel = instance.clone().into();
                    instance_active.status = Set("COMPLETED".to_string());
                    instance_active.end_time = Set(Some(chrono::Utc::now()));
                    instance_active.update(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
                    
                    crate::services::event_bus::EVENT_BUS.publish(crate::services::event_bus::BusinessEvent::BpmProcessFinished {
                        business_type: req.business_type.clone(),
                        business_id: req.business_id,
                        approved: true,
                    });
                }
            }
        }

        txn.commit().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(StartProcessResponse {
            instance_id: instance.id,
            instance_no,
            task_ids,
        })
    }

    pub async fn approve_task(&self, req: ApproveTaskRequest) -> Result<(), AppError> {
        let txn = self.db.begin().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        let task = bpm_task::Entity::find_by_id(req.task_id)
            .one(&txn).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Task not found".to_string()))?;

        if task.status != "PENDING" {
            return Err(AppError::ValidationError("Task is not pending".to_string()));
        }

        let process_instance_id = task.process_instance_id;
        let instance = bpm_process_instance::Entity::find_by_id(process_instance_id)
            .one(&txn).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Process instance not found".into()))?;

        let definition = bpm_process_definition::Entity::find_by_id(instance.process_definition_id)
            .one(&txn).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Definition not found".into()))?;

        // 1. Update current task status
        let mut task_active: bpm_task::ActiveModel = task.clone().into();
        task_active.status = Set(if req.action == "approve" { "COMPLETED".to_string() } else { "REJECTED".to_string() });
        task_active.assignee_id = Set(Some(req.handler_id));
        task_active.comment = Set(req.approval_opinion);
        task_active.completed_at = Set(Some(chrono::Utc::now()));
        task_active.updated_at = Set(chrono::Utc::now());
        task_active.update(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if req.action == "reject" {
            // End instance if rejected
            let mut instance_active: bpm_process_instance::ActiveModel = instance.clone().into();
            instance_active.status = Set("TERMINATED".to_string());
            instance_active.end_time = Set(Some(chrono::Utc::now()));
            instance_active.updated_at = Set(chrono::Utc::now());
            instance_active.update(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
            
            if let (Some(b_type), Some(b_id)) = (instance.business_type.clone(), instance.business_id) {
                crate::services::event_bus::EVENT_BUS.publish(crate::services::event_bus::BusinessEvent::BpmProcessFinished {
                    business_type: b_type,
                    business_id: b_id,
                    approved: false,
                });
            }
        } else {
            // Approve -> Find next node
            let mut next_task_created = false;
            
            if let Some(flow_def) = definition.config {
                if let (Some(nodes), Some(edges)) = (flow_def.get("nodes").and_then(|n| n.as_array()), flow_def.get("edges").and_then(|e| e.as_array())) {
                    // Find outgoing edge from current task.node_id
                    // TODO: evaluate conditions on edges if multiple
                    if let Some(edge) = edges.iter().find(|e| e.get("source").and_then(|s| s.as_str()) == Some(&task.node_id)) {
                        let target_id = edge.get("target").and_then(|t| t.as_str()).unwrap_or("");
                        let target_node = nodes.iter().find(|n| n.get("id").and_then(|i| i.as_str()) == Some(target_id));
                        
                        if let Some(next_node) = target_node {
                            let node_type = next_node.get("type").and_then(|t| t.as_str()).unwrap_or("");
                            if node_type == "user_task" {
                                let new_task = bpm_task::ActiveModel {
                                    process_instance_id: Set(instance.id),
                                    task_no: Set(format!("TSK{}{}", chrono::Local::now().format("%Y%m%d%H%M%S"), instance.id)),
                                    node_id: Set(next_node.get("id").and_then(|i| i.as_str()).unwrap_or("unknown").to_string()),
                                    node_name: Set(next_node.get("name").and_then(|n| n.as_str()).unwrap_or("Task").to_string()),
                                    name: Set(next_node.get("name").and_then(|n| n.as_str()).unwrap_or("Task").to_string()),
                                    task_type: Set("user_task".to_string()),
                                    assignee_id: Set(next_node.get("assignee_value").and_then(|a| a.as_str()).and_then(|s| s.parse::<i32>().ok())),
                                    status: Set("PENDING".to_string()),
                                    business_type: Set(task.business_type.clone()),
                                    business_id: Set(task.business_id),
                                    created_at: Set(chrono::Utc::now()),
                                    updated_at: Set(chrono::Utc::now()),
                                    ..Default::default()
                                };
                                new_task.insert(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
                                next_task_created = true;
                            } else if node_type == "end_event" {
                                // Handled below
                            }
                        }
                    }
                }
            }

            if !next_task_created {
                // No more user tasks, instance is completed
                let mut instance_active: bpm_process_instance::ActiveModel = instance.clone().into();
                instance_active.status = Set("COMPLETED".to_string());
                instance_active.end_time = Set(Some(chrono::Utc::now()));
                instance_active.updated_at = Set(chrono::Utc::now());
                instance_active.update(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
                
                if let (Some(b_type), Some(b_id)) = (instance.business_type.clone(), instance.business_id) {
                    crate::services::event_bus::EVENT_BUS.publish(crate::services::event_bus::BusinessEvent::BpmProcessFinished {
                        business_type: b_type,
                        business_id: b_id,
                        approved: true,
                    });
                }
            }
        }

        txn.commit().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    pub async fn query_user_tasks(&self, query: TaskQuery) -> Result<PageResponse<bpm_task::Model>, AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(10);
        
        let mut stmt = bpm_task::Entity::find();
        
        stmt = stmt.filter(bpm_task::Column::AssigneeId.eq(query.user_id));
        
        if let Some(status) = query.status {
            stmt = stmt.filter(bpm_task::Column::Status.eq(status));
        }
        
        let paginator = stmt.paginate(&*self.db, page_size);
        let total = paginator.num_items().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let items = paginator.fetch_page(page - 1).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        let total_pages = if total == 0 { 0 } else { total.div_ceil(page_size) };
        Ok(PageResponse {
            data: items,
            total,
            page,
            page_size,
            total_pages,
        })
    }
}