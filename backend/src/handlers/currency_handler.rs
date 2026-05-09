use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::middleware::auth_context::AuthContext;
use crate::services::currency_service::{
    CurrencyService, CreateCurrencyRequest, CreateExchangeRateRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::response::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct CreateCurrencyApiRequest {
    pub code: String,
    pub name: String,
    pub symbol: Option<String>,
    pub is_base: bool,
    pub precision: i32,
}

#[derive(Debug, Serialize)]
pub struct CurrencyResponse {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub symbol: Option<String>,
    pub is_base: bool,
    pub precision: i32,
    pub is_active: bool,
}

impl From<crate::models::currency::Model> for CurrencyResponse {
    fn from(model: crate::models::currency::Model) -> Self {
        Self {
            id: model.id,
            code: model.code,
            name: model.name,
            symbol: model.symbol,
            is_base: model.is_base,
            precision: model.precision,
            is_active: model.is_active,
        }
    }
}

pub async fn create_currency(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateCurrencyApiRequest>,
) -> Result<Json<ApiResponse<CurrencyResponse>>, StatusCode> {
    let service = CurrencyService::new(state.db);
    let create_req = CreateCurrencyRequest {
        code: req.code,
        name: req.name,
        symbol: req.symbol,
        is_base: req.is_base,
        precision: req.precision,
    };

    match service.create_currency(create_req).await {
        Ok(model) => Ok(Json(ApiResponse::success(CurrencyResponse::from(model)))),
        Err(e) => {
            tracing::error!("创建币种失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn list_currencies(
    State(state): State<AppState>,
    _auth: AuthContext,
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
    _auth: AuthContext,
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
    pub source: Option<String>,
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
    _auth: AuthContext,
    Json(req): Json<CreateExchangeRateApiRequest>,
) -> Result<Json<ApiResponse<ExchangeRateResponse>>, StatusCode> {
    let service = CurrencyService::new(state.db);
    let create_req = CreateExchangeRateRequest {
        from_currency: req.from_currency,
        to_currency: req.to_currency,
        rate: req.rate,
        effective_date: req.effective_date,
        source: req.source,
    };

    match service.create_exchange_rate(create_req).await {
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
    pub date: Option<NaiveDate>,
}

pub async fn get_exchange_rate(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<GetExchangeRateQuery>,
) -> Result<Json<ApiResponse<ExchangeRateResponse>>, StatusCode> {
    let service = CurrencyService::new(state.db);
    let date = query.date.unwrap_or_else(|| chrono::Local::now().naive_local().date());

    match service.get_exchange_rate(&query.from_currency, &query.to_currency, date).await {
        Ok(Some(model)) => Ok(Json(ApiResponse::success(ExchangeRateResponse::from(model)))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("获取汇率失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
