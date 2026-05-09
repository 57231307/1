//! 币种/汇率 Service
//!
//! 提供币种管理和汇率维护功能

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use std::sync::Arc;
use sea_orm::DatabaseConnection;

use crate::models::currency::{
    ActiveModel as CurrencyActiveModel, Entity as CurrencyEntity, Model as CurrencyModel,
};
use crate::models::exchange_rate::{
    ActiveModel as RateActiveModel, Entity as RateEntity, Model as RateModel,
};
use crate::utils::error::AppError;

/// 创建币种请求
#[derive(Debug, Clone)]
pub struct CreateCurrencyRequest {
    pub code: String,
    pub name: String,
    pub symbol: Option<String>,
    pub is_base: bool,
    pub precision: i32,
}

/// 更新币种请求
#[derive(Debug, Clone)]
pub struct UpdateCurrencyRequest {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub is_active: Option<bool>,
    pub precision: Option<i32>,
}

/// 创建汇率请求
#[derive(Debug, Clone)]
pub struct CreateExchangeRateRequest {
    pub from_currency: String,
    pub to_currency: String,
    pub rate: Decimal,
    pub effective_date: NaiveDate,
    pub source: Option<String>,
}

/// 币种/汇率 Service
pub struct CurrencyService {
    db: Arc<DatabaseConnection>,
}

impl CurrencyService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建币种
    pub async fn create_currency(
        &self,
        req: CreateCurrencyRequest,
    ) -> Result<CurrencyModel, AppError> {
        let active_model = CurrencyActiveModel {
            code: Set(req.code),
            name: Set(req.name),
            symbol: Set(req.symbol),
            is_base: Set(req.is_base),
            precision: Set(req.precision),
            is_active: Set(true),
            is_deleted: Set(false),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };

        let model = active_model
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(model)
    }

    /// 获取所有币种
    pub async fn list_currencies(&self) -> Result<Vec<CurrencyModel>, AppError> {
        let models = CurrencyEntity::find()
            .filter(crate::models::currency::Column::IsDeleted.eq(false))
            .order_by_asc(crate::models::currency::Column::Code)
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(models)
    }

    /// 获取本位币
    pub async fn get_base_currency(&self) -> Result<Option<CurrencyModel>, AppError> {
        let model = CurrencyEntity::find()
            .filter(crate::models::currency::Column::IsBase.eq(true))
            .filter(crate::models::currency::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(model)
    }

    /// 更新币种
    pub async fn update_currency(
        &self,
        id: i32,
        req: UpdateCurrencyRequest,
    ) -> Result<CurrencyModel, AppError> {
        let model = CurrencyEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("币种不存在".to_string()))?;

        let mut active_model: CurrencyActiveModel = model.into();

        if let Some(name) = req.name {
            active_model.name = Set(name);
        }
        if let Some(symbol) = req.symbol {
            active_model.symbol = Set(Some(symbol));
        }
        if let Some(is_active) = req.is_active {
            active_model.is_active = Set(is_active);
        }
        if let Some(precision) = req.precision {
            active_model.precision = Set(precision);
        }

        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }

    /// 创建汇率
    pub async fn create_exchange_rate(
        &self,
        req: CreateExchangeRateRequest,
    ) -> Result<RateModel, AppError> {
        let active_model = RateActiveModel {
            from_currency: Set(req.from_currency),
            to_currency: Set(req.to_currency),
            rate: Set(req.rate),
            effective_date: Set(req.effective_date),
            source: Set(req.source),
            is_deleted: Set(false),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };

        let model = active_model
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(model)
    }

    /// 获取汇率
    pub async fn get_exchange_rate(
        &self,
        from_currency: &str,
        to_currency: &str,
        date: NaiveDate,
    ) -> Result<Option<RateModel>, AppError> {
        let model = RateEntity::find()
            .filter(crate::models::exchange_rate::Column::FromCurrency.eq(from_currency))
            .filter(crate::models::exchange_rate::Column::ToCurrency.eq(to_currency))
            .filter(crate::models::exchange_rate::Column::EffectiveDate.lte(date))
            .filter(crate::models::exchange_rate::Column::IsDeleted.eq(false))
            .order_by_desc(crate::models::exchange_rate::Column::EffectiveDate)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(model)
    }

    /// 获取汇率列表
    pub async fn list_exchange_rates(
        &self,
        from_currency: Option<String>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<RateModel>, u64), AppError> {
        let mut select = RateEntity::find()
            .filter(crate::models::exchange_rate::Column::IsDeleted.eq(false));

        if let Some(from) = from_currency {
            select = select.filter(crate::models::exchange_rate::Column::FromCurrency.eq(from));
        }

        let total = select
            .clone()
            .count(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let paginator = select
            .order_by_desc(crate::models::exchange_rate::Column::EffectiveDate)
            .paginate(&*self.db, page_size);

        let models = paginator
            .fetch_page(page - 1)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok((models, total))
    }

    /// 货币转换
    pub async fn convert(
        &self,
        from_currency: &str,
        to_currency: &str,
        amount: Decimal,
        date: NaiveDate,
    ) -> Result<Decimal, AppError> {
        if from_currency == to_currency {
            return Ok(amount);
        }

        let rate = self
            .get_exchange_rate(from_currency, to_currency, date)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "未找到汇率: {} -> {} 在 {}",
                    from_currency, to_currency, date
                ))
            })?;

        Ok(amount * rate.rate)
    }
}
