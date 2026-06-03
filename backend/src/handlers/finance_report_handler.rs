use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Datelike;
use serde::Deserialize;

use crate::services::finance_report_service::{
    BalanceSheet, CashFlowStatement, FinanceReportService, GeneralLedger, IncomeStatement,
    SubsidiaryLedger, TrialBalance,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct DateRangeQuery {
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct PeriodQuery {
    pub period: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubsidiaryLedgerQuery {
    pub dimension_type: String,
    pub dimension_value: String,
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

/// 获取现金流量表
pub async fn get_cash_flow_statement(
    State(state): State<AppState>,
    Query(query): Query<DateRangeQuery>,
) -> Result<Json<ApiResponse<CashFlowStatement>>, AppError> {
    let service = FinanceReportService::new(state.db.clone());
    let start_date = query.start_date.unwrap_or_else(|| {
        chrono::Utc::now()
            .date_naive()
            .with_day(1)
            .unwrap_or_else(|| {
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
    let stmt = service.get_cash_flow_statement(start_date, end_date).await?;
    Ok(Json(ApiResponse::success(stmt)))
}

/// 获取试算平衡表
pub async fn get_trial_balance(
    State(state): State<AppState>,
    Query(query): Query<PeriodQuery>,
) -> Result<Json<ApiResponse<TrialBalance>>, AppError> {
    let service = FinanceReportService::new(state.db.clone());
    let trial_balance = service.get_trial_balance(query.period).await?;
    Ok(Json(ApiResponse::success(trial_balance)))
}

/// 获取总账（按科目代码）
pub async fn get_general_ledger(
    State(state): State<AppState>,
    Path(code): Path<String>,
    Query(query): Query<DateRangeQuery>,
) -> Result<Json<ApiResponse<GeneralLedger>>, AppError> {
    let service = FinanceReportService::new(state.db.clone());
    let start_date = query.start_date.unwrap_or_else(|| {
        chrono::Utc::now()
            .date_naive()
            .with_day(1)
            .unwrap_or_else(|| {
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
    let ledger = service.get_general_ledger(code, start_date, end_date).await?;
    Ok(Json(ApiResponse::success(ledger)))
}

/// 获取明细账（按辅助核算维度）
pub async fn get_subsidiary_ledger(
    State(state): State<AppState>,
    Query(query): Query<SubsidiaryLedgerQuery>,
) -> Result<Json<ApiResponse<SubsidiaryLedger>>, AppError> {
    let service = FinanceReportService::new(state.db.clone());
    let start_date = query.start_date.unwrap_or_else(|| {
        chrono::Utc::now()
            .date_naive()
            .with_day(1)
            .unwrap_or_else(|| {
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
    let ledger = service
        .get_subsidiary_ledger(query.dimension_type, query.dimension_value, start_date, end_date)
        .await?;
    Ok(Json(ApiResponse::success(ledger)))
}
