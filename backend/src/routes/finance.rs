//! 财务域路由
//!
//! 处理财务、AP/AR 应付应收、凭证/总账、固定资产、预算、资金管理、财务分析、币种等财务相关接口。

use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::{
    account_subject_handler, accounting_period_handler, ap_invoice_handler, ap_payment_handler,
    ap_payment_request_handler, ap_reconciliation_handler, ap_report_handler,
    ap_verification_handler, ar_invoice_handler, ar_payment_handler,
    ar_reconciliation_enhanced_handler, ar_reconciliation_handler, ar_report_handler,
    ar_verification_handler, budget_management_handler, currency_enhanced_handler,
    currency_handler, finance_invoice_handler, finance_payment_handler, finance_report_handler,
    financial_analysis_handler, fixed_asset_handler, fund_management_handler, missing_handlers,
    omni_audit_handler, voucher_handler,
};
use crate::utils::app_state::AppState;

/// 财务主路由（nest 到 /api/v1/erp/finance）
pub fn finance(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/payments", get(finance_payment_handler::list_payments))
        .route("/payments", post(finance_payment_handler::create_payment))
        .route("/payments/:id", get(finance_payment_handler::get_payment))
        .route(
            "/invoices",
            get(finance_invoice_handler::list_finance_invoices),
        )
        .route(
            "/invoices",
            post(finance_invoice_handler::create_finance_invoice),
        )
        .route(
            "/invoices/:id",
            get(finance_invoice_handler::get_finance_invoice)
                .put(finance_invoice_handler::update_finance_invoice)
                .delete(finance_invoice_handler::delete_finance_invoice),
        )
        .route(
            "/invoices/:id/approve",
            post(finance_invoice_handler::approve_finance_invoice),
        )
        .route(
            "/invoices/:id/verify",
            post(finance_invoice_handler::verify_invoice),
        )
        .route(
            "/accounting-periods",
            get(missing_handlers::get_accounting_periods)
                .post(missing_handlers::create_accounting_period),
        )
        .route(
            "/accounting-periods/current",
            get(accounting_period_handler::get_current_period),
        )
        .route(
            "/accounting-periods/init",
            post(accounting_period_handler::init_period),
        )
        .route(
            "/accounting-periods/:id",
            get(missing_handlers::get_accounting_period_detail)
                .put(missing_handlers::update_accounting_period)
                .delete(missing_handlers::delete_accounting_period),
        )
        .route(
            "/accounting-periods/:id/close",
            post(accounting_period_handler::close_period),
        )
        .route(
            "/reports/balance-sheet",
            get(finance_report_handler::get_balance_sheet),
        )
        .route(
            "/reports/income-statement",
            get(finance_report_handler::get_income_statement),
        )
        .route(
            "/reports/cash-flow",
            get(finance_report_handler::get_cash_flow_statement),
        )
        .route(
            "/reports/trial-balance",
            get(finance_report_handler::get_trial_balance),
        )
        .route(
            "/reports/general-ledger/:code",
            get(finance_report_handler::get_general_ledger),
        )
        .route(
            "/reports/subsidiary-ledger",
            get(finance_report_handler::get_subsidiary_ledger),
        )
        .route("/audit/track", post(omni_audit_handler::track_event))
        .route("/audit/stats", get(omni_audit_handler::get_dashboard_stats))
        .route("/audit/search", get(omni_audit_handler::search_logs))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::omni_audit::omni_audit_middleware,
        ))
}

/// 总账路由（nest 到 /api/v1/erp/gl）
pub fn gl() -> Router<AppState> {
    Router::new()
        .route("/subjects", get(account_subject_handler::list_subjects))
        .route(
            "/subjects/tree",
            get(account_subject_handler::get_subject_tree),
        )
        .route("/subjects", post(account_subject_handler::create_subject))
        .route("/subjects/:id", get(account_subject_handler::get_subject))
        .route(
            "/subjects/:id",
            put(account_subject_handler::update_subject),
        )
        .route(
            "/subjects/:id",
            delete(account_subject_handler::delete_subject),
        )
        .route("/vouchers/types", get(voucher_handler::get_voucher_types))
        .route("/vouchers", get(voucher_handler::list_vouchers))
        .route(
            "/vouchers/:id",
            get(voucher_handler::get_voucher)
                .put(voucher_handler::update_voucher)
                .delete(voucher_handler::delete_voucher),
        )
        .route("/vouchers", post(voucher_handler::create_voucher))
        .route(
            "/vouchers/:id/submit",
            post(voucher_handler::submit_voucher),
        )
        .route(
            "/vouchers/:id/review",
            post(voucher_handler::review_voucher),
        )
        .route("/vouchers/:id/post", post(voucher_handler::post_voucher))
}

/// 固定资产路由（nest 到 /api/v1/erp/fixed-assets）
pub fn fixed_assets() -> Router<AppState> {
    Router::new()
        .route("/", get(fixed_asset_handler::list_assets))
        .route("/", post(fixed_asset_handler::create_asset))
        .route("/:id", get(fixed_asset_handler::get_asset))
        .route("/:id", put(fixed_asset_handler::update_asset))
        .route("/:id", delete(fixed_asset_handler::delete_asset))
        .route(
            "/:id/depreciate",
            post(fixed_asset_handler::depreciate_asset),
        )
        .route("/:id/dispose", post(fixed_asset_handler::dispose_asset))
        .route(
            "/batch-depreciate",
            post(fixed_asset_handler::batch_depreciate),
        )
}

/// 预算管理路由（nest 到 /api/v1/erp/budgets）
pub fn budgets() -> Router<AppState> {
    Router::new()
        .route("/", get(budget_management_handler::list_budgets))
        .route("/", post(budget_management_handler::create_budget))
        .route("/:id", get(budget_management_handler::get_budget))
        .route("/:id", put(budget_management_handler::update_budget))
        .route("/:id", delete(budget_management_handler::delete_budget))
        .route(
            "/:id/approve",
            post(budget_management_handler::approve_budget),
        )
        .route("/adjust", post(budget_management_handler::adjust_budget))
        .route("/items", get(budget_management_handler::list_budget_items))
        .route(
            "/items",
            post(budget_management_handler::create_budget_item),
        )
        .route(
            "/items/:id",
            get(budget_management_handler::get_budget_item),
        )
        .route(
            "/items/:id",
            put(budget_management_handler::update_budget_item),
        )
        .route(
            "/items/:id",
            delete(budget_management_handler::delete_budget_item),
        )
        .route("/plans", get(budget_management_handler::list_plans))
        .route("/plans", post(budget_management_handler::create_plan))
        .route("/plans/:id", get(budget_management_handler::get_plan))
        .route(
            "/plans/:id/approve",
            post(budget_management_handler::approve_plan),
        )
        .route(
            "/plans/:id/execute",
            post(budget_management_handler::execute_plan),
        )
        .route(
            "/plans/:id/executions",
            get(budget_management_handler::get_plan_executions),
        )
        .route(
            "/plans/:id/executions",
            post(budget_management_handler::create_execution),
        )
        .route(
            "/control/:plan_id",
            get(budget_management_handler::get_control),
        )
        .route(
            "/control/:plan_id/data",
            get(budget_management_handler::get_budget_control_data),
        )
}

/// 财务分析路由（nest 到 /api/v1/erp/financial-analysis）
pub fn financial_analysis() -> Router<AppState> {
    Router::new()
        .route("/reports", get(financial_analysis_handler::list_reports))
        .route("/reports", post(financial_analysis_handler::create_report))
        .route("/reports/:id", get(financial_analysis_handler::get_report))
        .route(
            "/reports/:id/execute",
            post(financial_analysis_handler::execute_report),
        )
        .route(
            "/indicators",
            get(financial_analysis_handler::get_indicators)
                .post(financial_analysis_handler::create_indicator),
        )
        .route(
            "/trends",
            get(financial_analysis_handler::get_trends)
                .post(financial_analysis_handler::create_trend),
        )
}

/// 资金管理路由（nest 到 /api/v1/erp/fund-management）
pub fn fund_management() -> Router<AppState> {
    Router::new()
        .route("/accounts", get(fund_management_handler::list_accounts))
        .route("/accounts", post(fund_management_handler::create_account))
        .route(
            "/accounts/:id",
            get(fund_management_handler::get_account)
                .put(fund_management_handler::update_account)
                .delete(fund_management_handler::delete_account),
        )
        .route(
            "/accounts/:id/deposit",
            post(fund_management_handler::deposit),
        )
        .route(
            "/accounts/:id/withdraw",
            post(fund_management_handler::withdraw),
        )
        .route(
            "/accounts/:id/freeze",
            post(fund_management_handler::freeze_funds),
        )
        .route(
            "/accounts/:id/unfreeze",
            post(fund_management_handler::unfreeze_funds),
        )
        .route("/transfer", post(fund_management_handler::transfer))
        .route(
            "/transfers",
            get(fund_management_handler::list_transfer_records),
        )
        .route(
            "/transfers/:id",
            get(fund_management_handler::get_transfer_record),
        )
}

/// AP 应付账款路由（nest 到 /api/v1/erp/ap）
pub fn ap() -> Router<AppState> {
    Router::new()
        .route("/invoices", get(ap_invoice_handler::list_ap_invoices))
        .route("/invoices", post(ap_invoice_handler::create_ap_invoice))
        .route("/invoices/:id", get(ap_invoice_handler::get_ap_invoice))
        .route("/invoices/:id", put(ap_invoice_handler::update_ap_invoice))
        .route(
            "/invoices/:id",
            delete(ap_invoice_handler::delete_ap_invoice),
        )
        .route(
            "/invoices/:id/approve",
            post(ap_invoice_handler::approve_ap_invoice),
        )
        .route(
            "/invoices/:id/cancel",
            post(ap_invoice_handler::cancel_ap_invoice),
        )
        .route(
            "/invoices/auto-generate",
            post(ap_invoice_handler::auto_generate),
        )
        .route(
            "/invoices/aging",
            get(ap_invoice_handler::get_aging_analysis),
        )
        .route(
            "/invoices/balance",
            get(ap_invoice_handler::get_balance_summary),
        )
        .route(
            "/invoices/statistics",
            get(ap_invoice_handler::get_statistics),
        )
        .route("/payments", get(ap_payment_handler::list_payments))
        .route("/payments", post(ap_payment_handler::create_payment))
        .route("/payments/:id", get(ap_payment_handler::get_payment))
        .route("/payments/:id", put(ap_payment_handler::update_payment))
        .route(
            "/payments/:id/confirm",
            post(ap_payment_handler::confirm_payment),
        )
        .route(
            "/payment-requests",
            get(ap_payment_request_handler::list_requests),
        )
        .route(
            "/payment-requests",
            post(ap_payment_request_handler::create_request),
        )
        .route(
            "/payment-requests/:id",
            get(ap_payment_request_handler::get_request),
        )
        .route(
            "/payment-requests/:id",
            put(ap_payment_request_handler::update_request),
        )
        .route(
            "/payment-requests/:id",
            delete(ap_payment_request_handler::delete_request),
        )
        .route(
            "/payment-requests/:id/submit",
            post(ap_payment_request_handler::submit_request),
        )
        .route(
            "/payment-requests/:id/approve",
            post(ap_payment_request_handler::approve_request),
        )
        .route(
            "/payment-requests/:id/reject",
            post(ap_payment_request_handler::reject_request),
        )
        .route(
            "/verifications",
            get(ap_verification_handler::list_verifications),
        )
        .route(
            "/verifications/:id",
            get(ap_verification_handler::get_verification),
        )
        .route(
            "/verifications/auto",
            post(ap_verification_handler::auto_verify),
        )
        .route(
            "/verifications/manual",
            post(ap_verification_handler::manual_verify),
        )
        .route(
            "/verifications/:id/cancel",
            post(ap_verification_handler::cancel_verification),
        )
        .route(
            "/verifications/unverified/invoices",
            get(ap_verification_handler::get_unverified_invoices),
        )
        .route(
            "/verifications/unverified/payments",
            get(ap_verification_handler::get_unverified_payments),
        )
        .route(
            "/reconciliations",
            get(ap_reconciliation_handler::list_reconciliations),
        )
        .route(
            "/reconciliations/:id",
            get(ap_reconciliation_handler::get_reconciliation),
        )
        .route(
            "/reconciliations/generate",
            post(ap_reconciliation_handler::generate_reconciliation),
        )
        .route(
            "/reconciliations/:id/confirm",
            post(ap_reconciliation_handler::confirm_reconciliation),
        )
        .route(
            "/reconciliations/:id/dispute",
            post(ap_reconciliation_handler::dispute_reconciliation),
        )
        .route(
            "/reconciliations/auto",
            post(ap_reconciliation_handler::auto_reconcile_all),
        )
        .route(
            "/reconciliations/summary",
            get(ap_reconciliation_handler::get_supplier_summary),
        )
        .route(
            "/invoices/:id/relations",
            get(ap_reconciliation_handler::get_invoice_relations),
        )
        .route(
            "/reports/statistics",
            get(ap_report_handler::get_statistics_report),
        )
        .route("/reports/daily", get(ap_report_handler::get_daily_report))
        .route(
            "/reports/monthly",
            get(ap_report_handler::get_monthly_report),
        )
        .route("/reports/aging", get(ap_report_handler::get_aging_report))
}

/// AR 应收账款路由（nest 到 /api/v1/erp/ar）
pub fn ar() -> Router<AppState> {
    Router::new()
        .route("/invoices", get(ar_invoice_handler::list_ar_invoices))
        .route("/invoices", post(ar_invoice_handler::create_ar_invoice))
        .route("/invoices/:id", get(ar_invoice_handler::get_ar_invoice))
        .route("/invoices/:id", put(ar_invoice_handler::update_ar_invoice))
        .route(
            "/invoices/:id",
            delete(ar_invoice_handler::delete_ar_invoice),
        )
        .route(
            "/invoices/:id/approve",
            post(ar_invoice_handler::approve_ar_invoice),
        )
        .route(
            "/invoices/:id/cancel",
            post(ar_invoice_handler::cancel_ar_invoice),
        )
        .route("/payments", get(ar_payment_handler::list_payments))
        .route("/payments", post(ar_payment_handler::create_payment))
        .route("/payments/:id", get(ar_payment_handler::get_payment))
        .route("/payments/:id", put(ar_payment_handler::update_payment))
        .route(
            "/payments/:id/confirm",
            post(ar_payment_handler::confirm_payment),
        )
        .route(
            "/verifications",
            get(ar_verification_handler::list_verifications),
        )
        .route(
            "/verifications/:id",
            get(ar_verification_handler::get_verification),
        )
        .route(
            "/verifications/auto",
            post(ar_verification_handler::auto_verify),
        )
        .route(
            "/verifications/manual",
            post(ar_verification_handler::manual_verify),
        )
        .route(
            "/verifications/:id/cancel",
            post(ar_verification_handler::cancel_verification),
        )
        .route(
            "/verifications/unverified/invoices",
            get(ar_verification_handler::get_unverified_invoices),
        )
        .route(
            "/verifications/unverified/payments",
            get(ar_verification_handler::get_unverified_payments),
        )
        .route(
            "/reports/statistics",
            get(ar_report_handler::get_statistics_report),
        )
        .route("/reports/daily", get(ar_report_handler::get_daily_report))
        .route(
            "/reports/monthly",
            get(ar_report_handler::get_monthly_report),
        )
        .route("/reports/aging", get(ar_report_handler::get_aging_report))
}

/// 应收对账增强路由（nest 到 /api/v1/erp/ar-reconciliations/enhanced）
pub fn ar_reconciliations_enhanced() -> Router<AppState> {
    Router::new()
        .route(
            "/auto-match",
            post(ar_reconciliation_enhanced_handler::auto_match),
        )
        .route(
            "/aging-report",
            get(ar_reconciliation_enhanced_handler::aging_report),
        )
        .route(
            "/:id/details",
            get(ar_reconciliation_enhanced_handler::get_reconciliation_details),
        )
        .route(
            "/:id/confirm",
            post(ar_reconciliation_enhanced_handler::confirm_reconciliation),
        )
        .route(
            "/:id/dispute",
            post(ar_reconciliation_enhanced_handler::dispute_reconciliation),
        )
        .route(
            "/:id/pdf",
            get(ar_reconciliation_enhanced_handler::export_reconciliation_pdf),
        )
        .route(
            "/generate",
            post(ar_reconciliation_enhanced_handler::generate_reconciliation),
        )
}

/// 应收对账别名路由（nest 到 /api/v1/erp/ar-reconciliation）
pub fn ar_reconciliation_alias() -> Router<AppState> {
    Router::new()
        .route(
            "/auto-reconcile",
            post(ar_reconciliation_enhanced_handler::auto_match),
        )
        .route(
            "/auto-reconcile/results",
            get(ar_reconciliation_enhanced_handler::list_results),
        )
        .route(
            "/aging-analysis",
            get(ar_reconciliation_enhanced_handler::aging_report),
        )
        .route(
            "/:id/details",
            get(ar_reconciliation_enhanced_handler::get_reconciliation_details),
        )
        .route(
            "/:id/confirm/send",
            post(ar_reconciliation_enhanced_handler::send_confirmation),
        )
        .route(
            "/confirmations",
            get(ar_reconciliation_enhanced_handler::list_confirmations)
                .post(ar_reconciliation_enhanced_handler::create_confirmation),
        )
        .route(
            "/confirmations/:id/status",
            put(ar_reconciliation_enhanced_handler::update_confirmation_status),
        )
        .route(
            "/disputes",
            get(ar_reconciliation_enhanced_handler::list_disputes)
                .post(ar_reconciliation_enhanced_handler::create_dispute),
        )
        .route(
            "/disputes/:id",
            get(ar_reconciliation_enhanced_handler::get_dispute),
        )
        .route(
            "/disputes/:id/resolve",
            put(ar_reconciliation_enhanced_handler::resolve_dispute),
        )
}

/// 应收对账路由（nest 到 /api/v1/erp/ar-reconciliations）
pub fn ar_reconciliations() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(ar_reconciliation_handler::list_reconciliations)
                .post(ar_reconciliation_handler::create_reconciliation),
        )
        .route("/:id", get(ar_reconciliation_handler::get_reconciliation))
        .route(
            "/:id/status",
            put(ar_reconciliation_handler::update_reconciliation_status),
        )
}

/// 多币种路由（nest 到 /api/v1/erp/currencies）
pub fn currencies() -> Router<AppState> {
    Router::new()
        .route("/", get(currency_handler::list_currencies))
        .route("/base", get(currency_handler::get_base_currency))
        .route(
            "/rates/history",
            get(currency_enhanced_handler::get_exchange_rate_history),
        )
        .route("/convert", post(currency_enhanced_handler::convert_amount))
        .route("/sync-all", post(currency_enhanced_handler::sync_all_rates))
        .route(
            "/supported",
            get(currency_enhanced_handler::get_supported_currencies),
        )
}

/// 汇率路由（nest 到 /api/v1/erp/exchange-rates）
pub fn exchange_rates() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(currency_handler::list_exchange_rates).post(currency_handler::create_exchange_rate),
        )
        .route("/query", get(currency_handler::get_exchange_rate))
}

/// 财务域统一入口
pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(finance(state.clone()))
        .merge(gl())
        .merge(fixed_assets())
        .merge(budgets())
        .merge(financial_analysis())
        .merge(fund_management())
        .merge(ap())
        .merge(ar())
        .merge(ar_reconciliations_enhanced())
        .merge(ar_reconciliation_alias())
        .merge(ar_reconciliations())
        .merge(currencies())
        .merge(exchange_rates())
        // 显式使用 middleware 抑制未使用警告
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::rate_limit::rate_limit_by_ip,
        ))
}
