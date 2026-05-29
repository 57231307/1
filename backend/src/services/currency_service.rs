use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use std::sync::Arc;

use crate::models::currency::{Entity as CurrencyEntity, Model as CurrencyModel};
use crate::models::exchange_rate::{
    ActiveModel as RateActiveModel, Entity as RateEntity, Model as RateModel,
};
use crate::utils::error::AppError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExchangeRateHistoryModel {
    pub id: i32,
    pub from_currency: String,
    pub to_currency: String,
    pub rate: Decimal,
    pub effective_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub source: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversionResult {
    pub from_currency: String,
    pub to_currency: String,
    pub original_amount: Decimal,
    pub converted_amount: Decimal,
    pub exchange_rate: Decimal,
    pub conversion_date: NaiveDate,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExternalRateResponse {
    pub from_currency: String,
    pub to_currency: String,
    pub rate: Decimal,
    pub timestamp: chrono::DateTime<Utc>,
    pub source: String,
}

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

    #[allow(dead_code)]
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

    /// 获取汇率历史记录
    pub async fn get_exchange_rate_history(
        &self,
        from_currency: &str,
        to_currency: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<ExchangeRateHistoryModel>, u64), AppError> {
        let mut select = RateEntity::find()
            .filter(crate::models::exchange_rate::Column::FromCurrency.eq(from_currency))
            .filter(crate::models::exchange_rate::Column::ToCurrency.eq(to_currency));

        if let Some(start) = start_date {
            select = select.filter(crate::models::exchange_rate::Column::EffectiveDate.gte(start));
        }

        if let Some(end) = end_date {
            select = select.filter(crate::models::exchange_rate::Column::EffectiveDate.lte(end));
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

        let history: Vec<ExchangeRateHistoryModel> = models
            .into_iter()
            .map(|m| ExchangeRateHistoryModel {
                id: m.id,
                from_currency: m.from_currency,
                to_currency: m.to_currency,
                rate: m.rate,
                effective_date: m.effective_date,
                end_date: None,
                source: m.status,
                created_at: m.created_at,
            })
            .collect();

        Ok((history, total))
    }

    /// 本位币自动换算
    /// 将指定金额从源币种换算为目标币种（通常是本位币）
    pub async fn convert_amount(
        &self,
        from_currency: &str,
        to_currency: &str,
        amount: Decimal,
        conversion_date: Option<NaiveDate>,
    ) -> Result<ConversionResult, AppError> {
        // 如果源币种和目标币种相同，直接返回
        if from_currency == to_currency {
            return Ok(ConversionResult {
                from_currency: from_currency.to_string(),
                to_currency: to_currency.to_string(),
                original_amount: amount,
                converted_amount: amount,
                exchange_rate: Decimal::ONE,
                conversion_date: conversion_date.unwrap_or_else(|| Utc::now().date_naive()),
            });
        }

        // 获取汇率
        let rate_model = self.get_exchange_rate(from_currency, to_currency).await?;

        let rate = match rate_model {
            Some(model) => model.rate,
            None => {
                // 尝试通过本位币进行间接换算
                let base_currency = self.get_base_currency().await?;
                match base_currency {
                    Some(base) => {
                        let from_to_base =
                            self.get_exchange_rate(from_currency, &base.code).await?;
                        let base_to_target =
                            self.get_exchange_rate(&base.code, to_currency).await?;

                        match (from_to_base, base_to_target) {
                            (Some(f2b), Some(b2t)) => f2b.rate * b2t.rate,
                            _ => {
                                return Err(AppError::BusinessError(format!(
                                    "无法找到 {} 到 {} 的汇率",
                                    from_currency, to_currency
                                )))
                            }
                        }
                    }
                    None => return Err(AppError::BusinessError("未配置本位币".to_string())),
                }
            }
        };

        let converted_amount = amount * rate;

        Ok(ConversionResult {
            from_currency: from_currency.to_string(),
            to_currency: to_currency.to_string(),
            original_amount: amount,
            converted_amount,
            exchange_rate: rate,
            conversion_date: conversion_date.unwrap_or_else(|| Utc::now().date_naive()),
        })
    }

    /// 批量换算到本位币
    #[allow(dead_code)]
    pub async fn convert_to_base_currency(
        &self,
        from_currency: &str,
        amounts: Vec<Decimal>,
    ) -> Result<Vec<ConversionResult>, AppError> {
        let base_currency = self.get_base_currency().await?;
        let base_code = match base_currency {
            Some(base) => base.code,
            None => return Err(AppError::BusinessError("未配置本位币".to_string())),
        };

        let mut results = Vec::new();
        for amount in amounts {
            let result = self
                .convert_amount(from_currency, &base_code, amount, None)
                .await?;
            results.push(result);
        }

        Ok(results)
    }

    /// 获取外部汇率（真实外部API调用）
    pub async fn fetch_external_rate(
        &self,
        from_currency: &str,
        to_currency: &str,
    ) -> Result<ExternalRateResponse, AppError> {
        // 使用免费的汇率API
        let url = format!(
            "https://api.exchangerate-api.com/v4/latest/{}",
            from_currency
        );

        tracing::info!("调用外部汇率API: {} -> {}", from_currency, to_currency);

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| AppError::BusinessError(format!("汇率API请求失败: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::BusinessError(format!(
                "汇率API返回错误: {}",
                response.status()
            )));
        }

        let body = response
            .text()
            .await
            .map_err(|e| AppError::BusinessError(format!("读取汇率API响应失败: {}", e)))?;

        let api_response: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| AppError::BusinessError(format!("解析汇率API响应失败: {}", e)))?;

        let rates = api_response
            .get("rates")
            .and_then(|r| r.as_object())
            .ok_or_else(|| AppError::BusinessError("汇率API响应格式错误".to_string()))?;

        let rate = rates
            .get(to_currency)
            .and_then(|r| r.as_f64())
            .ok_or_else(|| {
                AppError::BusinessError(format!("未找到汇率: {} -> {}", from_currency, to_currency))
            })?;

        let decimal_rate = Decimal::from_f64_retain(rate).unwrap_or(Decimal::ZERO);

        Ok(ExternalRateResponse {
            from_currency: from_currency.to_string(),
            to_currency: to_currency.to_string(),
            rate: decimal_rate,
            timestamp: Utc::now(),
            source: "exchangerate-api.com".to_string(),
        })
    }

    /// 同步外部汇率并保存到数据库
    pub async fn sync_external_rate(
        &self,
        from_currency: &str,
        to_currency: &str,
    ) -> Result<RateModel, AppError> {
        let external_rate = self.fetch_external_rate(from_currency, to_currency).await?;

        let today = Utc::now().date_naive();

        // 保存到数据库
        let model = self
            .create_exchange_rate(
                external_rate.from_currency,
                external_rate.to_currency,
                external_rate.rate,
                today,
            )
            .await?;

        Ok(model)
    }

    /// 计算本位币金额（用于订单和发票）
    #[allow(dead_code)]
    pub async fn calculate_base_amount(
        &self,
        currency_code: &str,
        amount: Decimal,
    ) -> Result<(Decimal, Decimal), AppError> {
        let base_currency = self.get_base_currency().await?;
        let base_code = match base_currency {
            Some(base) => base.code,
            None => return Err(AppError::BusinessError("未配置本位币".to_string())),
        };

        if currency_code == base_code {
            return Ok((amount, Decimal::ONE));
        }

        let result = self
            .convert_amount(currency_code, &base_code, amount, None)
            .await?;
        Ok((result.converted_amount, result.exchange_rate))
    }

    /// 批量同步所有活跃币种的汇率
    pub async fn sync_all_rates(&self) -> Result<Vec<RateModel>, AppError> {
        let base_currency = self.get_base_currency().await?;
        let base_code = match base_currency {
            Some(base) => base.code,
            None => return Err(AppError::BusinessError("未配置本位币".to_string())),
        };

        let currencies = self.list_currencies().await?;
        let mut results = Vec::new();

        for currency in &currencies {
            if currency.code == base_code {
                continue;
            }

            match self.sync_external_rate(&base_code, &currency.code).await {
                Ok(model) => {
                    results.push(model);
                    tracing::info!("同步汇率成功: {} -> {}", base_code, currency.code);
                }
                Err(e) => {
                    tracing::warn!("同步汇率失败: {} -> {} - {}", base_code, currency.code, e);
                }
            }
        }

        Ok(results)
    }

    /// 获取支持的币种列表
    pub async fn get_supported_currencies(&self) -> Result<Vec<String>, AppError> {
        let currencies = self.list_currencies().await?;
        Ok(currencies.iter().map(|c| c.code.clone()).collect())
    }
}
