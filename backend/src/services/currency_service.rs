use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use std::sync::Arc;
use sea_orm::DatabaseConnection;

use crate::models::currency::{
    Entity as CurrencyEntity, Model as CurrencyModel,
};
use crate::models::exchange_rate::{
    ActiveModel as RateActiveModel, Entity as RateEntity, Model as RateModel,
};
use crate::utils::error::AppError;

pub struct CurrencyService {
    db: Arc<DatabaseConnection>,
}

impl CurrencyService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn list_currencies(&self) -> Result<Vec<CurrencyModel>, AppError> {
        let models = CurrencyEntity::find()
            .order_by_asc(crate::models::currency::Column::Code)
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(models)
    }

    pub async fn get_base_currency(&self) -> Result<Option<CurrencyModel>, AppError> {
        let model = CurrencyEntity::find()
            .filter(crate::models::currency::Column::IsBase.eq(true))
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(model)
    }

    pub async fn list_exchange_rates(
        &self,
        from_currency: Option<String>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<RateModel>, u64), AppError> {
        let mut select = RateEntity::find();

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

    pub async fn get_exchange_rate(
        &self,
        from_currency: &str,
        to_currency: &str,
    ) -> Result<Option<RateModel>, AppError> {
        let model = RateEntity::find()
            .filter(crate::models::exchange_rate::Column::FromCurrency.eq(from_currency))
            .filter(crate::models::exchange_rate::Column::ToCurrency.eq(to_currency))
            .order_by_desc(crate::models::exchange_rate::Column::EffectiveDate)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(model)
    }

    pub async fn create_exchange_rate(
        &self,
        from_currency: String,
        to_currency: String,
        rate: Decimal,
        effective_date: NaiveDate,
    ) -> Result<RateModel, AppError> {
        let active_model = RateActiveModel {
            from_currency: Set(from_currency),
            to_currency: Set(to_currency),
            rate: Set(rate),
            effective_date: Set(effective_date),
            status: Set(Some("active".to_string())),
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
}
