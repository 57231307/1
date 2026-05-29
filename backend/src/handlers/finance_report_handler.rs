use axum::{
    extract::{Query, State},
    Json,
};
use chrono::Datelike;
use serde::Deserialize;

use crate::services::finance_report_service::{
    BalanceSheet, FinanceReportService, IncomeStatement,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct DateRangeQuery {
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
}

/// 获取资产负债表
pub async fn get_balance_sheet(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<BalanceSheet>>, AppError> {
    let service = FinanceReportService::new(state.db.clone());
    let sheet = service.get_balance_sheet().await?;
    Ok(Json(ApiResponse::success(sheet)))
}

/// 获取利润表
pub async fn get_income_statement(
    State(state): State<AppState>,
    Query(query): Query<DateRangeQuery>,
) -> Result<Json<ApiResponse<IncomeStatement>>, AppError> {
    let service = FinanceReportService::new(state.db.clone());
    let start_date = query.start_date.unwrap_or_else(|| {
        chrono::Utc::now()
            .date_naive()
            .with_day(1)
            .unwrap_or_else(|| {
                // Fallback to first day of month
                chrono::NaiveDate::from_ymd_opt(
                    chrono::Utc::now().date_naive().year(),
                    chrono::Utc::now().date_naive().month(),
                    1,
                )
                .unwrap_or(chrono::Utc::now().date_naive())
            })
    });
    let end_date = query
        .end_date
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let stmt = service.get_income_statement(start_date, end_date).await?;
    Ok(Json(ApiResponse::success(stmt)))
}
