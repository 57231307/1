//! 面料多色号定价扩展 - 价格历史 Service
//!
//! 价格历史记录与查询
//! 创建时间: 2026-06-18

use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};
use std::sync::Arc;
use thiserror::Error;

use crate::models::color_price_history;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

/// 业务错误
#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
    /// 批次 263：接入 paginate_with_total（返回 AppError）所需的错误转换
    #[error("应用错误: {0}")]
    App(#[from] AppError),
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
    ///
    /// 批次 263 修复：接入 paginate_with_total 工具函数，消除手写 num_items + fetch_page 重复。
    /// paginate_with_total 内部已做 page.saturating_sub(1) 偏移，调用方不可再减 1。
    /// 补 page.clamp(1, 1000) + page_size.clamp(1, 100) 防 DoS（原实现无任何 clamp 保护）。
    pub async fn list_by_price(
        &self,
        price_id: i64,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<color_price_history::Model>, u64), HistoryError> {
        let paginator = color_price_history::Entity::find()
            .filter(color_price_history::Column::ProductColorPriceId.eq(price_id))
            .order_by_desc(color_price_history::Column::OperatedAt)
            .paginate(&*self.db, page_size.clamp(1, 100));

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
        Ok((items, total))
    }
}
