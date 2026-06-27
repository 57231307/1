//! 色卡借出管理服务
//!
//! 提供借出 / 归还 / 遗失 / 损坏 / 历史查询业务
//! 状态机：borrowed → returned / lost / damaged（终态不可再转换）
//! 创建时间: 2026-06-17

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
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
}

#[allow(dead_code)] // TODO(tech-debt): 色卡借还路由接入后移除
impl BorrowStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Borrowed => "borrowed",
            Self::Returned => "returned",
            Self::Lost => "lost",
            Self::Damaged => "damaged",
        }
    }

    /// 从字符串解析借出状态（保持 Option 返回值以兼容现有调用方及测试）
    #[allow(clippy::should_implement_trait)] // TODO(tech-debt): 业务稳定后迁移到 std::str::FromStr
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "borrowed" => Some(Self::Borrowed),
            "returned" => Some(Self::Returned),
            "lost" => Some(Self::Lost),
            "damaged" => Some(Self::Damaged),
            _ => None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Returned | Self::Lost | Self::Damaged)
    }
}

/// 借出管理服务
pub struct ColorCardBorrowService {
    db: Arc<DatabaseConnection>,
}

#[allow(dead_code)] // TODO(tech-debt): 色卡借还路由接入后移除
impl ColorCardBorrowService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db: db.clone(),
        }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 创建借出记录
    #[allow(clippy::too_many_arguments)]
    pub async fn borrow(
        &self,
        color_card_id: i64,
        customer_id: i64,
        borrowed_by: i64,
        expected_return_at: Option<chrono::DateTime<Utc>>,
        purpose: Option<String>,
        notes: Option<String>,
        tenant_id: i64,
    ) -> Result<color_card_borrow_record::Model, BorrowError> {
        // 验证色卡存在
        let _card = ColorCardEntity::find_by_id(color_card_id)
            .filter(color_card::Column::TenantId.eq(tenant_id))
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
            status: Set("borrowed".to_string()),
            purpose: Set(purpose),
            notes: Set(notes),
            compensation_amount: Set(None),
            tenant_id: Set(tenant_id),
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
        tenant_id: i64,
    ) -> Result<color_card_borrow_record::Model, BorrowError> {
        let existing = self.get_by_id(record_id, tenant_id).await?;

        // 状态机校验
        let current = BorrowStatus::from_str(&existing.status)
            .ok_or_else(|| BorrowError::InvalidState(format!("未知状态: {}", existing.status)))?;
        if current != BorrowStatus::Borrowed {
            return Err(BorrowError::InvalidState(format!(
                "当前状态 {} 不允许归还操作",
                existing.status
            )));
        }

        let mut active: BorrowActive = existing.into();
        active.status = Set("returned".to_string());
        active.actual_return_at = Set(Some(actual_return_at.unwrap_or_else(Utc::now)));
        if let Some(n) = notes {
            active.notes = Set(Some(n));
        }
        active.updated_at = Set(Utc::now());

        let result = active.update(&*self.db).await?;
        Ok(result)
    }

    /// 登记遗失（含赔付金额）
    pub async fn mark_lost(
        &self,
        record_id: i64,
        compensation_amount: rust_decimal::Decimal,
        notes: Option<String>,
        tenant_id: i64,
    ) -> Result<color_card_borrow_record::Model, BorrowError> {
        if compensation_amount <= rust_decimal::Decimal::ZERO {
            return Err(BorrowError::Validation("赔付金额必须 > 0".to_string()));
        }

        let existing = self.get_by_id(record_id, tenant_id).await?;
        let current = BorrowStatus::from_str(&existing.status)
            .ok_or_else(|| BorrowError::InvalidState(format!("未知状态: {}", existing.status)))?;
        if current != BorrowStatus::Borrowed {
            return Err(BorrowError::InvalidState(format!(
                "当前状态 {} 不允许登记遗失",
                existing.status
            )));
        }

        let txn = self.db.begin().await?;

        // 1. 更新借出记录
        let mut active: BorrowActive = existing.clone().into();
        active.status = Set("lost".to_string());
        active.compensation_amount = Set(Some(compensation_amount));
        if let Some(n) = notes.clone() {
            active.notes = Set(Some(n));
        }
        active.actual_return_at = Set(Some(Utc::now()));
        active.updated_at = Set(Utc::now());
        let updated = active.update(&txn).await?;

        // 2. 更新色卡状态为 lost
        let card = ColorCardEntity::find_by_id(existing.color_card_id)
            .filter(color_card::Column::TenantId.eq(tenant_id))
            .one(&txn)
            .await?
            .ok_or(BorrowError::ColorCardNotFound)?;
        let mut card_active: color_card::ActiveModel = card.into();
        card_active.status = Set("lost".to_string());
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
        tenant_id: i64,
    ) -> Result<color_card_borrow_record::Model, BorrowError> {
        if let Some(amt) = compensation_amount {
            if amt < rust_decimal::Decimal::ZERO {
                return Err(BorrowError::Validation("赔付金额不能为负".to_string()));
            }
        }

        let existing = self.get_by_id(record_id, tenant_id).await?;
        let current = BorrowStatus::from_str(&existing.status)
            .ok_or_else(|| BorrowError::InvalidState(format!("未知状态: {}", existing.status)))?;
        if current != BorrowStatus::Borrowed {
            return Err(BorrowError::InvalidState(format!(
                "当前状态 {} 不允许标记损坏",
                existing.status
            )));
        }

        let mut active: BorrowActive = existing.into();
        active.status = Set("damaged".to_string());
        active.compensation_amount = Set(compensation_amount);
        if let Some(n) = notes {
            active.notes = Set(Some(n));
        }
        active.actual_return_at = Set(Some(Utc::now()));
        active.updated_at = Set(Utc::now());

        let result = active.update(&*self.db).await?;
        Ok(result)
    }

    /// 按 ID 查询（多租户隔离）
    pub async fn get_by_id(
        &self,
        record_id: i64,
        tenant_id: i64,
    ) -> Result<color_card_borrow_record::Model, BorrowError> {
        BorrowEntity::find_by_id(record_id)
            .filter(color_card_borrow_record::Column::TenantId.eq(tenant_id))
            .one(&*self.db)
            .await?
            .ok_or(BorrowError::RecordNotFound)
    }

    /// 列表查询（分页 + 多条件 + 多租户隔离）
    #[allow(clippy::too_many_arguments)]
    pub async fn list_records(
        &self,
        tenant_id: i64,
        query: ListBorrowRecordsQuery,
    ) -> Result<(Vec<color_card_borrow_record::Model>, u64), BorrowError> {
        let find = BorrowEntity::find();

        // 强制多租户隔离
        let mut cond = Condition::all()
            .add(color_card_borrow_record::Column::TenantId.eq(tenant_id));

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
        let page_size = query.page_size.unwrap_or(20);

        let paginator = find
            .filter(cond)
            .order_by_desc(color_card_borrow_record::Column::BorrowedAt)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;
        Ok((items, total))
    }
}
