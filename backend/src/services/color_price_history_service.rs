//! 面料多色号定价扩展 - 价格历史 Service
//!
//! 价格历史记录与查询
//! 创建时间: 2026-06-18

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use std::sync::Arc;
use thiserror::Error;

use crate::models::color_price_history::{self, ActiveModel as HistoryActive};

/// 业务错误
#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
}

/// 价格历史服务
pub struct ColorPriceHistoryService {
    db: Arc<DatabaseConnection>,
}

impl ColorPriceHistoryService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &crate::utils::app_state::AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 按 price_id 查询历史
    pub async fn list_by_price(
        &self,
        price_id: i64,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<color_price_history::Model>, u64), HistoryError> {
        let paginator = color_price_history::Entity::find()
            .filter(color_price_history::Column::ProductColorPriceId.eq(price_id))
            .order_by_desc(color_price_history::Column::OperatedAt)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;
        Ok((items, total))
    }

    /// 记录价格变更（供其他 service 调用）
    pub async fn record_change(
        &self,
        price_id: i64,
        old_price: Decimal,
        new_price: Decimal,
        currency: String,
        change_type: String,
        change_reason: Option<String>,
        change_percent: Option<Decimal>,
        quantity: Option<Decimal>,
        operated_by: i64,
    ) -> Result<color_price_history::Model, HistoryError> {
        let history = HistoryActive {
            id: Default::default(),
            product_color_price_id: Set(price_id),
            old_price: Set(old_price),
            new_price: Set(new_price),
            currency: Set(currency),
            change_type: Set(change_type),
            change_reason: Set(change_reason),
            change_percent: Set(change_percent),
            quantity: Set(quantity),
            operated_by: Set(operated_by),
            operated_at: Set(Utc::now()),
            approved_by: Set(None),
            approved_at: Set(None),
        };
        let result = history.insert(&*self.db).await?;
        Ok(result)
    }
}
