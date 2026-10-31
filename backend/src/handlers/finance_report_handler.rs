use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::Datelike;
use serde::Deserialize;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::dto::finance_report_dto::{
    BalanceSheet, CashFlowStatement, GeneralLedger, IncomeStatement, SubsidiaryLedger,
    TrialBalance, VoucherItemDetail,
};
use crate::services::export_approval_service::ExportApprovalService;
use crate::services::finance_report_service::FinanceReportService;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use crate::utils::xlsx_export::{XlsxTable, build_xlsx_response};

/// 财务报表的导出范围声明键。
///
/// finance_report 是一个资源类型、六个导出端点，申请侧落库的范围键由前端注册表统一
/// 成同一份词汇表（report_type、period、start_date、end_date、subject_code、
/// dimension_type、dimension_value）。各导出端点把自己不消费的声明键也收进查询结构，
/// 受理门才能把本次请求的范围与批准范围逐字比对；消费与否由各端点的取数逻辑决定。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct DateRangeQuery {
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    /// 以下为申请侧词汇表的声明键，本结构消费的取数键只有 start_date 与 end_date
    pub period: Option<String>,
    pub subject_code: Option<String>,
    pub dimension_type: Option<String>,
    pub dimension_value: Option<String>,
    /// 敏感导出 fail-closed：导出审批令牌
    pub download_token: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct PeriodQuery {
    pub period: Option<String>,
    /// 申请侧词汇表的声明键，本结构消费的取数键只有 period
    pub subject_code: Option<String>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub dimension_type: Option<String>,
    pub dimension_value: Option<String>,
    /// 敏感导出 fail-closed：导出审批令牌
    pub download_token: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct SubsidiaryLedgerQuery {
    pub dimension_type: String,
    pub dimension_value: String,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    /// 申请侧词汇表的声明键，本结构消费的取数键是上面四项
    pub period: Option<String>,
    pub subject_code: Option<String>,
    /// 敏感导出 fail-closed：导出审批令牌
    pub download_token: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct GeneralLedgerQuery {
    pub subject_code: String,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    /// 申请侧词汇表的声明键，本结构消费的取数键是上面三项
    pub period: Option<String>,
    pub dimension_type: Option<String>,
    pub dimension_value: Option<String>,
    /// 敏感导出 fail-closed：导出审批令牌
    pub download_token: Option<String>,
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
                    chrono::Datelike::year(&chrono::Utc::now().date_naive()),
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
                    chrono::Datelike::year(&chrono::Utc::now().date_naive()),
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
                    chrono::Datelike::year(&chrono::Utc::now().date_naive()),
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
                    chrono::Datelike::year(&chrono::Utc::now().date_naive()),
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
#[allow(dead_code, reason = "反序列化输入字段")]
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
            let period = query.period.ok_or_else(|| {
                AppError::validation_displayable("trial_balance 穿透需要 period 参数")
            })?;
            let subject_code = query.subject_code.ok_or_else(|| {
                AppError::validation_displayable("trial_balance 穿透需要 subject_code 参数")
            })?;
            service
                .drill_down_by_period_and_subject(period, subject_code)
                .await?
        }
        _ => {
            let subject_prefix = query.subject_prefix.ok_or_else(|| {
                AppError::validation_displayable("报表穿透需要 subject_prefix 参数")
            })?;
            let start_date = query.start_date.unwrap_or_else(|| {
                chrono::NaiveDate::from_ymd_opt(
                    chrono::Datelike::year(&chrono::Utc::now().date_naive()),
                    1,
                    1,
                )
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

/// 财务报表导出受理门用的范围参数组装。
///
/// 功能：把本次请求声明的财务导出范围键收敛成对象，只收本次实际带的键
/// 调用方：本文件六个财务报表导出端点的受理门
/// 入参：report_type 取端点身份常量，其余六项取申请侧词汇表的声明键
/// 传给谁：ExportApprovalService::enforce_export_download_secured 的 requested_params，
/// 由 validate_scope_against_approval 与批准参数逐项比对
/// 存什么：不落库，只参与受理门比对
/// 存哪里：不适用
///
/// report_type 不取请求参数，取端点身份，避免用别的报表种类的批准令牌打到本端点。
/// 日期按 ISO 面输出，与 serde 对 NaiveDate 的序列化形态一致。
fn finance_scope_params(
    report_type: &str,
    period: &Option<String>,
    subject_code: &Option<String>,
    start_date: Option<chrono::NaiveDate>,
    end_date: Option<chrono::NaiveDate>,
    dimension_type: &Option<String>,
    dimension_value: &Option<String>,
) -> serde_json::Value {
    ExportApprovalService::scope_params(&[
        ("report_type", Some(serde_json::Value::from(report_type))),
        ("period", period.as_deref().map(serde_json::Value::from)),
        (
            "subject_code",
            subject_code.as_deref().map(serde_json::Value::from),
        ),
        (
            "start_date",
            start_date.map(|d| serde_json::Value::from(d.to_string())),
        ),
        (
            "end_date",
            end_date.map(|d| serde_json::Value::from(d.to_string())),
        ),
        (
            "dimension_type",
            dimension_type.as_deref().map(serde_json::Value::from),
        ),
        (
            "dimension_value",
            dimension_value.as_deref().map(serde_json::Value::from),
        ),
    ])
}

/// 导出试算平衡表为 xlsx，端点为 GET /api/v1/erp/finance/reports/trial-balance/export
///
/// 功能：按本次请求的期间取试算平衡表并落成 xlsx
/// 调用方：财务路由的试算平衡表导出端点
/// 入参：查询参数（period 与审批令牌）、应用状态、会话上下文
/// 传给谁：受理门交 ExportApprovalService::enforce_export_download_secured，导出腿交
/// run_export_leg，取数交 FinanceReportService::get_trial_balance，成功登记交 record_download
/// 存什么：不写业务表；科目行写进导出文件，令牌消费写导出审批表
/// 存哪里：不适用
///
/// report_type 按本端点身份写成常量 trial_balance，与前端注册表把申请分流到本端点的
/// 取值同源；端点身份不取请求参数，避免用别的 report_type 的令牌打到本端点。
/// period 未带时服务侧按当期取数，因此本请求没带期间就不进范围参数。
pub async fn export_trial_balance(
    State(state): State<AppState>,
    Query(query): Query<PeriodQuery>,
    auth: AuthContext,
) -> Result<axum::response::Response, AppError> {
    // 敏感导出 fail-closed：受理门走加固入口（申请人本人 + 批准范围逐项比对）
    let approval_svc = ExportApprovalService::new(state.db.clone());
    let requested_scope = finance_scope_params(
        "trial_balance",
        &query.period,
        &query.subject_code,
        query.start_date,
        query.end_date,
        &query.dimension_type,
        &query.dimension_value,
    );
    let approval = approval_svc
        .enforce_export_download_secured(
            query.download_token.as_deref(),
            "finance_report",
            auth.user_id,
            Some(requested_scope),
        )
        .await?;

    let table = approval_svc
        .run_export_leg(approval.id, async {
            const EXPORT_LIMIT: usize = 10000;
            let service = FinanceReportService::new(state.db.clone());
            let trial_balance = service.get_trial_balance(query.period.clone()).await?;

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

            // 敏感导出 fail-closed：记录令牌消费
            approval_svc
                .record_download(
                    approval.id,
                    "trial_balance_export".to_string(),
                    table.rows.len() as i64,
                    String::new(),
                )
                .await?;

            Ok(table)
        })
        .await?;

    build_xlsx_response(&table, "trial_balance_export")
}

/// 导出资产负债表为 xlsx，端点为 GET /api/v1/erp/finance/reports/balance-sheet/export
///
/// 功能：取资产负债表并落成 xlsx
/// 调用方：财务路由的资产负债表导出端点
/// 入参：查询参数（period 与审批令牌）、应用状态、会话上下文
/// 传给谁：受理门交 ExportApprovalService::enforce_export_download_secured，导出腿交
/// run_export_leg，取数交 FinanceReportService::get_balance_sheet，成功登记交 record_download
/// 存什么：不写业务表；资产、负债与所有者权益行写进导出文件，令牌消费写导出审批表
/// 存哪里：不适用
///
/// report_type 按本端点身份写成常量 balance_sheet，与前端注册表把申请分流到本端点的
/// 取值同源。注意服务侧的 get_balance_sheet 不接期间参数，本次申请带期间时该期间
/// 只参与范围比对，不改变取数窗口。
pub async fn export_balance_sheet(
    State(state): State<AppState>,
    Query(query): Query<PeriodQuery>,
    auth: AuthContext,
) -> Result<axum::response::Response, AppError> {
    // 敏感导出 fail-closed：受理门走加固入口（申请人本人 + 批准范围逐项比对）
    let approval_svc = ExportApprovalService::new(state.db.clone());
    let requested_scope = finance_scope_params(
        "balance_sheet",
        &query.period,
        &query.subject_code,
        query.start_date,
        query.end_date,
        &query.dimension_type,
        &query.dimension_value,
    );
    let approval = approval_svc
        .enforce_export_download_secured(
            query.download_token.as_deref(),
            "finance_report",
            auth.user_id,
            Some(requested_scope),
        )
        .await?;

    let table = approval_svc
        .run_export_leg(approval.id, async {
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

            // 敏感导出 fail-closed：记录令牌消费
            approval_svc
                .record_download(
                    approval.id,
                    "balance_sheet_export".to_string(),
                    table.rows.len() as i64,
                    String::new(),
                )
                .await?;

            Ok(table)
        })
        .await?;

    build_xlsx_response(&table, "balance_sheet_export")
}

/// 导出利润表为 xlsx，端点为 GET /api/v1/erp/finance/reports/income-statement/export
///
/// 功能：按本次请求的日期区间取利润表并落成 xlsx
/// 调用方：财务路由的利润表导出端点
/// 入参：查询参数（start_date、end_date 与申请侧声明键及审批令牌）、应用状态、会话上下文
/// 传给谁：受理门交 ExportApprovalService::enforce_export_download_secured，导出腿交
/// run_export_leg，取数交 FinanceReportService::get_income_statement，成功登记交 record_download
/// 存什么：不写业务表；收入与营业费用行写进导出文件，令牌消费写导出审批表
/// 存哪里：不适用
///
/// report_type 按本端点身份写成常量 income_statement，与前端注册表把申请分流到本端点
/// 的取值同源，不取请求参数里的 report_type。取数只消费 start_date 与 end_date，
/// 两者缺省时按当月起到今天取数；period 等其余声明键只参与范围比对，不改变取数窗口。
pub async fn export_income_statement(
    State(state): State<AppState>,
    Query(query): Query<DateRangeQuery>,
    auth: AuthContext,
) -> Result<axum::response::Response, AppError> {
    // 敏感导出 fail-closed：受理门走加固入口（申请人本人 + 批准范围逐项比对）
    let approval_svc = ExportApprovalService::new(state.db.clone());
    let requested_scope = finance_scope_params(
        "income_statement",
        &query.period,
        &query.subject_code,
        query.start_date,
        query.end_date,
        &query.dimension_type,
        &query.dimension_value,
    );
    let approval = approval_svc
        .enforce_export_download_secured(
            query.download_token.as_deref(),
            "finance_report",
            auth.user_id,
            Some(requested_scope),
        )
        .await?;

    let table = approval_svc
        .run_export_leg(approval.id, async {
            const EXPORT_LIMIT: usize = 10000;
            let service = FinanceReportService::new(state.db.clone());
            let start_date = query.start_date.unwrap_or_else(|| {
                chrono::Utc::now()
                    .date_naive()
                    .with_day(1)
                    .unwrap_or_else(|| chrono::Utc::now().date_naive())
            });
            let end_date = query
                .end_date
                .unwrap_or_else(|| chrono::Utc::now().date_naive());
            let income_statement = service.get_income_statement(start_date, end_date).await?;

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
            for item in income_statement
                .operating_expenses
                .iter()
                .take(EXPORT_LIMIT)
            {
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

            // 敏感导出 fail-closed：记录令牌消费
            approval_svc
                .record_download(
                    approval.id,
                    "income_statement_export".to_string(),
                    table.rows.len() as i64,
                    String::new(),
                )
                .await?;

            Ok(table)
        })
        .await?;

    build_xlsx_response(&table, "income_statement_export")
}

/// V15 P0 5-1 修复：导出现金流量表为 xlsx
/// 导出现金流量表为 xlsx，端点为 GET /api/v1/erp/finance/reports/cash-flow/export
///
/// 功能：按本次请求的日期区间取现金流量表并落成 xlsx
/// 调用方：财务路由的现金流量表导出端点
/// 入参：查询参数（start_date、end_date 与申请侧声明键及审批令牌）、应用状态、会话上下文
/// 传给谁：受理门交 ExportApprovalService::enforce_export_download_secured，导出腿交
/// run_export_leg，取数交 FinanceReportService::get_cash_flow_statement，成功登记交 record_download
/// 存什么：不写业务表；经营、投资与筹资活动行写进导出文件，令牌消费写导出审批表
/// 存哪里：不适用
///
/// report_type 按本端点身份写成常量 cash_flow。取数只消费 start_date 与 end_date，
/// 两者缺省时按当月起到今天取数；period 等其余声明键只参与范围比对，不改变取数窗口。
pub async fn export_cash_flow_statement(
    State(state): State<AppState>,
    Query(query): Query<DateRangeQuery>,
    auth: AuthContext,
) -> Result<axum::response::Response, AppError> {
    // 敏感导出 fail-closed：受理门走加固入口（申请人本人 + 批准范围逐项比对）
    let approval_svc = ExportApprovalService::new(state.db.clone());
    let requested_scope = finance_scope_params(
        "cash_flow",
        &query.period,
        &query.subject_code,
        query.start_date,
        query.end_date,
        &query.dimension_type,
        &query.dimension_value,
    );
    let approval = approval_svc
        .enforce_export_download_secured(
            query.download_token.as_deref(),
            "finance_report",
            auth.user_id,
            Some(requested_scope),
        )
        .await?;

    let table = approval_svc
        .run_export_leg(approval.id, async {
            const EXPORT_LIMIT: usize = 10000;
            let service = FinanceReportService::new(state.db.clone());
            let start_date = query.start_date.unwrap_or_else(|| {
                chrono::Utc::now()
                    .date_naive()
                    .with_day(1)
                    .unwrap_or_else(|| chrono::Utc::now().date_naive())
            });
            let end_date = query
                .end_date
                .unwrap_or_else(|| chrono::Utc::now().date_naive());
            let cash_flow = service
                .get_cash_flow_statement(start_date, end_date)
                .await?;

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

            // 敏感导出 fail-closed：记录令牌消费
            approval_svc
                .record_download(
                    approval.id,
                    "cash_flow_statement_export".to_string(),
                    table.rows.len() as i64,
                    String::new(),
                )
                .await?;

            Ok(table)
        })
        .await?;

    build_xlsx_response(&table, "cash_flow_statement_export")
}

/// 导出总账为 xlsx，端点为 GET /api/v1/erp/finance/reports/general-ledger/export
///
/// 功能：按本次请求的科目与日期区间取总账并落成 xlsx
/// 调用方：财务路由的总账导出端点
/// 入参：查询参数（subject_code 必填、start_date、end_date 与申请侧声明键及审批令牌）、
/// 应用状态、会话上下文
/// 传给谁：受理门交 ExportApprovalService::enforce_export_download_secured，导出腿交
/// run_export_leg，取数交 FinanceReportService::get_general_ledger，成功登记交 record_download
/// 存什么：不写业务表；凭证明细行写进导出文件，令牌消费写导出审批表
/// 存哪里：不适用
///
/// report_type 按本端点身份写成常量 general_ledger。subject_code 是本端点的必填取数键，
/// 因此必进范围参数；批准时未记该科目即按超范围拒绝。start_date 与 end_date 缺省时
/// 按当月起到今天取数，period 等其余声明键只参与范围比对。
pub async fn export_general_ledger(
    State(state): State<AppState>,
    Query(query): Query<GeneralLedgerQuery>,
    auth: AuthContext,
) -> Result<axum::response::Response, AppError> {
    // 敏感导出 fail-closed：受理门走加固入口（申请人本人 + 批准范围逐项比对）
    let approval_svc = ExportApprovalService::new(state.db.clone());
    let requested_scope = finance_scope_params(
        "general_ledger",
        &query.period,
        &Some(query.subject_code.clone()),
        query.start_date,
        query.end_date,
        &query.dimension_type,
        &query.dimension_value,
    );
    let approval = approval_svc
        .enforce_export_download_secured(
            query.download_token.as_deref(),
            "finance_report",
            auth.user_id,
            Some(requested_scope),
        )
        .await?;

    let table = approval_svc
        .run_export_leg(approval.id, async {
            const EXPORT_LIMIT: usize = 10000;
            let service = FinanceReportService::new(state.db.clone());
            let start_date = query.start_date.unwrap_or_else(|| {
                chrono::Utc::now()
                    .date_naive()
                    .with_day(1)
                    .unwrap_or_else(|| chrono::Utc::now().date_naive())
            });
            let end_date = query
                .end_date
                .unwrap_or_else(|| chrono::Utc::now().date_naive());
            let ledger = service
                .get_general_ledger(query.subject_code.clone(), start_date, end_date)
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

            // 敏感导出 fail-closed：记录令牌消费
            approval_svc
                .record_download(
                    approval.id,
                    "general_ledger_export".to_string(),
                    table.rows.len() as i64,
                    String::new(),
                )
                .await?;

            Ok(table)
        })
        .await?;

    build_xlsx_response(&table, "general_ledger_export")
}

/// 导出明细账为 xlsx，端点为 GET /api/v1/erp/finance/reports/subsidiary-ledger/export
///
/// 功能：按本次请求的维度与日期区间取明细账并落成 xlsx
/// 调用方：财务路由的明细账导出端点
/// 入参：查询参数（dimension_type 与 dimension_value 必填、start_date、end_date
/// 与申请侧声明键及审批令牌）、应用状态、会话上下文
/// 传给谁：受理门交 ExportApprovalService::enforce_export_download_secured，导出腿交
/// run_export_leg，取数交 FinanceReportService::get_subsidiary_ledger，成功登记交 record_download
/// 存什么：不写业务表；业务明细分录行写进导出文件，令牌消费写导出审批表
/// 存哪里：不适用
///
/// report_type 按本端点身份写成常量 subsidiary_ledger。dimension_type 与 dimension_value
/// 是本端点的必填取数键，因此必进范围参数；批准时未记这两个维度即按超范围拒绝，
/// 而前端报表页的申请参数只落 report_type、period 与 subject_code，不含这两维，
/// 该端点的批准范围核验需要申请侧补齐（已在回报单点名）。
pub async fn export_subsidiary_ledger(
    State(state): State<AppState>,
    Query(query): Query<SubsidiaryLedgerQuery>,
    auth: AuthContext,
) -> Result<axum::response::Response, AppError> {
    // 敏感导出 fail-closed：受理门走加固入口（申请人本人 + 批准范围逐项比对）
    let approval_svc = ExportApprovalService::new(state.db.clone());
    let requested_scope = finance_scope_params(
        "subsidiary_ledger",
        &query.period,
        &query.subject_code,
        query.start_date,
        query.end_date,
        &Some(query.dimension_type.clone()),
        &Some(query.dimension_value.clone()),
    );
    let approval = approval_svc
        .enforce_export_download_secured(
            query.download_token.as_deref(),
            "finance_report",
            auth.user_id,
            Some(requested_scope),
        )
        .await?;

    let table = approval_svc
        .run_export_leg(approval.id, async {
            const EXPORT_LIMIT: usize = 10000;
            let service = FinanceReportService::new(state.db.clone());
            let start_date = query.start_date.unwrap_or_else(|| {
                chrono::Utc::now()
                    .date_naive()
                    .with_day(1)
                    .unwrap_or_else(|| chrono::Utc::now().date_naive())
            });
            let end_date = query
                .end_date
                .unwrap_or_else(|| chrono::Utc::now().date_naive());
            let ledger = service
                .get_subsidiary_ledger(
                    query.dimension_type.clone(),
                    query.dimension_value.clone(),
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

            // 敏感导出 fail-closed：记录令牌消费
            approval_svc
                .record_download(
                    approval.id,
                    "subsidiary_ledger_export".to_string(),
                    table.rows.len() as i64,
                    String::new(),
                )
                .await?;

            Ok(table)
        })
        .await?;

    build_xlsx_response(&table, "subsidiary_ledger_export")
}
