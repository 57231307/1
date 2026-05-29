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
use crate::utils::response::ApiResponse;

// =====================================================
// 请求/响应结构体
// =====================================================

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

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct BaseCurrencyResponse {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub symbol: Option<String>,
    pub decimal_places: Option<i32>,
}

// =====================================================
// Handler 函数
// =====================================================

/// GET /api/v1/erp/currencies/rates/history
/// 获取汇率历史记录
pub async fn get_exchange_rate_history(
    State(state): State<AppState>,
    Query(query): Query<ExchangeRateHistoryQuery>,
) -> Result<Json<ApiResponse<Vec<ExchangeRateHistoryResponse>>>, StatusCode> {
    let service = CurrencyService::new(state.db);

    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    match service
        .get_exchange_rate_history(
            &query.from_currency,
            &query.to_currency,
            query.start_date,
            query.end_date,
            page,
            page_size,
        )
        .await
    {
        Ok((models, _total)) => {
            let responses: Vec<ExchangeRateHistoryResponse> = models
                .into_iter()
                .map(ExchangeRateHistoryResponse::from)
                .collect();
            Ok(Json(ApiResponse::success(responses)))
        }
        Err(e) => {
            tracing::error!("获取汇率历史失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /api/v1/erp/currencies/convert
/// 金额换算
pub async fn convert_amount(
    State(state): State<AppState>,
    Json(req): Json<ConvertAmountRequest>,
) -> Result<Json<ApiResponse<ConversionResultResponse>>, StatusCode> {
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
        Err(e) => {
            tracing::error!("金额换算失败: {}", e);
            match e {
                crate::utils::error::AppError::BusinessError(msg) => Ok(Json(
                    ApiResponse::error_with_status(StatusCode::BAD_REQUEST, msg),
                )),
                _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    }
}

/// GET /api/v1/erp/currencies/base
/// 获取本位币信息
#[allow(dead_code)]
pub async fn get_base_currency(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<BaseCurrencyResponse>>, StatusCode> {
    let service = CurrencyService::new(state.db);

    match service.get_base_currency().await {
        Ok(Some(model)) => Ok(Json(ApiResponse::success(BaseCurrencyResponse {
            id: model.id,
            code: model.code,
            name: model.name,
            symbol: model.symbol,
            decimal_places: model.decimal_places,
        }))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("获取本位币失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /api/v1/erp/currencies/sync-all
/// 批量同步所有币种汇率
pub async fn sync_all_rates(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    let service = CurrencyService::new(state.db);

    match service.sync_all_rates().await {
        Ok(results) => {
            let count = results.len();
            tracing::info!("批量同步汇率成功，共同步 {} 个币种", count);
            Ok(Json(ApiResponse::success(serde_json::json!({
                "synced_count": count,
                "message": format!("成功同步 {} 个币种的汇率", count),
            }))))
        }
        Err(e) => {
            tracing::error!("批量同步汇率失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v1/erp/currencies/supported
/// 获取支持的币种列表
pub async fn get_supported_currencies(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<String>>>, StatusCode> {
    let service = CurrencyService::new(state.db);

    match service.get_supported_currencies().await {
        Ok(currencies) => Ok(Json(ApiResponse::success(currencies))),
        Err(e) => {
            tracing::error!("获取支持币种列表失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
