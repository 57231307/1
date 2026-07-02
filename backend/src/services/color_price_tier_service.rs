//! 面料多色号定价扩展 - 阶梯定价 Service
//!
//! 阶梯价 CRUD + 按数量查询
//! 创建时间: 2026-06-18

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use std::sync::Arc;
use thiserror::Error;

use crate::models::color_price_tier::{self, ActiveModel as TierActive, Entity as TierEntity};
use crate::models::color_price_tier_dto::CreatePriceTierDto;

#[derive(Debug, Error)]
pub enum TierError {
    #[error("阶梯价不存在")]
    NotFound,
    #[error("色号价格不存在")]
    PriceNotFound,
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
}

pub struct ColorPriceTierService {
    db: Arc<DatabaseConnection>,
}

impl ColorPriceTierService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &crate::utils::app_state::AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 按 price_id 列出所有阶梯
    pub async fn list_by_price(
        &self,
        price_id: i64,
    ) -> Result<Vec<color_price_tier::Model>, TierError> {
        let items = TierEntity::find()
            .filter(color_price_tier::Column::ProductColorPriceId.eq(price_id))
            .order_by_asc(color_price_tier::Column::Sequence)
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 创建阶梯价
    pub async fn create(
        &self,
        dto: CreatePriceTierDto,
    ) -> Result<color_price_tier::Model, TierError> {
        if let Some(max) = dto.max_quantity {
            if dto.min_quantity >= max {
                return Err(TierError::Validation(
                    "min_quantity 必须小于 max_quantity".to_string(),
                ));
            }
        }

        let now = Utc::now();
        let active = TierActive {
            id: Default::default(),
            product_color_price_id: Set(dto.product_color_price_id),
            min_quantity: Set(dto.min_quantity),
            max_quantity: Set(dto.max_quantity),
            tier_price: Set(dto.tier_price),
            customer_level: Set(dto.customer_level),
            sequence: Set(dto.sequence.unwrap_or(0)),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let result = active.insert(&*self.db).await?;
        Ok(result)
    }

    /// 删除阶梯价
    pub async fn delete(&self, id: i64) -> Result<(), TierError> {
        // P0 8-3 修复：delete 操作补审计日志（i64 主键变体）
        crate::services::audit_log_service::AuditLogService::delete_with_audit_i64::<
            TierEntity,
            _,
        >(&*self.db, "color_price_tier", id, Some(0))
        .await
        .map_err(|e| match e {
            crate::utils::error::AppError::NotFound(_) => TierError::NotFound,
            other => TierError::Validation(other.to_string()),
        })
    }
}
