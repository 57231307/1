//! 定制订单状态机服务
//!
//! 处理定制订单的 5 阶段工艺状态推进
//! 设计依据：docs/superpowers/specs/2026-06-16-custom-order-design.md §3.3
//! 创建时间: 2026-06-17

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait};
use std::sync::Arc;
use thiserror::Error;

use crate::models::custom_order::{self, ActiveModel, Entity};
use crate::models::process_log::{self, ActiveModel as LogActive, Entity as LogEntity};
use crate::models::process_node::{self, Entity as NodeEntity};
use crate::models::status::process_node as node_status;
use crate::utils::app_state::AppState;
use crate::utils::process_state_machine::{
    can_transition, next_status, CustomOrderStatus, StateMachineError,
};

/// 业务错误
#[derive(Debug, Error)]
pub enum StateError {
    #[error("订单不存在")]
    NotFound,
    #[error("非法状态转换: {0}")]
    InvalidTransition(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
    #[error("状态机错误: {0}")]
    StateMachine(#[from] StateMachineError),
}

/// 定制订单状态机服务
pub struct CustomOrderStateService {
    db: Arc<DatabaseConnection>,
}

impl CustomOrderStateService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 推进到下一阶段（自动判断下一状态）
    pub async fn advance(
        &self,
        order_id: i64,
        operator_id: i64,
        notes: Option<String>,
    ) -> Result<custom_order::Model, StateError> {
        let order = Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or(StateError::NotFound)?;

        // 计算下一状态
        let next = next_status(&order.status)?;
        let next_str = next.as_str().to_string();

        // 更新主表状态
        let mut active: ActiveModel = order.clone().into();
        active.status = Set(next_str.clone());
        active.updated_at = Set(Utc::now());

        // 若推进到 delivery 阶段，记录 expected_delivery_date 为 actual_delivery_date
        if next == CustomOrderStatus::Delivery {
            // 保持原值
        }

        let updated = active.update(&*self.db).await?;

        // 更新对应工艺节点状态
        let node = NodeEntity::find()
            .filter(process_node::Column::CustomOrderId.eq(order_id))
            .filter(process_node::Column::Status.eq(node_status::IN_PROGRESS))
            .one(&*self.db)
            .await?;

        if let Some(n) = node {
            // 当前 in_progress 节点标记为 completed
            let mut n_active: process_node::ActiveModel = n.clone().into();
            n_active.status = Set(node_status::COMPLETED.to_string());
            n_active.actual_end_date = Set(Some(Utc::now()));
            n_active.updated_at = Set(Utc::now());
            n_active.update(&*self.db).await?;

            // 记录日志
            let log = LogActive {
                id: Default::default(),
                process_node_id: Set(n.id),
                action: Set("complete".to_string()),
                operator_id: Set(Some(operator_id)),
                before_status: Set(Some(node_status::IN_PROGRESS.to_string())),
                after_status: Set(Some(node_status::COMPLETED.to_string())),
                log_time: Set(Utc::now()),
                log_content: Set(notes.clone()),
                attachments: Set(serde_json::json!([])),
            };
            log.insert(&*self.db).await?;
        }

        // 启动下一节点
        let next_node = NodeEntity::find()
            .filter(process_node::Column::CustomOrderId.eq(order_id))
            .filter(process_node::Column::NodeType.eq(next_str.clone()))
            .one(&*self.db)
            .await?;

        if let Some(n) = next_node {
            let mut n_active: process_node::ActiveModel = n.into();
            n_active.status = Set(node_status::IN_PROGRESS.to_string());
            n_active.actual_start_date = Set(Some(Utc::now()));
            n_active.operator_id = Set(Some(operator_id));
            n_active.updated_at = Set(Utc::now());
            n_active.update(&*self.db).await?;
        }

        Ok(updated)
    }

    /// 直接设置状态（用于取消等场景）
    pub async fn set_status(
        &self,
        order_id: i64,
        target: &str,
        operator_id: i64,
        notes: Option<String>,
    ) -> Result<custom_order::Model, StateError> {
        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现无 txn 无 lock，两并发 set_status 均通过状态检查后基于过期状态写入，
        // 导致状态机被绕过、日志记录与主表状态不一致。
        let txn = (*self.db).begin().await?;

        let order = Entity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(StateError::NotFound)?;

        if !can_transition(&order.status, target) {
            return Err(StateError::InvalidTransition(format!(
                "{} → {}",
                order.status, target
            )));
        }

        let mut active: ActiveModel = order.clone().into();
        active.status = Set(target.to_string());
        active.updated_at = Set(Utc::now());
        let updated = active.update(&txn).await?;

        // 记录日志到第一个节点（如有）
        if let Some(first_node) = NodeEntity::find()
            .filter(process_node::Column::CustomOrderId.eq(order_id))
            .order_by_asc(process_node::Column::Sequence)
            .one(&txn)
            .await?
        {
            let log = LogActive {
                id: Default::default(),
                process_node_id: Set(first_node.id),
                action: Set("status_change".to_string()),
                operator_id: Set(Some(operator_id)),
                before_status: Set(Some(order.status.clone())),
                after_status: Set(Some(target.to_string())),
                log_time: Set(Utc::now()),
                log_content: Set(notes),
                attachments: Set(serde_json::json!([])),
            };
            log.insert(&txn).await?;
        }

        txn.commit().await?;
        Ok(updated)
    }

    /// 列出指定订单的工艺日志
    pub async fn list_logs(
        &self,
        order_id: i64,
    ) -> Result<Vec<process_log::Model>, StateError> {
        // 获取所有节点
        let nodes = NodeEntity::find()
            .filter(process_node::Column::CustomOrderId.eq(order_id))
            .all(&*self.db)
            .await?;

        let node_ids: Vec<i64> = nodes.iter().map(|n| n.id).collect();
        if node_ids.is_empty() {
            return Ok(vec![]);
        }

        let logs = LogEntity::find()
            .filter(process_log::Column::ProcessNodeId.is_in(node_ids))
            .all(&*self.db)
            .await?;
        Ok(logs)
    }
}
