//! 面料多色号定价扩展 - 季节调价 Service
//!
//! 季节调价规则 + 自动应用
//! 创建时间: 2026-06-18

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use std::sync::Arc;
use thiserror::Error;

use crate::models::seasonal_price_rule::{self, ActiveModel as RuleActive, Entity as RuleEntity};
use crate::models::seasonal_price_rule_dto::{
    CreateSeasonalRuleDto, ListSeasonalRulesQuery, UpdateSeasonalRuleDto,
};
use crate::utils::error::AppError;

/// 业务错误
#[derive(Debug, Error)]
pub enum SeasonalError {
    #[error("规则不存在")]
    NotFound,
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
}

/// 季节调价服务
pub struct ColorPriceSeasonalService {
    db: Arc<DatabaseConnection>,
}

impl ColorPriceSeasonalService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &crate::utils::app_state::AppState) -> Self {
        // 批次 95 CI 修复：复用 new 构造函数，消除 new 的 dead_code 警告
        Self::new(state.db.clone())
    }

    /// 列表查询
    pub async fn list(
        &self,
        query: &ListSeasonalRulesQuery,
    ) -> Result<(Vec<seasonal_price_rule::Model>, u64), SeasonalError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        let mut q = RuleEntity::find();

        if let Some(s) = &query.season {
            q = q.filter(seasonal_price_rule::Column::Season.eq(s.clone()));
        }
        if let Some(active) = query.is_active {
            q = q.filter(seasonal_price_rule::Column::IsActive.eq(active));
        }
        if let Some(cat) = query.product_category_id {
            q = q.filter(seasonal_price_rule::Column::ProductCategoryId.eq(cat));
        }

        let paginator = q
            .order_by_desc(seasonal_price_rule::Column::CreatedAt)
            .paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;
        Ok((items, total))
    }

    /// 按 ID 查询
    pub async fn get_by_id(
        &self,
        id: i64,
    ) -> Result<seasonal_price_rule::Model, SeasonalError> {
        RuleEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or(SeasonalError::NotFound)
    }

    /// 创建规则
    pub async fn create(
        &self,
        dto: CreateSeasonalRuleDto,
    ) -> Result<seasonal_price_rule::Model, SeasonalError> {
        Self::validate_season(&dto.season)?;
        Self::validate_adjustment_type(&dto.adjustment_type)?;
        Self::validate_date_range(dto.valid_from, dto.valid_until)?;

        let now = Utc::now();
        let active = RuleActive {
            id: Default::default(),
            rule_name: Set(dto.rule_name),
            season: Set(dto.season),
            product_category_id: Set(dto.product_category_id),
            adjustment_type: Set(dto.adjustment_type),
            adjustment_value: Set(dto.adjustment_value),
            valid_from: Set(dto.valid_from),
            valid_until: Set(dto.valid_until),
            is_active: Set(true),
            description: Set(dto.description),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let result = active.insert(&*self.db).await?;
        Ok(result)
    }

    /// 更新规则
    pub async fn update(
        &self,
        id: i64,
        dto: UpdateSeasonalRuleDto,
    ) -> Result<seasonal_price_rule::Model, SeasonalError> {
        let existing = self.get_by_id(id).await?;

        if let Some(s) = &dto.season {
            Self::validate_season(s)?;
        }
        if let Some(t) = &dto.adjustment_type {
            Self::validate_adjustment_type(t)?;
        }
        if dto.valid_from.is_some() || dto.valid_until.is_some() {
            let from = dto.valid_from.unwrap_or(existing.valid_from);
            let until = dto.valid_until.or(existing.valid_until);
            Self::validate_date_range(from, until)?;
        }

        let mut active: RuleActive = existing.into();
        if let Some(v) = dto.rule_name {
            active.rule_name = Set(v);
        }
        if let Some(v) = dto.season {
            active.season = Set(v);
        }
        if let Some(v) = dto.product_category_id {
            active.product_category_id = Set(Some(v));
        }
        if let Some(v) = dto.adjustment_type {
            active.adjustment_type = Set(v);
        }
        if let Some(v) = dto.adjustment_value {
            active.adjustment_value = Set(v);
        }
        if let Some(v) = dto.valid_from {
            active.valid_from = Set(v);
        }
        if let Some(v) = dto.valid_until {
            active.valid_until = Set(Some(v));
        }
        if let Some(v) = dto.is_active {
            active.is_active = Set(v);
        }
        if let Some(v) = dto.description {
            active.description = Set(Some(v));
        }
        active.updated_at = Set(Utc::now());

        let result = active.update(&*self.db).await?;
        Ok(result)
    }

    /// 删除规则
    // 批次 94 P2-5 修复：补 user_id 参数 + 使用 delete_with_audit_i64 记录审计日志
    // 原实现仅软删除（is_active=false）无审计日志，改为物理删除 + 审计日志，满足合规追溯要求
    pub async fn delete(&self, id: i64, user_id: i32) -> Result<(), AppError> {
        crate::services::audit_log_service::AuditLogService::delete_with_audit_i64::<RuleEntity, _>(
            &*self.db,
            "color_price_seasonal",
            id,
            Some(user_id),
        )
        .await
    }

    // ----------------------------------------------------------------------
    // 校验
    // ----------------------------------------------------------------------

    fn validate_season(s: &str) -> Result<(), SeasonalError> {
        match s {
            "SS" | "AW" | "HOLIDAY" => Ok(()),
            _ => Err(SeasonalError::Validation(format!(
                "无效的季节: {}（允许: SS / AW / HOLIDAY）",
                s
            ))),
        }
    }

    fn validate_adjustment_type(t: &str) -> Result<(), SeasonalError> {
        match t {
            "percentage" | "fixed" => Ok(()),
            _ => Err(SeasonalError::Validation(format!(
                "无效的调整方式: {}（允许: percentage / fixed）",
                t
            ))),
        }
    }

    fn validate_date_range(
        from: chrono::NaiveDate,
        until: Option<chrono::NaiveDate>,
    ) -> Result<(), SeasonalError> {
        if let Some(u) = until {
            if from > u {
                return Err(SeasonalError::Validation(
                    "valid_from 必须早于 valid_until".to_string(),
                ));
            }
        }
        Ok(())
    }
}
