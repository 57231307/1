//! 催收任务服务（V15 P0-B03 Batch 481 创建）
//!
//! 业务流程：
//! - 期末扫描逾期 ar_invoice，自动生成催收任务（按客户聚合）
//! - 4 种催收方式：phone(电话) / visit(上门) / email(邮件) / letter(函件)
//! - 优先级按逾期天数自动评估：< 30 天 normal / 30-90 天 high / > 90 天 urgent
//! - 状态机：pending → in_progress → completed / cancelled
//!
//! 关联任务：P0-B03（§17.3-D3）
//! 关联文件：models/collection_task.rs / models/collection_task_dto.rs /
//!          handlers/collection_task_handler.rs / routes/collection_task.rs

use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use std::sync::Arc;
use thiserror::Error;

use crate::models::ar_invoice;
use crate::models::collection_task::{self, ActiveModel, Entity};
use crate::models::collection_task_dto::{
    AutoGenerateTasksRequest, CancelTaskRequest, CreateTaskRequest, ListTaskQuery,
    RecordContactRequest, ReassignTaskRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

/// 业务错误
#[derive(Debug, Error)]
pub enum CollectionTaskError {
    #[error("催收任务不存在")]
    NotFound,
    #[error("当前状态 {current} 不允许此操作（期望 {expected}）")]
    InvalidState {
        current: String,
        expected: &'static str,
    },
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
    /// paginate_with_total 返回 AppError，透传所需
    #[error("应用错误: {0}")]
    App(#[from] AppError),
}

/// 任务状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// 任务类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    Phone,
    Visit,
    Email,
    Letter,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Phone => "phone",
            Self::Visit => "visit",
            Self::Email => "email",
            Self::Letter => "letter",
        }
    }

    /// 根据逾期天数自动选择催收方式
    /// < 30 天：phone / 30-90 天：visit / > 90 天：letter
    pub fn from_overdue_days(days: i64) -> Self {
        if days < 30 {
            Self::Phone
        } else if days <= 90 {
            Self::Visit
        } else {
            Self::Letter
        }
    }
}

/// 优先级枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Urgent,
}

impl TaskPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }

    /// 根据逾期天数自动评估优先级
    /// < 30 天：normal / 30-90 天：high / > 90 天：urgent
    pub fn from_overdue_days(days: i64) -> Self {
        if days < 30 {
            Self::Normal
        } else if days <= 90 {
            Self::High
        } else {
            Self::Urgent
        }
    }
}

/// 催收任务服务
pub struct CollectionTaskService {
    db: Arc<DatabaseConnection>,
}

impl CollectionTaskService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self::new(state.db.clone())
    }

    /// 自动生成催收任务（B03）
    ///
    /// 业务规则：
    /// 1. 扫描 due_date < as_of_date 且 unpaid_amount > 0 的 ar_invoice
    /// 2. 逾期天数 >= min_overdue_days（默认 1）
    /// 3. 按客户聚合，每客户生成一条任务
    /// 4. 已存在该客户的 pending/in_progress 任务则跳过（避免重复催收）
    /// 5. 优先级和催收方式按逾期天数自动评估
    pub async fn auto_generate_tasks(
        &self,
        req: AutoGenerateTasksRequest,
        assigned_by: i32,
    ) -> Result<Vec<collection_task::Model>, CollectionTaskError> {
        let min_overdue_days = req.min_overdue_days.unwrap_or(1).max(1);
        let as_of_date = req.as_of_date.unwrap_or_else(|| Utc::now().date_naive());

        let txn = (*self.db).begin().await?;

        // 扫描逾期未收 ar_invoice
        let invoices = ar_invoice::Entity::find()
            .filter(ar_invoice::Column::UnpaidAmount.gt(Decimal::ZERO))
            .filter(ar_invoice::Column::DueDate.lt(as_of_date))
            .filter(ar_invoice::Column::ApprovalStatus.eq("approved"))
            .all(&txn)
            .await?;

        // 按客户聚合（最大逾期天数代表催收紧迫度）
        let mut customer_aggr: std::collections::HashMap<i64, CustomerOverdueAggr> =
            std::collections::HashMap::new();

        for inv in invoices {
            let overdue_days = (as_of_date - inv.due_date).num_days();
            if overdue_days < min_overdue_days as i64 {
                continue;
            }
            let customer_id = inv.customer_id as i64;
            let aggr = customer_aggr.entry(customer_id).or_insert_with(|| {
                CustomerOverdueAggr {
                    customer_id,
                    ar_invoice_id: Some(inv.id),
                    total_overdue: Decimal::ZERO,
                    max_overdue_days: 0,
                }
            });
            aggr.total_overdue += inv.unpaid_amount;
            if overdue_days > aggr.max_overdue_days {
                aggr.max_overdue_days = overdue_days;
                aggr.ar_invoice_id = Some(inv.id);
            }
        }

        let now = Utc::now();
        let today = now.date_naive();
        let due_date = today + Duration::days(7);
        let mut created: Vec<collection_task::Model> = Vec::new();

        // 按客户 ID 升序生成任务号，保证可预测
        let mut sorted: Vec<CustomerOverdueAggr> = customer_aggr.into_values().collect();
        sorted.sort_by_key(|a| a.customer_id);

        // 单次扫描日期内序号从 1 开始递增（CT-YYYYMMDD-NNN）
        let mut seq: u32 = 0;
        for aggr in sorted {
            // 幂等检查：该客户已有 pending/in_progress 任务则跳过
            let existing = Entity::find()
                .filter(collection_task::Column::CustomerId.eq(aggr.customer_id))
                .filter(
                    collection_task::Column::Status
                        .is_in([TaskStatus::Pending.as_str(), TaskStatus::InProgress.as_str()]),
                )
                .one(&txn)
                .await?;
            if existing.is_some() {
                continue;
            }

            seq += 1;
            let task_no = format!("CT-{}-{:03}", today.format("%Y%m%d"), seq);
            let task_type = TaskType::from_overdue_days(aggr.max_overdue_days);
            let priority = TaskPriority::from_overdue_days(aggr.max_overdue_days);

            let active = ActiveModel {
                id: Default::default(),
                task_no: Set(task_no),
                customer_id: Set(aggr.customer_id),
                ar_invoice_id: Set(aggr.ar_invoice_id),
                overdue_amount: Set(aggr.total_overdue),
                overdue_days: Set(aggr.max_overdue_days as i32),
                task_type: Set(task_type.as_str().to_string()),
                priority: Set(priority.as_str().to_string()),
                due_date: Set(due_date),
                assigned_to: Set(assigned_by),
                assigned_at: Set(now),
                assigned_by: Set(Some(assigned_by)),
                status: Set(TaskStatus::Pending.as_str().to_string()),
                contact_result: Set(None),
                contact_at: Set(None),
                next_action_date: Set(None),
                next_action_type: Set(None),
                remark: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            };
            let model = active.insert(&txn).await?;
            created.push(model);
        }

        txn.commit().await?;
        Ok(created)
    }

    /// 手动创建催收任务
    pub async fn create_task(
        &self,
        req: CreateTaskRequest,
        assigned_by: i32,
    ) -> Result<collection_task::Model, CollectionTaskError> {
        // 校验 task_type
        if !["phone", "visit", "email", "letter"].contains(&req.task_type.as_str()) {
            return Err(CollectionTaskError::Validation(format!(
                "非法 task_type: {}，合法值：phone/visit/email/letter",
                req.task_type
            )));
        }
        // 校验 priority
        let priority = req.priority.as_deref().unwrap_or("normal");
        if !["low", "normal", "high", "urgent"].contains(&priority) {
            return Err(CollectionTaskError::Validation(format!(
                "非法 priority: {}，合法值：low/normal/high/urgent",
                priority
            )));
        }
        if req.overdue_amount < Decimal::ZERO {
            return Err(CollectionTaskError::Validation(
                "overdue_amount 不能为负".to_string(),
            ));
        }

        let txn = (*self.db).begin().await?;

        // 生成任务号：CT-YYYYMMDD-NNN（基于当日已有任务数 + 1）
        let today = Utc::now().date_naive();
        let prefix = format!("CT-{}-", today.format("%Y%m%d"));
        let count_today = Entity::find()
            .filter(collection_task::Column::TaskNo.starts_with(&prefix))
            .count(&txn)
            .await?;
        let task_no = format!("CT-{}-{:03}", today.format("%Y%m%d"), count_today + 1);

        let now = Utc::now();
        let active = ActiveModel {
            id: Default::default(),
            task_no: Set(task_no),
            customer_id: Set(req.customer_id),
            ar_invoice_id: Set(req.ar_invoice_id),
            overdue_amount: Set(req.overdue_amount),
            overdue_days: Set(req.overdue_days),
            task_type: Set(req.task_type),
            priority: Set(priority.to_string()),
            due_date: Set(req.due_date),
            assigned_to: Set(req.assigned_to),
            assigned_at: Set(now),
            assigned_by: Set(Some(assigned_by)),
            status: Set(TaskStatus::Pending.as_str().to_string()),
            contact_result: Set(None),
            contact_at: Set(None),
            next_action_date: Set(None),
            next_action_type: Set(None),
            remark: Set(req.remark),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let model = active.insert(&txn).await?;
        txn.commit().await?;
        Ok(model)
    }

    /// 记录催收结果
    ///
    /// 状态流转：
    /// - pending → in_progress（首次记录）
    /// - in_progress → in_progress（追加记录）
    /// - mark_completed=true 时 → completed（终态）
    pub async fn record_contact(
        &self,
        task_id: i64,
        req: RecordContactRequest,
    ) -> Result<collection_task::Model, CollectionTaskError> {
        if req.contact_result.trim().is_empty() {
            return Err(CollectionTaskError::Validation(
                "contact_result 不能为空".to_string(),
            ));
        }

        let txn = (*self.db).begin().await?;
        let existing = Entity::find_by_id(task_id)
            .one(&txn)
            .await?
            .ok_or(CollectionTaskError::NotFound)?;

        // 仅 pending/in_progress 状态可记录
        if !["pending", "in_progress"].contains(&existing.status.as_str()) {
            return Err(CollectionTaskError::InvalidState {
                current: existing.status,
                expected: "pending 或 in_progress",
            });
        }

        let now = Utc::now();
        let mark_completed = req.mark_completed.unwrap_or(false);
        let new_status = if mark_completed {
            TaskStatus::Completed.as_str().to_string()
        } else {
            TaskStatus::InProgress.as_str().to_string()
        };

        let mut active: ActiveModel = existing.into();
        active.status = Set(new_status);
        active.contact_result = Set(Some(req.contact_result));
        active.contact_at = Set(Some(now));
        active.next_action_date = Set(req.next_action_date);
        active.next_action_type = Set(req.next_action_type);
        if let Some(remark) = req.remark {
            active.remark = Set(Some(remark));
        }
        active.updated_at = Set(now);
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 重新分配任务（pending 或 in_progress 状态可重新分配）
    pub async fn reassign(
        &self,
        task_id: i64,
        req: ReassignTaskRequest,
    ) -> Result<collection_task::Model, CollectionTaskError> {
        let txn = (*self.db).begin().await?;
        let existing = Entity::find_by_id(task_id)
            .one(&txn)
            .await?
            .ok_or(CollectionTaskError::NotFound)?;

        if !["pending", "in_progress"].contains(&existing.status.as_str()) {
            return Err(CollectionTaskError::InvalidState {
                current: existing.status,
                expected: "pending 或 in_progress",
            });
        }

        let now = Utc::now();
        let mut active: ActiveModel = existing.into();
        active.assigned_to = Set(req.assigned_to);
        active.assigned_at = Set(now);
        if let Some(remark) = req.remark {
            active.remark = Set(Some(remark));
        }
        active.updated_at = Set(now);
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 取消任务（pending 或 in_progress → cancelled）
    pub async fn cancel(
        &self,
        task_id: i64,
        req: CancelTaskRequest,
    ) -> Result<collection_task::Model, CollectionTaskError> {
        if req.cancel_reason.trim().is_empty() {
            return Err(CollectionTaskError::Validation(
                "cancel_reason 不能为空".to_string(),
            ));
        }

        let txn = (*self.db).begin().await?;
        let existing = Entity::find_by_id(task_id)
            .one(&txn)
            .await?
            .ok_or(CollectionTaskError::NotFound)?;

        if !["pending", "in_progress"].contains(&existing.status.as_str()) {
            return Err(CollectionTaskError::InvalidState {
                current: existing.status,
                expected: "pending 或 in_progress",
            });
        }

        let now = Utc::now();
        let mut active: ActiveModel = existing.into();
        active.status = Set(TaskStatus::Cancelled.as_str().to_string());
        active.remark = Set(Some(format!("取消原因：{}", req.cancel_reason)));
        active.updated_at = Set(now);
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 按 ID 查询任务
    pub async fn get_task(
        &self,
        task_id: i64,
    ) -> Result<collection_task::Model, CollectionTaskError> {
        Entity::find_by_id(task_id)
            .one(&*self.db)
            .await?
            .ok_or(CollectionTaskError::NotFound)
    }

    /// 列表查询
    pub async fn list_tasks(
        &self,
        query: ListTaskQuery,
    ) -> Result<(Vec<collection_task::Model>, u64), CollectionTaskError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let mut select = Entity::find();
        if let Some(v) = query.customer_id {
            select = select.filter(collection_task::Column::CustomerId.eq(v));
        }
        if let Some(v) = query.ar_invoice_id {
            select = select.filter(collection_task::Column::ArInvoiceId.eq(v));
        }
        if let Some(v) = query.assigned_to {
            select = select.filter(collection_task::Column::AssignedTo.eq(v));
        }
        if let Some(v) = query.status {
            if !["pending", "in_progress", "completed", "cancelled"].contains(&v.as_str()) {
                return Err(CollectionTaskError::Validation(format!(
                    "非法 status: {}",
                    v
                )));
            }
            select = select.filter(collection_task::Column::Status.eq(v));
        }
        if let Some(v) = query.priority {
            if !["low", "normal", "high", "urgent"].contains(&v.as_str()) {
                return Err(CollectionTaskError::Validation(format!(
                    "非法 priority: {}",
                    v
                )));
            }
            select = select.filter(collection_task::Column::Priority.eq(v));
        }
        if let Some(v) = query.task_type {
            if !["phone", "visit", "email", "letter"].contains(&v.as_str()) {
                return Err(CollectionTaskError::Validation(format!(
                    "非法 task_type: {}",
                    v
                )));
            }
            select = select.filter(collection_task::Column::TaskType.eq(v));
        }
        if query.overdue_only.unwrap_or(false) {
            // due_date < today 视为逾期未处理
            let today = Utc::now().date_naive();
            select = select.filter(collection_task::Column::DueDate.lt(today));
        }

        let paginator = select
            .order_by_desc(collection_task::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
        Ok((items, total))
    }
}

/// 客户逾期聚合（内部辅助结构）
struct CustomerOverdueAggr {
    customer_id: i64,
    ar_invoice_id: Option<i32>,
    total_overdue: Decimal,
    max_overdue_days: i64,
}
