use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::services::currency_service::{
    ConversionResult, CurrencyService, ExchangeRateHistoryModel,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

#[derive(Debug, Serialize)]
pub struct CurrencyResponse {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub symbol: Option<String>,
    pub is_base: Option<bool>,
    pub precision: Option<i32>,
    pub is_active: Option<bool>,
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

/// BE-A/H 统一（2026-06-26）：错误类型从 StatusCode 改为 AppError，
/// 并使用 `?` 运算符简化错误传播；`AppError: From<sea_orm::DbErr>` 已实现自动转换。
pub async fn list_currencies(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<CurrencyResponse>>>, AppError> {
    let service = CurrencyService::new(state.db);
    let models = service.list_currencies().await?;
    let responses: Vec<CurrencyResponse> =
        models.into_iter().map(CurrencyResponse::from).collect();
    Ok(Json(ApiResponse::success(responses)))
}

pub async fn get_base_currency(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<CurrencyResponse>>, AppError> {
    let service = CurrencyService::new(state.db);
    match service.get_base_currency().await? {
        Some(model) => Ok(Json(ApiResponse::success(CurrencyResponse::from(model)))),
        None => Err(AppError::not_found("未设置本位币")),
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
) -> Result<Json<ApiResponse<ExchangeRateResponse>>, AppError> {
    let service = CurrencyService::new(state.db);
    let model = service
        .create_exchange_rate(
            req.from_currency,
            req.to_currency,
            req.rate,
            req.effective_date,
        )
        .await?;
    Ok(Json(ApiResponse::success(ExchangeRateResponse::from(model))))
}

#[derive(Debug, Deserialize)]
pub struct GetExchangeRateQuery {
    pub from_currency: String,
    pub to_currency: String,
}

pub async fn get_exchange_rate(
    State(state): State<AppState>,
    Query(query): Query<GetExchangeRateQuery>,
) -> Result<Json<ApiResponse<ExchangeRateResponse>>, AppError> {
    let service = CurrencyService::new(state.db);
    match service
        .get_exchange_rate(&query.from_currency, &query.to_currency)
        .await?
    {
        Some(model) => Ok(Json(ApiResponse::success(ExchangeRateResponse::from(model)))),
        None => Err(AppError::not_found(format!(
            "未找到 {} -> {} 的汇率记录",
            query.from_currency, query.to_currency
        ))),
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
) -> Result<Json<ApiResponse<Vec<ExchangeRateResponse>>>, AppError> {
    let service = CurrencyService::new(state.db);
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let (models, _total) = service
        .list_exchange_rates(query.from_currency, page, page_size)
        .await?;
    let responses: Vec<ExchangeRateResponse> =
        models.into_iter().map(ExchangeRateResponse::from).collect();
    Ok(Json(ApiResponse::success(responses)))
}

// ============================================================================
// 增强版合并自 currency_enhanced_handler.rs：汇率历史、金额换算、批量同步、支持币种
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ExchangeRateHistoryQuery {
    pub from_currency: String,
    pub to_currency: String,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ExchangeRateHistoryResponse {
    pub id: i32,
    pub from_currency: String,
    pub to_currency: String,
    pub rate: String,
    pub effective_date: String,
    pub end_date: Option<String>,
    pub source: Option<String>,
    pub created_at: String,
}

impl From<ExchangeRateHistoryModel> for ExchangeRateHistoryResponse {
    fn from(model: ExchangeRateHistoryModel) -> Self {
        Self {
            id: model.id,
            from_currency: model.from_currency,
            to_currency: model.to_currency,
            rate: model.rate.to_string(),
            effective_date: model.effective_date.to_string(),
            end_date: model.end_date.map(|d| d.to_string()),
            source: model.source,
            created_at: model.created_at.to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ConvertAmountRequest {
    pub from_currency: String,
    pub to_currency: String,
    pub amount: Decimal,
    pub conversion_date: Option<NaiveDate>,
}

#[derive(Debug, Serialize)]
pub struct ConversionResultResponse {
    pub from_currency: String,
    pub to_currency: String,
    pub original_amount: String,
    pub converted_amount: String,
    pub exchange_rate: String,
    pub conversion_date: String,
}

impl From<ConversionResult> for ConversionResultResponse {
    fn from(result: ConversionResult) -> Self {
        Self {
            from_currency: result.from_currency,
            to_currency: result.to_currency,
            original_amount: result.original_amount.to_string(),
            converted_amount: result.converted_amount.to_string(),
            exchange_rate: result.exchange_rate.to_string(),
            conversion_date: result.conversion_date.to_string(),
        }
    }
}

/// 获取汇率历史记录
pub async fn get_exchange_rate_history(
    State(state): State<AppState>,
    Query(query): Query<ExchangeRateHistoryQuery>,
) -> Result<Json<ApiResponse<Vec<ExchangeRateHistoryResponse>>>, AppError> {
    let service = CurrencyService::new(state.db);
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let (models, _total) = service
        .get_exchange_rate_history(
            &query.from_currency,
            &query.to_currency,
            query.start_date,
            query.end_date,
            page,
            page_size,
        )
        .await?;
    let responses: Vec<ExchangeRateHistoryResponse> = models
        .into_iter()
        .map(ExchangeRateHistoryResponse::from)
        .collect();
    Ok(Json(ApiResponse::success(responses)))
}

/// 金额换算
///
/// 注意：业务错误（如币种不存在）通过 200 + ApiResponse::error 返回，
/// 让前端可以正常解析业务错误码；其他错误类型直接走 `?` 传播。
pub async fn convert_amount(
    State(state): State<AppState>,
    Json(req): Json<ConvertAmountRequest>,
) -> Result<Json<ApiResponse<ConversionResultResponse>>, AppError> {
    let service = CurrencyService::new(state.db);
    match service
        .convert_amount(
            &req.from_currency,
            &req.to_currency,
            req.amount,
            req.conversion_date,
        )
        .await
    {
        Ok(result) => Ok(Json(ApiResponse::success(ConversionResultResponse::from(
            result,
        )))),
        Err(crate::utils::error::AppError::BusinessError(msg)) => Ok(Json(
            ApiResponse::error_with_status(StatusCode::BAD_REQUEST, msg),
        )),
        Err(e) => Err(e),
    }
}

/// 批量同步所有币种汇率
pub async fn sync_all_rates(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CurrencyService::new(state.db);
    let results = service.sync_all_rates().await?;
    let count = results.len();
    tracing::info!("批量同步汇率成功，共同步 {} 个币种", count);
    Ok(Json(ApiResponse::success(serde_json::json!({
        "synced_count": count,
        "message": format!("成功同步 {} 个币种的汇率", count),
    }))))
}

/// 获取支持的币种列表
pub async fn get_supported_currencies(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<String>>>, AppError> {
    let service = CurrencyService::new(state.db);
    let currencies = service.get_supported_currencies().await?;
    Ok(Json(ApiResponse::success(currencies)))
}
