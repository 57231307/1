//! 工艺流程推进服务
//!
//! 处理工艺节点的状态推进（独立于订单级别状态机）
//! 创建时间: 2026-06-17

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, Set};
use std::sync::Arc;
use thiserror::Error;

use crate::models::custom_order_update_dto::{
    AddProcessLogDto, AdvanceNodeDto, CreateProcessNodeDto, UpdateProcessNodeDto,
};
use crate::models::process_log::{self, ActiveModel as LogActive, Entity as LogEntity};
use crate::models::process_node::{self, ActiveModel as NodeActive, Entity as NodeEntity};
use crate::utils::app_state::AppState;

/// 业务错误
#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("节点不存在")]
    NotFound,
    #[error("非法状态: {0}")]
    InvalidState(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
}

/// 工艺流程推进服务
pub struct CustomOrderProcessService {
    db: Arc<DatabaseConnection>,
}

impl CustomOrderProcessService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 添加工艺节点
    pub async fn add_node(
        &self,
        custom_order_id: i64,
        tenant_id: i64,
        dto: CreateProcessNodeDto,
    ) -> Result<process_node::Model, ProcessError> {
        let now = Utc::now();
        let active = NodeActive {
            id: Default::default(),
            custom_order_id: Set(custom_order_id),
            node_type: Set(dto.node_type),
            node_name: Set(dto.node_name),
            sequence: Set(dto.sequence),
            status: Set("pending".to_string()),
            planned_start_date: Set(dto.planned_start_date),
            planned_end_date: Set(dto.planned_end_date),
            actual_start_date: Set(None),
            actual_end_date: Set(None),
            operator_id: Set(None),
            notes: Set(None),
            tenant_id: Set(tenant_id),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let result = active.insert(&*self.db).await?;
        Ok(result)
    }

    /// 更新工艺节点
    pub async fn update_node(
        &self,
        node_id: i64,
        tenant_id: i64,
        dto: UpdateProcessNodeDto,
    ) -> Result<process_node::Model, ProcessError> {
        let existing = NodeEntity::find_by_id(node_id)
            .filter(process_node::Column::TenantId.eq(tenant_id))
            .one(&*self.db)
            .await?
            .ok_or(ProcessError::NotFound)?;

        let mut active: NodeActive = existing.into();
        if let Some(v) = dto.status {
            active.status = Set(v);
        }
        if let Some(v) = dto.operator_id {
            active.operator_id = Set(Some(v));
        }
        if let Some(v) = dto.actual_start_date {
            active.actual_start_date = Set(Some(v));
        }
        if let Some(v) = dto.actual_end_date {
            active.actual_end_date = Set(Some(v));
        }
        if let Some(v) = dto.notes {
            active.notes = Set(Some(v));
        }
        active.updated_at = Set(Utc::now());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 推进工艺节点（start / pause / resume / complete / block）
    pub async fn advance_node(
        &self,
        node_id: i64,
        tenant_id: i64,
        dto: AdvanceNodeDto,
    ) -> Result<process_node::Model, ProcessError> {
        let existing = NodeEntity::find_by_id(node_id)
            .filter(process_node::Column::TenantId.eq(tenant_id))
            .one(&*self.db)
            .await?
            .ok_or(ProcessError::NotFound)?;

        let new_status = match dto.action.as_str() {
            "start" => "in_progress",
            "pause" => "pending",
            "resume" => "in_progress",
            "complete" => "completed",
            "block" => "blocked",
            "unblock" => "in_progress",
            _ => {
                return Err(ProcessError::InvalidState(format!(
                    "不支持的操作: {}",
                    dto.action
                )));
            }
        };

        let mut active: NodeActive = existing.clone().into();
        active.status = Set(new_status.to_string());
        active.operator_id = Set(Some(dto.operator_id));
        active.updated_at = Set(Utc::now());

        let now = Utc::now();
        match dto.action.as_str() {
            "start" | "resume" => {
                if active.actual_start_date.is_set() == false {
                    active.actual_start_date = Set(Some(now));
                }
            }
            "complete" => {
                active.actual_end_date = Set(Some(now));
            }
            _ => {}
        }

        if let Some(ref n) = dto.notes {
            active.notes = Set(Some(n.clone()));
        }

        let updated = active.update(&*self.db).await?;

        // 记录日志
        let log = LogActive {
            id: Default::default(),
            process_node_id: Set(node_id),
            action: Set(dto.action),
            operator_id: Set(Some(dto.operator_id)),
            before_status: Set(Some(existing.status)),
            after_status: Set(Some(new_status.to_string())),
            log_time: Set(Utc::now()),
            log_content: Set(dto.notes),
            attachments: Set(serde_json::json!(dto.attachments.unwrap_or_default())),
            tenant_id: Set(tenant_id),
        };
        log.insert(&*self.db).await?;

        Ok(updated)
    }

    /// 添加工艺日志
    pub async fn add_log(
        &self,
        node_id: i64,
        tenant_id: i64,
        dto: AddProcessLogDto,
    ) -> Result<process_log::Model, ProcessError> {
        let active = LogActive {
            id: Default::default(),
            process_node_id: Set(node_id),
            action: Set(dto.action),
            operator_id: Set(Some(dto.operator_id)),
            before_status: Set(dto.before_status),
            after_status: Set(dto.after_status),
            log_time: Set(Utc::now()),
            log_content: Set(dto.log_content),
            attachments: Set(serde_json::json!(dto.attachments.unwrap_or_default())),
            tenant_id: Set(tenant_id),
        };
        let result = active.insert(&*self.db).await?;
        Ok(result)
    }

    /// 列出节点日志
    pub async fn list_node_logs(
        &self,
        node_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<process_log::Model>, ProcessError> {
        let logs = LogEntity::find()
            .filter(process_log::Column::ProcessNodeId.eq(node_id))
            .filter(process_log::Column::TenantId.eq(tenant_id))
            .order_by_desc(process_log::Column::LogTime)
            .all(&*self.db)
            .await?;
        Ok(logs)
    }

    /// 获取完整时间线（节点 + 日志合并）
    pub async fn get_timeline(
        &self,
        order_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<(process_node::Model, Vec<process_log::Model>)>, ProcessError> {
        let nodes = NodeEntity::find()
            .filter(process_node::Column::CustomOrderId.eq(order_id))
            .filter(process_node::Column::TenantId.eq(tenant_id))
            .order_by_asc(process_node::Column::Sequence)
            .all(&*self.db)
            .await?;

        let mut result = Vec::new();
        for n in nodes {
            let logs = LogEntity::find()
                .filter(process_log::Column::ProcessNodeId.eq(n.id))
                .order_by_desc(process_log::Column::LogTime)
                .all(&*self.db)
                .await?;
            result.push((n, logs));
        }
        Ok(result)
    }
}
