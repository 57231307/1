//! 色卡发放管理服务（V15 P0-F04 创建）
//!
//! 替代旧 color_card_borrow_service（已废弃）
//! 业务：发放 / 归还 / 遗失 / 损坏 / 取消
//! 状态机：issued → returned / lost / damaged / cancelled（终态不可再转换）
//! 5 道发放前闸门校验（P0-F08）：
//!   1. 卡片状态 = active
//!   2. 发放数量 > 0（库存数量 >= 发放数量，色卡单张发放）
//!   3. 客户信用额度 > 0（未超额）
//!   4. 客户无未归还超期记录
//!   5. 客户状态 = active（白名单校验）

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::Serialize;
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;

use crate::models::audit_log::{OperationType, Severity};
use crate::models::color_card::{self, Entity as ColorCardEntity};
use crate::models::color_card_issue::{self, ActiveModel as IssueActive, Entity as IssueEntity};
use crate::models::customer::Entity as CustomerEntity;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::utils::app_state::AppState;

/// 业务错误
#[derive(Debug, Error)]
pub enum IssueError {
    #[error("色卡不存在")]
    ColorCardNotFound,
    #[error("客户不存在")]
    CustomerNotFound,
    #[error("发放记录不存在")]
    RecordNotFound,
    #[error("色卡当前状态不允许此操作")]
    InvalidState(String),
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("闸门校验失败: {0}")]
    GateCheckFailed(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
}

/// 发放记录状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueStatus {
    /// 发放中
    Issued,
    /// 已归还
    Returned,
    /// 已遗失
    Lost,
    /// 已损坏
    Damaged,
    /// 已取消
    Cancelled,
}

impl IssueStatus {
    /// 序列化为字符串（持久化到数据库的稳定字符串）
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Issued => "issued",
            Self::Returned => "returned",
            Self::Lost => "lost",
            Self::Damaged => "damaged",
            Self::Cancelled => "cancelled",
        }
    }

    /// 是否为终态（终态不可再转换为其它状态）
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Returned | Self::Lost | Self::Damaged | Self::Cancelled
        )
    }
}

/// IssueStatus 解析错误
#[derive(Debug, Clone)]
pub struct IssueStatusParseError(pub String);

impl std::fmt::Display for IssueStatusParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IssueStatus 解析失败: {}", self.0)
    }
}

impl std::error::Error for IssueStatusParseError {}

impl FromStr for IssueStatus {
    type Err = IssueStatusParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "issued" => Ok(Self::Issued),
            "returned" => Ok(Self::Returned),
            "lost" => Ok(Self::Lost),
            "damaged" => Ok(Self::Damaged),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(IssueStatusParseError(s.to_string())),
        }
    }
}

/// 发放记录查询参数
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct ListIssuesQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub color_card_id: Option<i64>,
    pub customer_id: Option<i64>,
    pub status: Option<String>,
    pub from_date: Option<chrono::DateTime<Utc>>,
    pub to_date: Option<chrono::DateTime<Utc>>,
    /// V15 P1 缺陷 10.3-1：按销售订单 ID 过滤（订单驱动发放色卡场景）
    pub sales_order_id: Option<i64>,
}

/// 发放参数（封装以避免 clippy::too_many_arguments，由 handler 从 DTO 转换而来）
#[derive(Debug, Clone)]
pub struct IssueParams {
    pub color_card_id: i64,
    pub customer_id: i64,
    pub issued_by: i64,
    pub issue_qty: i32,
    pub expected_return_date: Option<chrono::NaiveDate>,
    pub purpose: Option<String>,
    pub remark: Option<String>,
    pub dye_lot_no: Option<String>,
    /// V15 P1 缺陷 10.3-1：关联销售订单 ID（订单驱动发放色卡场景）
    pub sales_order_id: Option<i64>,
}

/// V15 P1 缺陷 10.2-4：客户专属色卡库视图（status=issued，含发放记录关键信息）
#[derive(Debug, Clone, Serialize)]
pub struct CustomerColorCardView {
    pub issue_id: i64,
    pub color_card_id: i64,
    pub color_card_code: String,
    pub color_card_name: String,
    pub customer_id: i64,
    pub issue_qty: i32,
    pub issued_at: chrono::DateTime<Utc>,
    pub expected_return_date: Option<chrono::NaiveDate>,
    pub dye_lot_no: Option<String>,
    pub status: String,
}

/// V15 P1 缺陷 10.3-2：复购同缸号查询结果（含客户历史发放色卡 + 同缸号当前库存可用量）
#[derive(Debug, Clone, Serialize)]
pub struct ReorderDyeLotView {
    pub color_card_id: i64,
    pub color_card_code: String,
    pub color_card_name: String,
    pub dye_lot_no: String,
    pub last_issued_at: chrono::DateTime<Utc>,
    pub stock_quantity: i32,
    pub issued_quantity: i32,
    pub available_quantity: i32,
}

/// 发放管理服务
pub struct ColorCardIssueService {
    db: Arc<DatabaseConnection>,
}

impl ColorCardIssueService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self::new(state.db.clone())
    }

    /// V15 P0-F08：5 道发放前闸门校验（任一失败即拒绝：active/qty>0/信用未超额/无超期/客户active）
    async fn validate_issue_gates(
        &self,
        color_card_id: i64,
        customer_id: i64,
        issue_qty: i32,
        expected_return_date: Option<chrono::NaiveDate>,
    ) -> Result<(), IssueError> {
        self.check_card_status_and_stock(color_card_id, issue_qty)
            .await?;
        self.check_customer_credit_and_status(customer_id).await?;
        self.check_customer_overdue(customer_id).await?;
        Self::validate_expected_return_date(expected_return_date)?;
        Ok(())
    }

    /// 闸门 1+2：校验色卡状态为 active 且库存充足
    async fn check_card_status_and_stock(
        &self,
        color_card_id: i64,
        issue_qty: i32,
    ) -> Result<(), IssueError> {
        let card = ColorCardEntity::find_by_id(color_card_id)
            .one(&*self.db)
            .await?
            .ok_or(IssueError::ColorCardNotFound)?;
        if card.status != "active" {
            return Err(IssueError::GateCheckFailed(format!(
                "闸门 1 失败：色卡当前状态为 {}，非 active，不允许发放",
                card.status
            )));
        }
        if issue_qty <= 0 {
            return Err(IssueError::GateCheckFailed(
                "闸门 2 失败：发放数量必须 > 0".to_string(),
            ));
        }
        // V15 P0-F10：库存充足校验（可用 = stock - issued）
        let available = card.stock_quantity - card.issued_quantity;
        if available < issue_qty {
            return Err(IssueError::GateCheckFailed(format!(
                "闸门 2 失败：库存不足，可用 {} 张，本次发放 {} 张",
                available, issue_qty
            )));
        }
        Ok(())
    }

    /// 闸门 3+5：校验客户信用额度 > 0 且状态为 active
    async fn check_customer_credit_and_status(&self, customer_id: i64) -> Result<(), IssueError> {
        let customer = CustomerEntity::find_by_id(customer_id as i32)
            .one(&*self.db)
            .await?
            .ok_or(IssueError::CustomerNotFound)?;
        if customer.credit_limit <= Decimal::ZERO {
            return Err(IssueError::GateCheckFailed(
                "闸门 3 失败：客户信用额度 <= 0，不允许发放".to_string(),
            ));
        }
        if customer.status != "active" {
            return Err(IssueError::GateCheckFailed(format!(
                "闸门 5 失败：客户状态为 {}，非 active，不在白名单",
                customer.status
            )));
        }
        Ok(())
    }

    /// 闸门 4：校验客户无未归还超期记录
    async fn check_customer_overdue(&self, customer_id: i64) -> Result<(), IssueError> {
        let today = Utc::now().date_naive();
        let overdue_count = IssueEntity::find()
            .filter(color_card_issue::Column::CustomerId.eq(customer_id))
            .filter(color_card_issue::Column::Status.eq("issued"))
            .filter(color_card_issue::Column::IsDeleted.eq(false))
            .filter(
                Condition::all().add(
                    color_card_issue::Column::ExpectedReturnDate
                        .lt(today)
                        .and(color_card_issue::Column::ExpectedReturnDate.is_not_null()),
                ),
            )
            .count(&*self.db)
            .await?;
        if overdue_count > 0 {
            return Err(IssueError::GateCheckFailed(format!(
                "闸门 4 失败：客户有 {} 条未归还超期记录，不允许发放",
                overdue_count
            )));
        }
        Ok(())
    }

    /// 校验预计归还时间：不早于今天且不超过发放时间 + 30 天
    fn validate_expected_return_date(
        expected: Option<chrono::NaiveDate>,
    ) -> Result<(), IssueError> {
        if let Some(expected) = expected {
            let today = Utc::now().date_naive();
            if expected < today {
                return Err(IssueError::Validation(
                    "预计归还时间不能早于今天".to_string(),
                ));
            }
            let max_expected = today + chrono::Duration::days(30);
            if expected > max_expected {
                return Err(IssueError::Validation(
                    "预计归还时间不能超过发放时间 + 30 天".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// 创建发放记录（5 道闸门校验通过后，事务内插入记录并扣减色卡库存，V15 P0-F10）
    pub async fn issue(&self, params: IssueParams) -> Result<color_card_issue::Model, IssueError> {
        // 5 道闸门校验（校验阶段不持锁）
        self.validate_issue_gates(
            params.color_card_id,
            params.customer_id,
            params.issue_qty,
            params.expected_return_date,
        )
        .await?;

        // V15 P0-F10：事务 + 行锁，串行化并发库存扣减，避免超量发放
        let txn = self.db.begin().await?;

        // 锁定色卡行，重检库存（避免并发超额）
        let card = ColorCardEntity::find_by_id(params.color_card_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(IssueError::ColorCardNotFound)?;
        let available = card.stock_quantity - card.issued_quantity;
        if available < params.issue_qty {
            return Err(IssueError::GateCheckFailed(format!(
                "并发发放冲突：库存不足，可用 {} 张，本次发放 {} 张",
                available, params.issue_qty
            )));
        }

        let now = Utc::now();
        let active = IssueActive {
            id: Default::default(),
            color_card_id: Set(params.color_card_id),
            customer_id: Set(params.customer_id),
            issue_qty: Set(params.issue_qty),
            issued_by: Set(params.issued_by),
            issued_at: Set(now),
            expected_return_date: Set(params.expected_return_date),
            actual_return_date: Set(None),
            status: Set(IssueStatus::Issued.as_str().to_string()),
            purpose: Set(params.purpose),
            remark: Set(params.remark),
            compensation_amount: Set(None),
            returned_by: Set(None),
            dye_lot_no: Set(params.dye_lot_no),
            sales_order_id: Set(params.sales_order_id),
            created_at: Set(now),
            updated_at: Set(now),
            is_deleted: Set(false),
        };
        let result = active.insert(&txn).await?;

        // 扣减库存：issued_quantity += issue_qty（stock_quantity 不变）
        let new_issued = card.issued_quantity + params.issue_qty;
        let mut card_active: color_card::ActiveModel = card.into();
        card_active.issued_quantity = Set(new_issued);
        card_active.updated_at = Set(now);
        card_active.update(&txn).await?;

        txn.commit().await?;

        // V15 Batch05-P1-3：发布 ColorCardIssued 事件（色卡库存扣减/过期回收/客户对色反馈）
        // 严格在事务 commit 之后发布，避免事件先于数据持久化被消费端读到
        crate::services::event_bus::EVENT_BUS.publish(
            crate::services::event_bus::BusinessEvent::ColorCardIssued {
                issue_id: result.id as i32,
                color_card_id: result.color_card_id as i32,
                customer_id: Some(result.customer_id as i32),
                issued_by: Some(result.issued_by as i32),
                issued_at: result.issued_at,
            },
        );

        Ok(result)
    }

    /// 归还色卡（V15 P0-F10：归还后 issued_quantity -= issue_qty，库存恢复可用）
    pub async fn return_card(
        &self,
        record_id: i64,
        returned_by: i64,
        actual_return_date: Option<chrono::NaiveDate>,
        remark: Option<String>,
    ) -> Result<color_card_issue::Model, IssueError> {
        // 事务 + 行锁，串行化并发状态变更
        let txn = self.db.begin().await?;

        let existing = IssueEntity::find_by_id(record_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(IssueError::RecordNotFound)?;

        let current = IssueStatus::from_str(&existing.status)
            .map_err(|_| IssueError::InvalidState(format!("未知状态: {}", existing.status)))?;
        if current.is_terminal() {
            return Err(IssueError::InvalidState(format!(
                "当前状态 {} 不允许归还操作",
                existing.status
            )));
        }

        let now = Utc::now();
        let mut active: IssueActive = existing.clone().into();
        active.status = Set(IssueStatus::Returned.as_str().to_string());
        active.actual_return_date =
            Set(Some(actual_return_date.unwrap_or_else(|| now.date_naive())));
        active.returned_by = Set(Some(returned_by));
        if let Some(r) = remark {
            active.remark = Set(Some(r));
        }
        active.updated_at = Set(now);
        let result = active.update(&txn).await?;

        // V15 P0-F10：恢复色卡 issued_quantity（ -= issue_qty）
        let card = ColorCardEntity::find_by_id(existing.color_card_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(IssueError::ColorCardNotFound)?;
        let new_issued = (card.issued_quantity - existing.issue_qty).max(0);
        let mut card_active: color_card::ActiveModel = card.into();
        card_active.issued_quantity = Set(new_issued);
        card_active.updated_at = Set(now);
        card_active.update(&txn).await?;

        txn.commit().await?;
        Ok(result)
    }

    /// 登记遗失（含赔付金额，V15 P0-F10：issued_quantity -= issue_qty，色卡状态变 lost 不可再用）
    pub async fn mark_lost(
        &self,
        record_id: i64,
        compensation_amount: Decimal,
        remark: Option<String>,
    ) -> Result<color_card_issue::Model, IssueError> {
        if compensation_amount <= Decimal::ZERO {
            return Err(IssueError::Validation("赔付金额必须 > 0".to_string()));
        }

        let txn = self.db.begin().await?;

        let existing = IssueEntity::find_by_id(record_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(IssueError::RecordNotFound)?;
        let current = IssueStatus::from_str(&existing.status)
            .map_err(|_| IssueError::InvalidState(format!("未知状态: {}", existing.status)))?;
        if current.is_terminal() {
            return Err(IssueError::InvalidState(format!(
                "当前状态 {} 不允许登记遗失",
                existing.status
            )));
        }

        let now = Utc::now();
        // 1. 更新发放记录
        let mut active: IssueActive = existing.clone().into();
        active.status = Set(IssueStatus::Lost.as_str().to_string());
        active.compensation_amount = Set(Some(compensation_amount));
        if let Some(r) = remark.clone() {
            active.remark = Set(Some(r));
        }
        active.actual_return_date = Set(Some(now.date_naive()));
        active.updated_at = Set(now);
        let updated = active.update(&txn).await?;

        // 2. 更新色卡：status = 'lost' + issued_quantity -= issue_qty（V15 P0-F10 库存联动）
        let card = ColorCardEntity::find_by_id(existing.color_card_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(IssueError::ColorCardNotFound)?;
        let new_issued = (card.issued_quantity - existing.issue_qty).max(0);
        let mut card_active: color_card::ActiveModel = card.into();
        card_active.status = Set(IssueStatus::Lost.as_str().to_string());
        card_active.issued_quantity = Set(new_issued);
        card_active.updated_at = Set(now);
        card_active.update(&txn).await?;

        txn.commit().await?;
        Ok(updated)
    }

    /// 标记损坏（V15 P0-F10：issued_quantity -= issue_qty，损坏色卡不可再用从已发放中扣除）
    pub async fn mark_damaged(
        &self,
        record_id: i64,
        compensation_amount: Option<Decimal>,
        remark: Option<String>,
    ) -> Result<color_card_issue::Model, IssueError> {
        if let Some(amt) = compensation_amount {
            if amt < Decimal::ZERO {
                return Err(IssueError::Validation("赔付金额不能为负".to_string()));
            }
        }

        let txn = self.db.begin().await?;

        let existing = IssueEntity::find_by_id(record_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(IssueError::RecordNotFound)?;
        let current = IssueStatus::from_str(&existing.status)
            .map_err(|_| IssueError::InvalidState(format!("未知状态: {}", existing.status)))?;
        if current.is_terminal() {
            return Err(IssueError::InvalidState(format!(
                "当前状态 {} 不允许标记损坏",
                existing.status
            )));
        }

        let now = Utc::now();
        let mut active: IssueActive = existing.clone().into();
        active.status = Set(IssueStatus::Damaged.as_str().to_string());
        active.compensation_amount = Set(compensation_amount);
        if let Some(r) = remark {
            active.remark = Set(Some(r));
        }
        active.actual_return_date = Set(Some(now.date_naive()));
        active.updated_at = Set(now);
        let result = active.update(&txn).await?;

        // V15 P0-F10：损坏色卡从已发放中扣除（不可再用）
        let card = ColorCardEntity::find_by_id(existing.color_card_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(IssueError::ColorCardNotFound)?;
        let new_issued = (card.issued_quantity - existing.issue_qty).max(0);
        let mut card_active: color_card::ActiveModel = card.into();
        card_active.issued_quantity = Set(new_issued);
        card_active.updated_at = Set(now);
        card_active.update(&txn).await?;

        txn.commit().await?;
        Ok(result)
    }

    /// 取消发放记录（仅 Issued 状态可取消，终态不可变更；V15 P0-F10：issued_quantity -= issue_qty 库存恢复）
    pub async fn cancel_issue(
        &self,
        record_id: i64,
        remark: Option<String>,
    ) -> Result<color_card_issue::Model, IssueError> {
        let txn = self.db.begin().await?;

        let existing = IssueEntity::find_by_id(record_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(IssueError::RecordNotFound)?;

        let current = IssueStatus::from_str(&existing.status)
            .map_err(|_| IssueError::InvalidState(format!("未知状态: {}", existing.status)))?;
        if current.is_terminal() {
            return Err(IssueError::InvalidState(format!(
                "当前状态 {} 不允许取消",
                existing.status
            )));
        }

        let now = Utc::now();
        let mut active: IssueActive = existing.clone().into();
        active.status = Set(IssueStatus::Cancelled.as_str().to_string());
        if let Some(r) = remark {
            active.remark = Set(Some(r));
        }
        active.updated_at = Set(now);
        let result = active.update(&txn).await?;

        // V15 P0-F10：恢复色卡 issued_quantity（ -= issue_qty，等同从未发放）
        let card = ColorCardEntity::find_by_id(existing.color_card_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(IssueError::ColorCardNotFound)?;
        let new_issued = (card.issued_quantity - existing.issue_qty).max(0);
        let mut card_active: color_card::ActiveModel = card.into();
        card_active.issued_quantity = Set(new_issued);
        card_active.updated_at = Set(now);
        card_active.update(&txn).await?;

        txn.commit().await?;
        Ok(result)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, record_id: i64) -> Result<color_card_issue::Model, IssueError> {
        IssueEntity::find_by_id(record_id)
            .one(&*self.db)
            .await?
            .ok_or(IssueError::RecordNotFound)
    }

    /// 列表查询（分页 + 多条件）
    pub async fn list_records(
        &self,
        query: ListIssuesQuery,
    ) -> Result<(Vec<color_card_issue::Model>, u64), IssueError> {
        let find = IssueEntity::find().filter(color_card_issue::Column::IsDeleted.eq(false));

        let mut cond = Condition::all();

        if let Some(card_id) = query.color_card_id {
            cond = cond.add(color_card_issue::Column::ColorCardId.eq(card_id));
        }
        if let Some(cust_id) = query.customer_id {
            cond = cond.add(color_card_issue::Column::CustomerId.eq(cust_id));
        }
        if let Some(status) = query.status {
            cond = cond.add(color_card_issue::Column::Status.eq(status));
        }
        if let Some(from) = query.from_date {
            cond = cond.add(color_card_issue::Column::IssuedAt.gte(from));
        }
        if let Some(to) = query.to_date {
            cond = cond.add(color_card_issue::Column::IssuedAt.lte(to));
        }
        // V15 P1 10.3-1：按销售订单 ID 过滤
        if let Some(sales_order_id) = query.sales_order_id {
            cond = cond.add(color_card_issue::Column::SalesOrderId.eq(sales_order_id));
        }

        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        let paginator = find
            .filter(cond)
            .order_by_desc(color_card_issue::Column::IssuedAt)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator
            .fetch_page(page.clamp(1, 1000).saturating_sub(1))
            .await?;
        Ok((items, total))
    }

    /// V15 P1 缺陷 10.3-1：按销售订单 ID 查询色卡发放记录（订单驱动场景，未软删除记录按发放时间倒序）
    pub async fn list_by_sales_order(
        &self,
        sales_order_id: i64,
    ) -> Result<Vec<color_card_issue::Model>, IssueError> {
        let items = IssueEntity::find()
            .filter(color_card_issue::Column::IsDeleted.eq(false))
            .filter(color_card_issue::Column::SalesOrderId.eq(sales_order_id))
            .order_by_desc(color_card_issue::Column::IssuedAt)
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// V15 P1 缺陷 10.2-4：查询客户专属色卡库（status='issued'，含色卡+发放记录信息）
    // 业务场景：销售避免重复发放/客户服务跟进/复购按缸号匹配历史色卡
    pub async fn list_customer_color_cards(
        &self,
        customer_id: i64,
    ) -> Result<Vec<CustomerColorCardView>, IssueError> {
        // 校验客户存在（fail-fast，避免无效查询）
        let _customer = CustomerEntity::find_by_id(customer_id as i32)
            .one(&*self.db)
            .await?
            .ok_or(IssueError::CustomerNotFound)?;

        // 查询客户当前持有的发放记录（status='issued'，未软删除）
        let issues = IssueEntity::find()
            .filter(color_card_issue::Column::CustomerId.eq(customer_id))
            .filter(color_card_issue::Column::Status.eq(IssueStatus::Issued.as_str()))
            .filter(color_card_issue::Column::IsDeleted.eq(false))
            .order_by_desc(color_card_issue::Column::IssuedAt)
            .all(&*self.db)
            .await?;

        if issues.is_empty() {
            return Ok(Vec::new());
        }

        // 批量查询关联的色卡信息（避免 N+1 查询）
        let card_ids: Vec<i64> = issues.iter().map(|i| i.color_card_id).collect();
        let cards = ColorCardEntity::find()
            .filter(color_card::Column::Id.is_in(card_ids))
            .all(&*self.db)
            .await?;
        let card_map: std::collections::HashMap<i64, color_card::Model> =
            cards.into_iter().map(|c| (c.id, c)).collect();

        // 组装视图
        let views = issues
            .into_iter()
            .filter_map(|issue| {
                let card = card_map.get(&issue.color_card_id)?;
                Some(CustomerColorCardView {
                    issue_id: issue.id,
                    color_card_id: issue.color_card_id,
                    color_card_code: card.card_no.clone(),
                    color_card_name: card.card_name.clone(),
                    customer_id: issue.customer_id,
                    issue_qty: issue.issue_qty,
                    issued_at: issue.issued_at,
                    expected_return_date: issue.expected_return_date,
                    dye_lot_no: issue.dye_lot_no.clone(),
                    status: issue.status,
                })
            })
            .collect();
        Ok(views)
    }

    /// V15 P1 缺陷 10.3-2：复购同缸号查询（按 last_issued_at 倒序返回同缸号色卡+库存可用量）
    // 业务场景：复购同缸号颜色一致；库存不足时提示重新排产
    pub async fn query_reorder_dye_lot(
        &self,
        customer_id: i64,
    ) -> Result<Vec<ReorderDyeLotView>, IssueError> {
        // 校验客户存在
        let _customer = CustomerEntity::find_by_id(customer_id as i32)
            .one(&*self.db)
            .await?
            .ok_or(IssueError::CustomerNotFound)?;

        // 简化实现：查询客户历史 dye_lot_no，再聚合色卡库存
        // 注：为避免 SeaORM 复杂查询，先查所有带 dye_lot_no 的发放记录，再在内存中按 (color_card_id, dye_lot_no) 去重
        let issues = IssueEntity::find()
            .filter(color_card_issue::Column::CustomerId.eq(customer_id))
            .filter(color_card_issue::Column::IsDeleted.eq(false))
            .filter(color_card_issue::Column::DyeLotNo.is_not_null())
            .order_by_desc(color_card_issue::Column::IssuedAt)
            .all(&*self.db)
            .await?;

        if issues.is_empty() {
            return Ok(Vec::new());
        }

        // 批量查询色卡库存
        let card_ids: Vec<i64> = issues.iter().map(|i| i.color_card_id).collect();
        let cards = ColorCardEntity::find()
            .filter(color_card::Column::Id.is_in(card_ids))
            .all(&*self.db)
            .await?;
        let card_map: std::collections::HashMap<i64, color_card::Model> =
            cards.into_iter().map(|c| (c.id, c)).collect();

        // 按 (color_card_id, dye_lot_no) 去重，保留最近一次发放记录
        let mut seen: std::collections::HashSet<(i64, String)> = std::collections::HashSet::new();
        let mut views: Vec<ReorderDyeLotView> = Vec::new();
        for issue in issues {
            let dye_lot = match &issue.dye_lot_no {
                Some(d) if !d.is_empty() => d.clone(),
                _ => continue,
            };
            let key = (issue.color_card_id, dye_lot.clone());
            if !seen.insert(key) {
                continue; // 已存在，跳过（保留首次=最近一次）
            }
            let card = match card_map.get(&issue.color_card_id) {
                Some(c) => c,
                None => continue,
            };
            let available = card.stock_quantity - card.issued_quantity;
            views.push(ReorderDyeLotView {
                color_card_id: card.id,
                color_card_code: card.card_no.clone(),
                color_card_name: card.card_name.clone(),
                dye_lot_no: dye_lot,
                last_issued_at: issue.issued_at,
                stock_quantity: card.stock_quantity,
                issued_quantity: card.issued_quantity,
                available_quantity: available,
            });
        }
        Ok(views)
    }

    /// V15 P1 缺陷 10.4-3：记录色卡发放审计日志（5类操作：发放/归还/遗失/损坏/取消，best-effort 不阻塞业务）
    // 参数：audit_service 来自 AppState；operation=issue/return/lost/damaged/cancel；
    // operator_id/operator_name=操作人；record=被操作记录；before=变更前快照（None 表示新建）
    pub fn record_issue_audit(
        &self,
        audit_service: Arc<AuditLogService>,
        operation: &str,
        operator_id: i32,
        operator_name: &str,
        record: &color_card_issue::Model,
        before: Option<serde_json::Value>,
    ) {
        let (op_type, severity, desc) = match operation {
            "issue" => (
                OperationType::Create,
                Severity::Info,
                format!(
                    "用户 {} 发放色卡（issue_id={}，色卡ID={}，客户ID={}，数量={}）",
                    operator_name,
                    record.id,
                    record.color_card_id,
                    record.customer_id,
                    record.issue_qty
                ),
            ),
            "return" => (
                OperationType::Update,
                Severity::Info,
                format!(
                    "用户 {} 归还色卡（issue_id={}，色卡ID={}）",
                    operator_name, record.id, record.color_card_id
                ),
            ),
            "lost" => (
                OperationType::Update,
                Severity::Warn,
                format!(
                    "用户 {} 登记色卡遗失（issue_id={}，色卡ID={}，赔付={:?}）",
                    operator_name, record.id, record.color_card_id, record.compensation_amount
                ),
            ),
            "damaged" => (
                OperationType::Update,
                Severity::Warn,
                format!(
                    "用户 {} 标记色卡损坏（issue_id={}，色卡ID={}）",
                    operator_name, record.id, record.color_card_id
                ),
            ),
            "cancel" => (
                OperationType::Delete,
                Severity::Warn,
                format!(
                    "用户 {} 取消色卡发放（issue_id={}，色卡ID={}）",
                    operator_name, record.id, record.color_card_id
                ),
            ),
            _ => return, // 未知操作不记录
        };

        let after_snapshot = serde_json::json!({
            "issue_id": record.id,
            "color_card_id": record.color_card_id,
            "customer_id": record.customer_id,
            "issue_qty": record.issue_qty,
            "status": record.status,
            "dye_lot_no": record.dye_lot_no,
            "sales_order_id": record.sales_order_id,
        });

        let event = AuditEvent {
            user_id: Some(operator_id),
            username: Some(operator_name.to_string()),
            operation_type: op_type,
            severity,
            resource_type: Some("color_card_issue".to_string()),
            resource_id: Some(record.id.to_string()),
            resource_name: Some(format!("色卡发放记录#{}", record.id)),
            description: Some(desc),
            request_method: None,
            request_path: None,
            before_snapshot: before,
            after_snapshot: Some(after_snapshot),
        };
        // 异步落库，best-effort 不阻塞业务
        audit_service.record_async(event, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn 测试_issue_status_as_str_全部状态映射() {
        assert_eq!(IssueStatus::Issued.as_str(), "issued");
        assert_eq!(IssueStatus::Returned.as_str(), "returned");
        assert_eq!(IssueStatus::Lost.as_str(), "lost");
        assert_eq!(IssueStatus::Damaged.as_str(), "damaged");
        assert_eq!(IssueStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn 测试_issue_status_终态判定_终态返回true() {
        assert!(IssueStatus::Returned.is_terminal());
        assert!(IssueStatus::Lost.is_terminal());
        assert!(IssueStatus::Damaged.is_terminal());
        assert!(IssueStatus::Cancelled.is_terminal());
    }

    #[test]
    fn 测试_issue_status_终态判定_非终态返回false() {
        assert!(!IssueStatus::Issued.is_terminal());
    }

    #[test]
    fn 测试_issue_status_from_str_合法字符串解析成功() {
        assert_eq!(
            IssueStatus::from_str("issued").unwrap(),
            IssueStatus::Issued
        );
        assert_eq!(
            IssueStatus::from_str("returned").unwrap(),
            IssueStatus::Returned
        );
        assert_eq!(IssueStatus::from_str("lost").unwrap(), IssueStatus::Lost);
        assert_eq!(
            IssueStatus::from_str("damaged").unwrap(),
            IssueStatus::Damaged
        );
        assert_eq!(
            IssueStatus::from_str("cancelled").unwrap(),
            IssueStatus::Cancelled
        );
    }

    #[test]
    fn 测试_issue_status_from_str_非法字符串解析失败() {
        assert!(IssueStatus::from_str("unknown").is_err());
        assert!(IssueStatus::from_str("").is_err());
        assert!(IssueStatus::from_str("ISSUED").is_err());
        assert!(IssueStatus::from_str(" issued ").is_err());
    }

    #[test]
    fn 测试_issue_status_序列化反序列化往返一致() {
        let all_statuses = [
            IssueStatus::Issued,
            IssueStatus::Returned,
            IssueStatus::Lost,
            IssueStatus::Damaged,
            IssueStatus::Cancelled,
        ];
        for status in all_statuses {
            let serialized = status.as_str();
            let parsed = IssueStatus::from_str(serialized).unwrap();
            assert_eq!(status, parsed, "状态 {:?} 序列化反序列化往返不一致", status);
        }
    }

    #[test]
    fn 测试_issue_status_状态机完整性_终态数量正确() {
        let all_statuses = [
            IssueStatus::Issued,
            IssueStatus::Returned,
            IssueStatus::Lost,
            IssueStatus::Damaged,
            IssueStatus::Cancelled,
        ];
        let terminal_count = all_statuses.iter().filter(|s| s.is_terminal()).count();
        assert_eq!(
            terminal_count, 4,
            "应有 4 个终态（returned/lost/damaged/cancelled）"
        );
        let non_terminal_count = all_statuses.iter().filter(|s| !s.is_terminal()).count();
        assert_eq!(non_terminal_count, 1, "应有 1 个非终态（issued）");
    }
}
