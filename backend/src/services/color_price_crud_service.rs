//! 面料多色号定价扩展 - CRUD Service
//!
//! 提供色号价格基础 CRUD 业务：create / list / get_by_id / update / delete
//! 其他业务（批量调价 / 历史 / 季节）在各自 service 中实现
//! 创建时间: 2026-06-18

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use std::sync::Arc;
use thiserror::Error;

use crate::models::color_price_dto::{
    CreateColorPriceDto, ListColorPricesQuery, UpdateColorPriceDto,
};
// 批次 158 v11 真实接入：审批状态常量替代字符串字面量
use crate::models::status::approval;
use crate::models::product_color_price::{
    self, ActiveModel as ColorPriceActive, Entity as ColorPriceEntity,
};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

/// 业务错误
#[derive(Debug, Error)]
pub enum CrudError {
    #[error("色号价格不存在")]
    NotFound,
    #[error("当前状态不允许此操作")]
    InvalidState,
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
    /// 批次 263：接入 paginate_with_total（返回 AppError）所需的错误转换
    #[error("应用错误: {0}")]
    App(#[from] AppError),
}

/// 色号价格 CRUD 服务
pub struct ColorPriceCrudService {
    db: Arc<DatabaseConnection>,
}

impl ColorPriceCrudService {
    /// 从数据库连接构造
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 从 AppState 构造
    pub fn from_state(state: &crate::utils::app_state::AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 创建色号价格
    pub async fn create(
        &self,
        dto: CreateColorPriceDto,
        operated_by: i64,
    ) -> Result<product_color_price::Model, CrudError> {
        // 1. 业务校验
        Self::validate_currency(&dto.currency)?;
        Self::validate_season(dto.season.as_deref())?;
        Self::validate_customer_level(dto.customer_level.as_deref())?;
        Self::validate_quantity_range(dto.min_quantity, dto.max_quantity)?;
        Self::validate_date_range(dto.effective_from, dto.effective_to)?;

        // 2. 插入主表
        let now = Utc::now();
        let active = ColorPriceActive {
            id: Default::default(),
            product_id: Set(dto.product_id),
            color_id: Set(dto.color_id),
            currency: Set(dto.currency),
            base_price: Set(dto.base_price),
            effective_from: Set(dto.effective_from),
            effective_to: Set(dto.effective_to),
            customer_level: Set(dto.customer_level),
            min_quantity: Set(dto.min_quantity),
            notes: Set(dto.notes),
            max_quantity: Set(dto.max_quantity),
            customer_id: Set(dto.customer_id),
            season: Set(dto.season),
            is_active: Set(true),
            priority: Set(dto.priority.unwrap_or(0)),
            created_by: Set(Some(operated_by)),
            approved_by: Set(None),
            approved_at: Set(None),
            approval_status: Set(approval::APPROVED.to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let result = active.insert(&*self.db).await?;
        Ok(result)
    }

    /// 列表查询（分页 + 过滤）
    ///
    /// 批次 263 修复：接入 paginate_with_total 工具函数，消除手写 num_items + fetch_page 重复。
    /// paginate_with_total 内部已做 page.saturating_sub(1) 偏移，调用方不可再减 1。
    /// 补 page.clamp(1, 1000) 防 DoS（page_size 已有 clamp(1,100)）。
    pub async fn list(
        &self,
        query: &ListColorPricesQuery,
    ) -> Result<(Vec<product_color_price::Model>, u64), CrudError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100); // v10 P1-1 修复：page_size clamp(1,100) 防 DoS

        let mut q = ColorPriceEntity::find();

        if let Some(pid) = query.product_id {
            q = q.filter(product_color_price::Column::ProductId.eq(pid));
        }
        if let Some(cid) = query.color_id {
            q = q.filter(product_color_price::Column::ColorId.eq(cid));
        }
        if let Some(cust) = query.customer_id {
            q = q.filter(product_color_price::Column::CustomerId.eq(cust));
        }
        if let Some(lvl) = &query.customer_level {
            q = q.filter(product_color_price::Column::CustomerLevel.eq(lvl.clone()));
        }
        if let Some(season) = &query.season {
            q = q.filter(product_color_price::Column::Season.eq(season.clone()));
        }
        if let Some(curr) = &query.currency {
            q = q.filter(product_color_price::Column::Currency.eq(curr.clone()));
        }
        if let Some(active) = query.is_active {
            q = q.filter(product_color_price::Column::IsActive.eq(active));
        }
        if let Some(status) = &query.approval_status {
            q = q.filter(product_color_price::Column::ApprovalStatus.eq(status.clone()));
        }

        let paginator = q
            .order_by_desc(product_color_price::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;

        Ok((items, total))
    }

    /// 按 ID 查询
    pub async fn get_by_id(
        &self,
        id: i64,
    ) -> Result<product_color_price::Model, CrudError> {
        ColorPriceEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or(CrudError::NotFound)
    }

    /// 更新色号价格
    pub async fn update(
        &self,
        id: i64,
        dto: UpdateColorPriceDto,
    ) -> Result<product_color_price::Model, CrudError> {
        let existing = self.get_by_id(id).await?;

        if let Some(c) = &dto.currency {
            Self::validate_currency(c)?;
        }
        if let Some(s) = &dto.season {
            Self::validate_season(Some(s.as_str()))?;
        }
        if let Some(l) = &dto.customer_level {
            Self::validate_customer_level(Some(l.as_str()))?;
        }

        let mut active: ColorPriceActive = existing.into();
        if let Some(v) = dto.currency {
            active.currency = Set(v);
        }
        if let Some(v) = dto.base_price {
            active.base_price = Set(v);
        }
        if let Some(v) = dto.effective_from {
            active.effective_from = Set(v);
        }
        if let Some(v) = dto.effective_to {
            active.effective_to = Set(Some(v));
        }
        if let Some(v) = dto.customer_level {
            active.customer_level = Set(Some(v));
        }
        if let Some(v) = dto.min_quantity {
            active.min_quantity = Set(Some(v));
        }
        if let Some(v) = dto.max_quantity {
            active.max_quantity = Set(Some(v));
        }
        if let Some(v) = dto.customer_id {
            active.customer_id = Set(Some(v));
        }
        if let Some(v) = dto.season {
            active.season = Set(Some(v));
        }
        if let Some(v) = dto.is_active {
            active.is_active = Set(v);
        }
        if let Some(v) = dto.priority {
            active.priority = Set(v);
        }
        if let Some(v) = dto.notes {
            active.notes = Set(Some(v));
        }
        active.updated_at = Set(Utc::now());

        let result = active.update(&*self.db).await?;
        Ok(result)
    }

    /// 软删除（is_active = false）
    pub async fn delete(&self, id: i64) -> Result<product_color_price::Model, CrudError> {
        let existing = self.get_by_id(id).await?;
        if !existing.is_active {
            return Err(CrudError::InvalidState);
        }
        let mut active: ColorPriceActive = existing.into();
        active.is_active = Set(false);
        active.updated_at = Set(Utc::now());
        let result = active.update(&*self.db).await?;
        Ok(result)
    }

    // ----------------------------------------------------------------------
    // 校验方法
    // ----------------------------------------------------------------------

    fn validate_currency(c: &str) -> Result<(), CrudError> {
        match c {
            "CNY" | "USD" | "EUR" => Ok(()),
            _ => Err(CrudError::Validation(format!(
                "无效的币种: {}（允许: CNY / USD / EUR）",
                c
            ))),
        }
    }

    fn validate_season(s: Option<&str>) -> Result<(), CrudError> {
        match s {
            None => Ok(()),
            Some(v) if v == "SS" || v == "AW" || v == "HOLIDAY" => Ok(()),
            Some(v) => Err(CrudError::Validation(format!(
                "无效的季节: {}（允许: SS / AW / HOLIDAY）",
                v
            ))),
        }
    }

    fn validate_customer_level(l: Option<&str>) -> Result<(), CrudError> {
        match l {
            None => Ok(()),
            Some(v) if v == "VIP" || v == "NORMAL" || v == "GOLD" || v == "SILVER" => Ok(()),
            Some(v) => Err(CrudError::Validation(format!(
                "无效的客户等级: {}（允许: VIP / NORMAL / GOLD / SILVER）",
                v
            ))),
        }
    }

    fn validate_quantity_range(
        min: Option<Decimal>,
        max: Option<Decimal>,
    ) -> Result<(), CrudError> {
        if let (Some(lo), Some(hi)) = (min, max) {
            if lo >= hi {
                return Err(CrudError::Validation(
                    "min_quantity 必须小于 max_quantity".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn validate_date_range(
        from: chrono::NaiveDate,
        until: Option<chrono::NaiveDate>,
    ) -> Result<(), CrudError> {
        if let Some(u) = until {
            if from > u {
                return Err(CrudError::Validation(
                    "effective_from 必须早于 effective_to".to_string(),
                ));
            }
        }
        Ok(())
    }
}
