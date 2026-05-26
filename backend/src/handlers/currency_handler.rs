use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::services::currency_service::CurrencyService;
use crate::utils::app_state::AppState;
use crate::utils::response::ApiResponse;

#[derive(Debug, Serialize)]
pub struct CurrencyResponse {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub symbol: Option<String>,
    pub is_base: Option<bool>,
    pub decimal_places: Option<i32>,
    pub status: Option<String>,
}

impl From<crate::models::currency::Model> for CurrencyResponse {
    fn from(model: crate::models::currency::Model) -> Self {
        Self {
            id: model.id,
            code: model.code,
            name: model.name,
            symbol: model.symbol,
            is_base: model.is_base,
            decimal_places: model.decimal_places,
            status: model.status,
        }
    }
}

pub async fn list_currencies(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<CurrencyResponse>>>, StatusCode> {
    let service = CurrencyService::new(state.db);

    match service.list_currencies().await {
        Ok(models) => {
            let responses: Vec<CurrencyResponse> = models.into_iter().map(CurrencyResponse::from).collect();
            Ok(Json(ApiResponse::success(responses)))
        }
        Err(e) => {
            tracing::error!("获取币种列表失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_base_currency(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<CurrencyResponse>>, StatusCode> {
    let service = CurrencyService::new(state.db);

    match service.get_base_currency().await {
        Ok(Some(model)) => Ok(Json(ApiResponse::success(CurrencyResponse::from(model)))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("获取本位币失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateExchangeRateApiRequest {
    pub from_currency: String,
    pub to_currency: String,
    pub rate: Decimal,
    pub effective_date: NaiveDate,
}

#[derive(Debug, Serialize)]
pub struct ExchangeRateResponse {
    pub id: i32,
    pub from_currency: String,
    pub to_currency: String,
    pub rate: String,
    pub effective_date: String,
}

impl From<crate::models::exchange_rate::Model> for ExchangeRateResponse {
    fn from(model: crate::models::exchange_rate::Model) -> Self {
        Self {
            id: model.id,
            from_currency: model.from_currency,
            to_currency: model.to_currency,
            rate: model.rate.to_string(),
            effective_date: model.effective_date.to_string(),
        }
    }
}

pub async fn create_exchange_rate(
    State(state): State<AppState>,
    Json(req): Json<CreateExchangeRateApiRequest>,
) -> Result<Json<ApiResponse<ExchangeRateResponse>>, StatusCode> {
    let service = CurrencyService::new(state.db);

    match service.create_exchange_rate(req.from_currency, req.to_currency, req.rate, req.effective_date).await {
        Ok(model) => Ok(Json(ApiResponse::success(ExchangeRateResponse::from(model)))),
        Err(e) => {
            tracing::error!("创建汇率失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GetExchangeRateQuery {
    pub from_currency: String,
    pub to_currency: String,
}

pub async fn get_exchange_rate(
    State(state): State<AppState>,
    Query(query): Query<GetExchangeRateQuery>,
) -> Result<Json<ApiResponse<ExchangeRateResponse>>, StatusCode> {
    let service = CurrencyService::new(state.db);

    match service.get_exchange_rate(&query.from_currency, &query.to_currency).await {
        Ok(Some(model)) => Ok(Json(ApiResponse::success(ExchangeRateResponse::from(model)))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("获取汇率失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListExchangeRatesQuery {
    pub from_currency: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

pub async fn list_exchange_rates(
    State(state): State<AppState>,
    Query(query): Query<ListExchangeRatesQuery>,
) -> Result<Json<ApiResponse<Vec<ExchangeRateResponse>>>, StatusCode> {
    let service = CurrencyService::new(state.db);
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    match service.list_exchange_rates(query.from_currency, page, page_size).await {
        Ok((models, _total)) => {
            let responses: Vec<ExchangeRateResponse> = models.into_iter().map(ExchangeRateResponse::from).collect();
            Ok(Json(ApiResponse::success(responses)))
        }
        Err(e) => {
            tracing::error!("获取汇率列表失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
