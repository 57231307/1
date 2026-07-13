//! 色卡借出管理服务
//!
//! 提供借出 / 归还 / 遗失 / 损坏 / 历史查询业务
//! 状态机：borrowed → returned / lost / damaged（终态不可再转换）
//! 创建时间: 2026-06-17

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;

use crate::models::color_card::{self, Entity as ColorCardEntity};
use crate::models::color_card_borrow_dto::ListBorrowRecordsQuery;
use crate::models::color_card_borrow_record::{
    self, ActiveModel as BorrowActive, Entity as BorrowEntity,
};
use crate::utils::app_state::AppState;

/// 业务错误
#[derive(Debug, Error)]
pub enum BorrowError {
    #[error("色卡不存在")]
    ColorCardNotFound,
    #[error("借出记录不存在")]
    RecordNotFound,
    #[error("色卡当前状态不允许此操作")]
    InvalidState(String),
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
}

/// 借出记录状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorrowStatus {
    Borrowed,
    Returned,
    Lost,
    Damaged,
    /// 已取消（L-22 修复，批次 368 v13 复审）：借出记录主动取消，不再参与归还/遗失/损坏流程
    Cancelled,
}

// v11 批次 147 P2-B：移除失效的 dead_code 标注
// - from_str 在 service 内部 line 160/203/260 调用
// - is_terminal / as_str 在业务中真实接入（v11 P1-5 真实实现）
// 批次 344 v11 复审 P1 修复：from_str 方法迁移到 std::str::FromStr trait，
// 消除 clippy::should_implement_trait 警告
impl BorrowStatus {
    /// 序列化为字符串（持久化到数据库的稳定字符串）
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Borrowed => "borrowed",
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

/// BorrowStatus 解析错误
#[derive(Debug, Clone)]
pub struct BorrowStatusParseError(pub String);

impl std::fmt::Display for BorrowStatusParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BorrowStatus 解析失败: {}", self.0)
    }
}

impl std::error::Error for BorrowStatusParseError {}

/// 批次 344 v11 复审 P1 修复：实现 std::str::FromStr trait 替代 from_str 方法
impl FromStr for BorrowStatus {
    type Err = BorrowStatusParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "borrowed" => Ok(Self::Borrowed),
            "returned" => Ok(Self::Returned),
            "lost" => Ok(Self::Lost),
            "damaged" => Ok(Self::Damaged),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(BorrowStatusParseError(s.to_string())),
        }
    }
}

/// 借出管理服务
pub struct ColorCardBorrowService {
    db: Arc<DatabaseConnection>,
}

// v11 批次 147 P2-B：移除失效的 dead_code 标注（被 handlers/color_card/borrow.rs 真实调用）
impl ColorCardBorrowService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self::new(state.db.clone())
    }

    /// 创建借出记录
    pub async fn borrow(
        &self,
        color_card_id: i64,
        customer_id: i64,
        borrowed_by: i64,
        expected_return_at: Option<chrono::DateTime<Utc>>,
        purpose: Option<String>,
        notes: Option<String>,
    ) -> Result<color_card_borrow_record::Model, BorrowError> {
        // 验证色卡存在
        let _card = ColorCardEntity::find_by_id(color_card_id)
            .one(&*self.db)
            .await?
            .ok_or(BorrowError::ColorCardNotFound)?;

        // 业务校验：预计归还时间不能超过借出时间 + 30 天
        if let Some(expected) = expected_return_at {
            let now = Utc::now();
            let max_expected = now + chrono::Duration::days(30);
            if expected > max_expected {
                return Err(BorrowError::Validation(
                    "预计归还时间不能超过借出时间 + 30 天".to_string(),
                ));
            }
        }

        let now = Utc::now();
        let active = BorrowActive {
            id: Default::default(),
            color_card_id: Set(color_card_id),
            customer_id: Set(customer_id),
            borrowed_by: Set(borrowed_by),
            borrowed_at: Set(now),
            expected_return_at: Set(expected_return_at),
            actual_return_at: Set(None),
            status: Set(BorrowStatus::Borrowed.as_str().to_string()),
            purpose: Set(purpose),
            notes: Set(notes),
            compensation_amount: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let result = active.insert(&*self.db).await?;
        Ok(result)
    }

    /// 归还色卡
    pub async fn return_card(
        &self,
        record_id: i64,
        actual_return_at: Option<chrono::DateTime<Utc>>,
        notes: Option<String>,
    ) -> Result<color_card_borrow_record::Model, BorrowError> {
        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 开启事务
        let txn = self.db.begin().await?;

        // 检查借出记录是否存在（行锁，串行化并发状态变更）
        let existing = BorrowEntity::find_by_id(record_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(BorrowError::RecordNotFound)?;

        // 状态机校验
        let current = BorrowStatus::from_str(&existing.status)
            .map_err(|_| BorrowError::InvalidState(format!("未知状态: {}", existing.status)))?;
        if current.is_terminal() {
            return Err(BorrowError::InvalidState(format!(
                "当前状态 {} 不允许归还操作",
                existing.status
            )));
        }

        let mut active: BorrowActive = existing.into();
        active.status = Set(BorrowStatus::Returned.as_str().to_string());
        active.actual_return_at = Set(Some(actual_return_at.unwrap_or_else(Utc::now)));
        if let Some(n) = notes {
            active.notes = Set(Some(n));
        }
        active.updated_at = Set(Utc::now());

        let result = active.update(&txn).await?;
        txn.commit().await?;
        Ok(result)
    }

    /// 登记遗失（含赔付金额）
    pub async fn mark_lost(
        &self,
        record_id: i64,
        compensation_amount: rust_decimal::Decimal,
        notes: Option<String>,
    ) -> Result<color_card_borrow_record::Model, BorrowError> {
        if compensation_amount <= rust_decimal::Decimal::ZERO {
            return Err(BorrowError::Validation("赔付金额必须 > 0".to_string()));
        }

        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 开启事务
        let txn = self.db.begin().await?;

        // 检查借出记录是否存在（行锁，串行化并发状态变更）
        let existing = BorrowEntity::find_by_id(record_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(BorrowError::RecordNotFound)?;
        let current = BorrowStatus::from_str(&existing.status)
            .map_err(|_| BorrowError::InvalidState(format!("未知状态: {}", existing.status)))?;
        if current.is_terminal() {
            return Err(BorrowError::InvalidState(format!(
                "当前状态 {} 不允许登记遗失",
                existing.status
            )));
        }

        // 1. 更新借出记录
        let mut active: BorrowActive = existing.clone().into();
        active.status = Set(BorrowStatus::Lost.as_str().to_string());
        active.compensation_amount = Set(Some(compensation_amount));
        if let Some(n) = notes.clone() {
            active.notes = Set(Some(n));
        }
        active.actual_return_at = Set(Some(Utc::now()));
        active.updated_at = Set(Utc::now());
        let updated = active.update(&txn).await?;

        // 2. 更新色卡状态为 lost
        let card = ColorCardEntity::find_by_id(existing.color_card_id)
            .one(&txn)
            .await?
            .ok_or(BorrowError::ColorCardNotFound)?;
        let mut card_active: color_card::ActiveModel = card.into();
        card_active.status = Set(BorrowStatus::Lost.as_str().to_string());
        card_active.updated_at = Set(Utc::now());
        card_active.update(&txn).await?;

        txn.commit().await?;
        Ok(updated)
    }

    /// 标记损坏
    pub async fn mark_damaged(
        &self,
        record_id: i64,
        compensation_amount: Option<rust_decimal::Decimal>,
        notes: Option<String>,
    ) -> Result<color_card_borrow_record::Model, BorrowError> {
        if let Some(amt) = compensation_amount {
            if amt < rust_decimal::Decimal::ZERO {
                return Err(BorrowError::Validation("赔付金额不能为负".to_string()));
            }
        }

        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 开启事务
        let txn = self.db.begin().await?;

        // 检查借出记录是否存在（行锁，串行化并发状态变更）
        let existing = BorrowEntity::find_by_id(record_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(BorrowError::RecordNotFound)?;
        let current = BorrowStatus::from_str(&existing.status)
            .map_err(|_| BorrowError::InvalidState(format!("未知状态: {}", existing.status)))?;
        if current.is_terminal() {
            return Err(BorrowError::InvalidState(format!(
                "当前状态 {} 不允许标记损坏",
                existing.status
            )));
        }

        let mut active: BorrowActive = existing.into();
        active.status = Set(BorrowStatus::Damaged.as_str().to_string());
        active.compensation_amount = Set(compensation_amount);
        if let Some(n) = notes {
            active.notes = Set(Some(n));
        }
        active.actual_return_at = Set(Some(Utc::now()));
        active.updated_at = Set(Utc::now());

        let result = active.update(&txn).await?;
        txn.commit().await?;
        Ok(result)
    }

    /// 取消借出记录（L-22 修复，批次 368 v13 复审）
    ///
    /// 仅允许 `Borrowed` 状态取消，取消后为终态不可再变更。
    /// 用于登记错误借出/客户撤回等场景，区别于 Returned（正常归还）。
    #[allow(dead_code)] // TODO(tech-debt): handler 接入后移除
    pub async fn cancel_borrow(
        &self,
        record_id: i64,
        notes: Option<String>,
    ) -> Result<color_card_borrow_record::Model, BorrowError> {
        let txn = self.db.begin().await?;

        let existing = BorrowEntity::find_by_id(record_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(BorrowError::RecordNotFound)?;

        let current = BorrowStatus::from_str(&existing.status)
            .map_err(|_| BorrowError::InvalidState(format!("未知状态: {}", existing.status)))?;
        if current.is_terminal() {
            return Err(BorrowError::InvalidState(format!(
                "当前状态 {} 不允许取消",
                existing.status
            )));
        }

        let mut active: BorrowActive = existing.into();
        active.status = Set(BorrowStatus::Cancelled.as_str().to_string());
        if let Some(n) = notes {
            active.notes = Set(Some(n));
        }
        active.updated_at = Set(Utc::now());

        let result = active.update(&txn).await?;
        txn.commit().await?;
        Ok(result)
    }

    /// 按 ID 查询
    pub async fn get_by_id(
        &self,
        record_id: i64,
    ) -> Result<color_card_borrow_record::Model, BorrowError> {
        BorrowEntity::find_by_id(record_id)
            .one(&*self.db)
            .await?
            .ok_or(BorrowError::RecordNotFound)
    }

    /// 列表查询（分页 + 多条件）
    pub async fn list_records(
        &self,
        query: ListBorrowRecordsQuery,
    ) -> Result<(Vec<color_card_borrow_record::Model>, u64), BorrowError> {
        let find = BorrowEntity::find();

        let mut cond = Condition::all();

        if let Some(card_id) = query.color_card_id {
            cond = cond.add(color_card_borrow_record::Column::ColorCardId.eq(card_id));
        }
        if let Some(cust_id) = query.customer_id {
            cond = cond.add(color_card_borrow_record::Column::CustomerId.eq(cust_id));
        }
        if let Some(status) = query.status {
            cond = cond.add(color_card_borrow_record::Column::Status.eq(status));
        }
        if let Some(from) = query.from_date {
            cond = cond.add(color_card_borrow_record::Column::BorrowedAt.gte(from));
        }
        if let Some(to) = query.to_date {
            cond = cond.add(color_card_borrow_record::Column::BorrowedAt.lte(to));
        }

        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100); // v10 P1-1 修复：page_size clamp(1,100) 防 DoS

        let paginator = find
            .filter(cond)
            .order_by_desc(color_card_borrow_record::Column::BorrowedAt)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
        let items = paginator.fetch_page(page.clamp(1, 1000).saturating_sub(1)).await?;
        Ok((items, total))
    }
}

// 死代码清理（2026-06-26）：_ensure_color_space_converter_used 为抑制未使用导入的 hack，
// 已删除多余的 use crate::utils::color_space_converter 语句，函数一并删除。
