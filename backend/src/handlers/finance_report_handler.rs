use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Datelike;
use serde::Deserialize;

use crate::services::finance_report_service::{
    BalanceSheet, CashFlowStatement, FinanceReportService, GeneralLedger, IncomeStatement,
    SubsidiaryLedger, TrialBalance, VoucherItemDetail,
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
    let stmt = service
        .get_cash_flow_statement(start_date, end_date)
        .await?;
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
    let ledger = service
        .get_general_ledger(code, start_date, end_date)
        .await?;
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
        .get_subsidiary_ledger(
            query.dimension_type,
            query.dimension_value,
            start_date,
            end_date,
        )
        .await?;
    Ok(Json(ApiResponse::success(ledger)))
}

/// F-P2-2 修复（批次 387 v13 复审）：报表穿透查询参数
#[derive(Debug, Deserialize)]
pub struct DrillDownQuery {
    /// 报表类型：balance_sheet / income_statement / cash_flow / trial_balance
    pub report_type: String,
    /// 科目编码前缀（balance_sheet/income_statement/cash_flow 用）
    pub subject_prefix: Option<String>,
    /// 科目编码（trial_balance 用）
    pub subject_code: Option<String>,
    /// 期间 YYYY-MM（trial_balance 用）
    pub period: Option<String>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
}

/// F-P2-2 修复（批次 387 v13 复审）：报表穿透到凭证明细
pub async fn drill_down_report(
    State(state): State<AppState>,
    Query(query): Query<DrillDownQuery>,
) -> Result<Json<ApiResponse<Vec<VoucherItemDetail>>>, AppError> {
    let service = FinanceReportService::new(state.db.clone());
    let details = match query.report_type.as_str() {
        "trial_balance" => {
            let period = query
                .period
                .ok_or_else(|| AppError::validation("trial_balance 穿透需要 period 参数"))?;
            let subject_code = query
                .subject_code
                .ok_or_else(|| AppError::validation("trial_balance 穿透需要 subject_code 参数"))?;
            service
                .drill_down_by_period_and_subject(period, subject_code)
                .await?
        }
        _ => {
            let subject_prefix = query.subject_prefix.ok_or_else(|| {
                AppError::validation("报表穿透需要 subject_prefix 参数")
            })?;
            let start_date = query.start_date.unwrap_or_else(|| {
                chrono::NaiveDate::from_ymd_opt(chrono::Utc::now().year(), 1, 1)
                    .unwrap_or_else(|| chrono::Utc::now().date_naive())
            });
            let end_date = query
                .end_date
                .unwrap_or_else(|| chrono::Utc::now().date_naive());
            service
                .drill_down_by_subject_prefix(subject_prefix, start_date, end_date)
                .await?
        }
    };
    Ok(Json(ApiResponse::success(details)))
}
