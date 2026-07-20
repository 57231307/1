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
use crate::models::lab_dip_request;
use crate::models::process_log::{self, ActiveModel as LogActive, Entity as LogEntity};
use crate::models::process_node::{self, Entity as NodeEntity};
use crate::models::sales_quotation;
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
    /// V15 P0-B11：状态门校验失败（打样未确认 / 报价未审批等）
    #[error("状态门校验失败: {0}")]
    GateValidation(String),
    /// V15 P0-B11：关联的打样通知单不存在
    #[error("关联的打样通知单不存在: lab_dip_request_id={0}")]
    LabDipRequestNotFound(i32),
    /// V15 P0-B11：关联的报价单不存在
    #[error("关联的报价单不存在: quotation_id={0}")]
    QuotationNotFound(i64),
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
    ///
    /// V15 P0-B11：状态门校验
    /// - `lab_dip → quotation`：校验 `lab_dip_request_id` 已关联且打样通知单 `approved_sample_id IS NOT NULL`
    ///   （客户已确认 OK 样才允许进入报价阶段）
    /// - `quotation → yarn_purchasing`：校验 `quotation_id` 已关联且报价单 `status = 'approved'`
    ///   （报价审批通过才允许进入生产阶段），并自动同步 `total_amount` 从报价单到定制订单
    pub async fn advance(
        &self,
        order_id: i64,
        operator_id: i64,
        notes: Option<String>,
    ) -> Result<custom_order::Model, StateError> {
        let order = self.lookup_order(order_id).await?;
        let next = next_status(&order.status)?;
        let next_str = next.as_str().to_string();

        self.validate_gate_before_advance(&order, next).await?;

        let updated = self.update_order_status(&order, next, next_str.clone()).await?;

        self.complete_current_node(order_id, operator_id, notes.clone()).await?;
        self.start_next_node(order_id, operator_id, &next_str).await?;

        Ok(updated)
    }

    async fn lookup_order(&self, order_id: i64) -> Result<custom_order::Model, StateError> {
        Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or(StateError::NotFound)
    }

    async fn validate_gate_before_advance(
        &self,
        order: &custom_order::Model,
        next: CustomOrderStatus,
    ) -> Result<(), StateError> {
        match next {
            CustomOrderStatus::Quotation => self.validate_quotation_gate(order).await,
            CustomOrderStatus::YarnPurchasing => self.validate_yarn_purchasing_gate(order).await,
            _ => Ok(()),
        }
    }

    async fn validate_quotation_gate(&self, order: &custom_order::Model) -> Result<(), StateError> {
        let lab_dip_id = order.lab_dip_request_id.ok_or_else(|| {
            StateError::GateValidation(
                "推进到报价阶段前必须关联打样通知单（lab_dip_request_id 不能为空）".to_string(),
            )
        })?;
        let lab_dip = lab_dip_request::Entity::find_by_id(lab_dip_id)
            .one(&*self.db)
            .await?
            .ok_or(StateError::LabDipRequestNotFound(lab_dip_id))?;
        if lab_dip.approved_sample_id.is_none() {
            return Err(StateError::GateValidation(format!(
                "打样通知单 {} 未确认 OK 样（approved_sample_id 为空），禁止推进到报价阶段",
                lab_dip_id
            )));
        }
        Ok(())
    }

    async fn validate_yarn_purchasing_gate(&self, order: &custom_order::Model) -> Result<(), StateError> {
        let quotation_id = order.quotation_id.ok_or_else(|| {
            StateError::GateValidation(
                "推进到生产阶段前必须关联报价单（quotation_id 不能为空）".to_string(),
            )
        })?;
        let quotation = sales_quotation::Entity::find_by_id(quotation_id)
            .one(&*self.db)
            .await?
            .ok_or(StateError::QuotationNotFound(quotation_id))?;
        if !quotation.status.eq_ignore_ascii_case("approved") {
            return Err(StateError::GateValidation(format!(
                "报价单 {} 状态为 {}，未审批通过（approved），禁止推进到生产阶段",
                quotation_id, quotation.status
            )));
        }
        Ok(())
    }

    async fn update_order_status(
        &self,
        order: &custom_order::Model,
        next: CustomOrderStatus,
        next_str: String,
    ) -> Result<custom_order::Model, StateError> {
        let mut active: ActiveModel = order.clone().into();
        active.status = Set(next_str);
        active.updated_at = Set(Utc::now());

        if next == CustomOrderStatus::YarnPurchasing {
            if let Some(qid) = order.quotation_id {
                if let Ok(Some(quotation)) = sales_quotation::Entity::find_by_id(qid)
                    .one(&*self.db)
                    .await
                {
                    active.total_amount = Set(Some(quotation.total_amount));
                }
            }
        }

        active.update(&*self.db).await.map_err(StateError::Database)
    }

    async fn complete_current_node(
        &self,
        order_id: i64,
        operator_id: i64,
        notes: Option<String>,
    ) -> Result<(), StateError> {
        let node = NodeEntity::find()
            .filter(process_node::Column::CustomOrderId.eq(order_id))
            .filter(process_node::Column::Status.eq(node_status::IN_PROGRESS))
            .one(&*self.db)
            .await?;

        if let Some(n) = node {
            let mut n_active: process_node::ActiveModel = n.clone().into();
            n_active.status = Set(node_status::COMPLETED.to_string());
            n_active.actual_end_date = Set(Some(Utc::now()));
            n_active.updated_at = Set(Utc::now());
            n_active.update(&*self.db).await.map_err(StateError::Database)?;

            let log = LogActive {
                id: Default::default(),
                process_node_id: Set(n.id),
                action: Set("complete".to_string()),
                operator_id: Set(Some(operator_id)),
                before_status: Set(Some(node_status::IN_PROGRESS.to_string())),
                after_status: Set(Some(node_status::COMPLETED.to_string())),
                log_time: Set(Utc::now()),
                log_content: Set(notes),
                attachments: Set(serde_json::json!([])),
            };
            log.insert(&*self.db).await.map_err(StateError::Database)?;
        }

        Ok(())
    }

    async fn start_next_node(
        &self,
        order_id: i64,
        operator_id: i64,
        next_str: &str,
    ) -> Result<(), StateError> {
        let next_node = NodeEntity::find()
            .filter(process_node::Column::CustomOrderId.eq(order_id))
            .filter(process_node::Column::NodeType.eq(next_str))
            .one(&*self.db)
            .await?;

        if let Some(n) = next_node {
            let mut n_active: process_node::ActiveModel = n.into();
            n_active.status = Set(node_status::IN_PROGRESS.to_string());
            n_active.actual_start_date = Set(Some(Utc::now()));
            n_active.operator_id = Set(Some(operator_id));
            n_active.updated_at = Set(Utc::now());
            n_active.update(&*self.db).await.map_err(StateError::Database)?;
        }

        Ok(())
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
