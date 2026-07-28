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
    ReassignTaskRequest, RecordContactRequest,
};
use crate::models::collection_template;
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

    /// 扫描逾期未收 ar_invoice
    async fn scan_overdue_invoices(
        txn: &sea_orm::DatabaseTransaction,
        as_of_date: chrono::NaiveDate,
    ) -> Result<Vec<ar_invoice::Model>, CollectionTaskError> {
        Ok(ar_invoice::Entity::find()
            .filter(ar_invoice::Column::UnpaidAmount.gt(Decimal::ZERO))
            .filter(ar_invoice::Column::DueDate.lt(as_of_date))
            .filter(ar_invoice::Column::ApprovalStatus.eq("approved"))
            .all(txn)
            .await?)
    }

    /// 按客户聚合逾期发票(最大逾期天数代表催收紧迫度)
    fn aggregate_by_customer(
        invoices: Vec<ar_invoice::Model>,
        as_of_date: chrono::NaiveDate,
        min_overdue_days: i32,
    ) -> std::collections::HashMap<i64, CustomerOverdueAggr> {
        let mut map: std::collections::HashMap<i64, CustomerOverdueAggr> =
            std::collections::HashMap::new();
        for inv in invoices {
            let overdue_days = (as_of_date - inv.due_date).num_days();
            if overdue_days < min_overdue_days as i64 {
                continue;
            }
            let customer_id = inv.customer_id as i64;
            let aggr = map
                .entry(customer_id)
                .or_insert_with(|| CustomerOverdueAggr {
                    customer_id,
                    ar_invoice_id: Some(inv.id),
                    total_overdue: Decimal::ZERO,
                    max_overdue_days: 0,
                });
            aggr.total_overdue += inv.unpaid_amount;
            if overdue_days > aggr.max_overdue_days {
                aggr.max_overdue_days = overdue_days;
                aggr.ar_invoice_id = Some(inv.id);
            }
        }
        map
    }

    /// 幂等检查:该客户是否已有 pending/in_progress 任务
    async fn customer_has_active_task(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        customer_id: i64,
    ) -> Result<bool, CollectionTaskError> {
        let existing = Entity::find()
            .filter(collection_task::Column::CustomerId.eq(customer_id))
            .filter(collection_task::Column::Status.is_in([
                TaskStatus::Pending.as_str(),
                TaskStatus::InProgress.as_str(),
            ]))
            .one(txn)
            .await?;
        Ok(existing.is_some())
    }

    /// 构造新建催收任务的 ActiveModel
    fn build_new_task_active(
        aggr: &CustomerOverdueAggr,
        task_no: String,
        due_date: chrono::NaiveDate,
        assigned_by: i32,
        now: chrono::DateTime<chrono::Utc>,
    ) -> ActiveModel {
        let task_type = TaskType::from_overdue_days(aggr.max_overdue_days);
        let priority = TaskPriority::from_overdue_days(aggr.max_overdue_days);
        ActiveModel {
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
        }
    }

    /// V15 P1 17.3-D5：根据 task_type + overdue_days 查询催收模板
    ///
    /// 优先级：精确匹配阶段 > all 通用模板；同阶段按 sort_order 升序取首条。
    /// 阶段映射：early(0-30天) / middle(31-90天) / late(90+天)。
    async fn find_template_for_task(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        task_type: &str,
        overdue_days: i32,
    ) -> Result<Option<collection_template::Model>, CollectionTaskError> {
        let stage = Self::stage_from_overdue_days(overdue_days);
        // 优先匹配精确阶段
        let template = collection_template::Entity::find()
            .filter(collection_template::Column::TaskType.eq(task_type))
            .filter(collection_template::Column::OverdueStage.eq(stage))
            .filter(collection_template::Column::IsEnabled.eq(true))
            .order_by_asc(collection_template::Column::SortOrder)
            .one(txn)
            .await?;
        if template.is_some() {
            return Ok(template);
        }
        // 回退到 all 通用模板
        let fallback = collection_template::Entity::find()
            .filter(collection_template::Column::TaskType.eq(task_type))
            .filter(collection_template::Column::OverdueStage.eq("all"))
            .filter(collection_template::Column::IsEnabled.eq(true))
            .order_by_asc(collection_template::Column::SortOrder)
            .one(txn)
            .await?;
        Ok(fallback)
    }

    /// V15 P1 17.3-D5：渲染模板占位符
    ///
    /// 支持占位符：{overdue_days} / {overdue_amount} / {customer_name} / {date}
    fn render_template(
        content: &str,
        overdue_days: i32,
        overdue_amount: Decimal,
        customer_name: Option<&str>,
    ) -> String {
        let mut rendered = content
            .replace("{overdue_days}", overdue_days.to_string().as_str())
            .replace(
                "{overdue_amount}",
                format!("{:.2}", overdue_amount).as_str(),
            )
            .replace("{customer_name}", customer_name.unwrap_or("客户"))
            .replace(
                "{date}",
                chrono::Utc::now()
                    .date_naive()
                    .format("%Y-%m-%d")
                    .to_string()
                    .as_str(),
            );
        rendered = rendered.replace("\\n", "\n");
        rendered
    }

    /// 根据逾期天数确定阶段：early(0-30) / middle(31-90) / late(90+)
    fn stage_from_overdue_days(days: i32) -> &'static str {
        if days <= 30 {
            "early"
        } else if days <= 90 {
            "middle"
        } else {
            "late"
        }
    }

    /// 为客户聚合结果生成催收任务(含幂等检查)
    ///
    /// V15 P1 17.3-D5：自动生成任务时，查询匹配的催收模板并渲染话术，
    /// 写入 remark 字段供催收员参考使用。
    async fn generate_tasks_for_customers(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        customer_aggr: std::collections::HashMap<i64, CustomerOverdueAggr>,
        today: chrono::NaiveDate,
        due_date: chrono::NaiveDate,
        now: chrono::DateTime<chrono::Utc>,
        assigned_by: i32,
    ) -> Result<Vec<collection_task::Model>, CollectionTaskError> {
        let mut sorted: Vec<CustomerOverdueAggr> = customer_aggr.into_values().collect();
        sorted.sort_by_key(|a| a.customer_id);
        let mut seq: u32 = 0;
        let mut created: Vec<collection_task::Model> = Vec::new();
        for aggr in sorted {
            if self.customer_has_active_task(txn, aggr.customer_id).await? {
                continue;
            }
            seq += 1;
            let task_no = format!("CT-{}-{:03}", today.format("%Y%m%d"), seq);
            let mut active =
                Self::build_new_task_active(&aggr, task_no, due_date, assigned_by, now);
            // V15 P1 17.3-D5：匹配模板并渲染话术
            let task_type = TaskType::from_overdue_days(aggr.max_overdue_days);
            if let Some(tpl) = self
                .find_template_for_task(txn, task_type.as_str(), aggr.max_overdue_days as i32)
                .await?
            {
                let rendered = Self::render_template(
                    &tpl.content,
                    aggr.max_overdue_days as i32,
                    aggr.total_overdue,
                    None,
                );
                active.remark = Set(Some(rendered));
            }
            let model = active.insert(txn).await?;
            created.push(model);
        }
        Ok(created)
    }

    /// 自动生成催收任务(扫描逾期发票,按客户聚合,幂等创建)
    pub async fn auto_generate_tasks(
        &self,
        req: AutoGenerateTasksRequest,
        assigned_by: i32,
    ) -> Result<Vec<collection_task::Model>, CollectionTaskError> {
        let min_overdue_days = req.min_overdue_days.unwrap_or(1).max(1);
        let as_of_date = req.as_of_date.unwrap_or_else(|| Utc::now().date_naive());
        let txn = (*self.db).begin().await?;
        let invoices = Self::scan_overdue_invoices(&txn, as_of_date).await?;
        let customer_aggr = Self::aggregate_by_customer(invoices, as_of_date, min_overdue_days);
        let now = Utc::now();
        let today = now.date_naive();
        let due_date = today + Duration::days(7);
        let created = self
            .generate_tasks_for_customers(&txn, customer_aggr, today, due_date, now, assigned_by)
            .await?;
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
        // V15 P1 17.3-D5：若未提供 remark，则查询匹配的催收模板并渲染话术
        let remark = if req.remark.is_some() {
            req.remark
        } else {
            let tpl = self
                .find_template_for_task(&txn, &req.task_type, req.overdue_days)
                .await?;
            tpl.map(|t| {
                Self::render_template(&t.content, req.overdue_days, req.overdue_amount, None)
            })
        };
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
            remark: Set(remark),
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
