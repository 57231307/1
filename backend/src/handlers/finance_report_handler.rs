use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Datelike;
use serde::Deserialize;

use crate::container::AppState;
use crate::services::finance_report_service::{
    BalanceSheet, CashFlowStatement, FinanceReportService, GeneralLedger, IncomeStatement,
    SubsidiaryLedger, TrialBalance, VoucherItemDetail,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use crate::utils::xlsx_export::{build_xlsx_response, XlsxTable};

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

#[derive(Debug, Deserialize)]
pub struct GeneralLedgerQuery {
    pub subject_code: String,
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
            let subject_prefix = query
                .subject_prefix
                .ok_or_else(|| AppError::validation("报表穿透需要 subject_prefix 参数"))?;
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

/// V15 P0 5-1 修复：导出试算平衡表为 xlsx
pub async fn export_trial_balance(
    State(state): State<AppState>,
    Query(query): Query<PeriodQuery>,
) -> Result<axum::response::Response, AppError> {
    const EXPORT_LIMIT: usize = 10000;
    let service = FinanceReportService::new(state.db.clone());
    let trial_balance = service.get_trial_balance(query.period).await?;

    let headers = vec![
        "科目编码".to_string(),
        "科目名称".to_string(),
        "期初借方".to_string(),
        "期初贷方".to_string(),
        "本期借方".to_string(),
        "本期贷方".to_string(),
        "期末借方".to_string(),
        "期末贷方".to_string(),
    ];

    let rows: Vec<Vec<String>> = trial_balance
        .entries
        .iter()
        .take(EXPORT_LIMIT)
        .map(|item| {
            vec![
                item.subject_code.clone(),
                item.subject_name.clone(),
                item.initial_debit.to_string(),
                item.initial_credit.to_string(),
                item.period_debit.to_string(),
                item.period_credit.to_string(),
                item.ending_debit.to_string(),
                item.ending_credit.to_string(),
            ]
        })
        .collect();

    let table = XlsxTable {
        sheet_name: "试算平衡表".to_string(),
        headers,
        rows,
    };

    build_xlsx_response(&table, "trial_balance_export")
}

/// V15 P0 5-1 修复：导出资产负债表为 xlsx
pub async fn export_balance_sheet(
    State(state): State<AppState>,
) -> Result<axum::response::Response, AppError> {
    const EXPORT_LIMIT: usize = 10000;
    let service = FinanceReportService::new(state.db.clone());
    let balance_sheet = service.get_balance_sheet().await?;

    let headers = vec![
        "类别".to_string(),
        "项目名称".to_string(),
        "金额".to_string(),
        "说明".to_string(),
    ];

    let mut rows: Vec<Vec<String>> = Vec::new();
    // 资产
    for item in balance_sheet.assets.iter().take(EXPORT_LIMIT) {
        rows.push(vec![
            "资产".to_string(),
            item.name.clone(),
            item.amount.to_string(),
            item.description.clone().unwrap_or_default(),
        ]);
    }
    // 负债
    for item in balance_sheet.liabilities.iter().take(EXPORT_LIMIT) {
        rows.push(vec![
            "负债".to_string(),
            item.name.clone(),
            item.amount.to_string(),
            item.description.clone().unwrap_or_default(),
        ]);
    }
    // 所有者权益
    for item in balance_sheet.equity.iter().take(EXPORT_LIMIT) {
        rows.push(vec![
            "所有者权益".to_string(),
            item.name.clone(),
            item.amount.to_string(),
            item.description.clone().unwrap_or_default(),
        ]);
    }

    let table = XlsxTable {
        sheet_name: "资产负债表".to_string(),
        headers,
        rows,
    };

    build_xlsx_response(&table, "balance_sheet_export")
}

/// V15 P0 5-1 修复：导出利润表为 xlsx
pub async fn export_income_statement(
    State(state): State<AppState>,
    Query(query): Query<PeriodQuery>,
) -> Result<axum::response::Response, AppError> {
    const EXPORT_LIMIT: usize = 10000;
    let service = FinanceReportService::new(state.db.clone());
    let income_statement = service.get_income_statement(query.period).await?;

    let headers = vec![
        "类别".to_string(),
        "项目名称".to_string(),
        "金额".to_string(),
        "说明".to_string(),
    ];

    let mut rows: Vec<Vec<String>> = Vec::new();
    // 收入
    for item in income_statement.revenue.iter().take(EXPORT_LIMIT) {
        rows.push(vec![
            "收入".to_string(),
            item.name.clone(),
            item.amount.to_string(),
            item.description.clone().unwrap_or_default(),
        ]);
    }
    // 营业费用
    for item in income_statement.operating_expenses.iter().take(EXPORT_LIMIT) {
        rows.push(vec![
            "营业费用".to_string(),
            item.name.clone(),
            item.amount.to_string(),
            item.description.clone().unwrap_or_default(),
        ]);
    }

    let table = XlsxTable {
        sheet_name: "利润表".to_string(),
        headers,
        rows,
    };

    build_xlsx_response(&table, "income_statement_export")
}

/// V15 P0 5-1 修复：导出现金流量表为 xlsx
pub async fn export_cash_flow_statement(
    State(state): State<AppState>,
    Query(query): Query<PeriodQuery>,
) -> Result<axum::response::Response, AppError> {
    const EXPORT_LIMIT: usize = 10000;
    let service = FinanceReportService::new(state.db.clone());
    let cash_flow = service.get_cash_flow_statement(query.period).await?;

    let headers = vec![
        "类别".to_string(),
        "项目名称".to_string(),
        "金额".to_string(),
        "说明".to_string(),
    ];

    let mut rows: Vec<Vec<String>> = Vec::new();
    // 经营活动
    for item in cash_flow.operating_activities.iter().take(EXPORT_LIMIT) {
        rows.push(vec![
            "经营活动".to_string(),
            item.name.clone(),
            item.amount.to_string(),
            item.description.clone().unwrap_or_default(),
        ]);
    }
    // 投资活动
    for item in cash_flow.investing_activities.iter().take(EXPORT_LIMIT) {
        rows.push(vec![
            "投资活动".to_string(),
            item.name.clone(),
            item.amount.to_string(),
            item.description.clone().unwrap_or_default(),
        ]);
    }
    // 筹资活动
    for item in cash_flow.financing_activities.iter().take(EXPORT_LIMIT) {
        rows.push(vec![
            "筹资活动".to_string(),
            item.name.clone(),
            item.amount.to_string(),
            item.description.clone().unwrap_or_default(),
        ]);
    }

    let table = XlsxTable {
        sheet_name: "现金流量表".to_string(),
        headers,
        rows,
    };

    build_xlsx_response(&table, "cash_flow_statement_export")
}

/// V15 P0 5-1 修复：导出总账为 xlsx
pub async fn export_general_ledger(
    State(state): State<AppState>,
    Query(query): Query<GeneralLedgerQuery>,
) -> Result<axum::response::Response, AppError> {
    const EXPORT_LIMIT: usize = 10000;
    let service = FinanceReportService::new(state.db.clone());
    let start_date = query.start_date.unwrap_or_else(|| {
        chrono::Utc::now().date_naive().with_day(1).unwrap_or_else(|| chrono::Utc::now().date_naive())
    });
    let end_date = query.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());
    let ledger = service
        .get_general_ledger(query.subject_code, start_date, end_date)
        .await?;

    let headers = vec![
        "凭证日期".to_string(),
        "凭证号".to_string(),
        "行号".to_string(),
        "摘要".to_string(),
        "借方".to_string(),
        "贷方".to_string(),
        "方向".to_string(),
        "余额".to_string(),
    ];

    let rows: Vec<Vec<String>> = ledger
        .entries
        .iter()
        .take(EXPORT_LIMIT)
        .map(|item| {
            vec![
                item.voucher_date.clone(),
                item.voucher_no.clone(),
                item.line_no.to_string(),
                item.summary.clone().unwrap_or_default(),
                item.debit.to_string(),
                item.credit.to_string(),
                item.direction.clone(),
                item.balance.to_string(),
            ]
        })
        .collect();

    let table = XlsxTable {
        sheet_name: format!("总账-{}", ledger.subject_name),
        headers,
        rows,
    };

    build_xlsx_response(&table, "general_ledger_export")
}

/// V15 P0 5-1 修复：导出明细账为 xlsx
pub async fn export_subsidiary_ledger(
    State(state): State<AppState>,
    Query(query): Query<SubsidiaryLedgerQuery>,
) -> Result<axum::response::Response, AppError> {
    const EXPORT_LIMIT: usize = 10000;
    let service = FinanceReportService::new(state.db.clone());
    let start_date = query.start_date.unwrap_or_else(|| {
        chrono::Utc::now().date_naive().with_day(1).unwrap_or_else(|| chrono::Utc::now().date_naive())
    });
    let end_date = query.end_date.unwrap_or_else(|| chrono::Utc::now().date_naive());
    let ledger = service
        .get_subsidiary_ledger(
            query.dimension_type,
            query.dimension_value,
            start_date,
            end_date,
        )
        .await?;

    let headers = vec![
        "业务日期".to_string(),
        "业务编号".to_string(),
        "业务类型".to_string(),
        "科目编码".to_string(),
        "科目名称".to_string(),
        "摘要".to_string(),
        "借方".to_string(),
        "贷方".to_string(),
    ];

    let rows: Vec<Vec<String>> = ledger
        .entries
        .iter()
        .take(EXPORT_LIMIT)
        .map(|item| {
            vec![
                item.business_date.clone(),
                item.business_no.clone(),
                item.business_type.clone(),
                item.subject_code.clone(),
                item.subject_name.clone(),
                item.summary.clone().unwrap_or_default(),
                item.debit.to_string(),
                item.credit.to_string(),
            ]
        })
        .collect();

    let table = XlsxTable {
        sheet_name: format!("明细账-{}", ledger.dimension_value),
        headers,
        rows,
    };

    build_xlsx_response(&table, "subsidiary_ledger_export")
}
