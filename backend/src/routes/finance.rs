//! 财务域路由
//!
//! 处理财务、AP/AR 应付应收、凭证/总账、固定资产、预算、资金管理、财务分析、币种等财务相关接口。
//!
//! 路由设计说明：所有子 router 内部 path 都已加上各自独立前缀
//!
//! P2 2-11 文档标注：本模块中 `POST /resource/:id/{action}` 形式的端点为"动作端点"，
//! 语义上等价于状态变更（approve/cancel/submit/verify/reject/close 等），RESTful 规范应为 `PATCH /resource/:id` + body `{status}`。
//! 短期保留 POST 动作端点以兼容前端；长期计划重构为 PATCH 统一状态变更语义。
//!（`/fixed-assets`、`/budgets`、`/financial-analysis`、
//!  `/fund-management`、`/ar-reconciliations`、`/ar-reconciliations-enhanced`、
//!  `/ar-reconciliation-alias`、`/currencies`、`/exchange-rates` 等），
//! 这样 `routes()` 入口用 `merge` 组合时不会出现 path+method 重叠，
//! 避免 axum 0.7 `Overlapping method route` panic。
//!
//! 注意 `finance()`/`gl()`/`ap()`/`ar()` 子 router 自身已使用
//! 业务级路径（`/payments`、`/invoices`、`/vouchers` 等），path 不冲突。

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

/// 财务主路由（path 前缀以 /payments、/invoices、/accounting-periods、/reports、/audit 区分）
pub fn finance() -> Router<AppState> {
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
    // P0 8-1 修复：omni_audit_middleware 已全局挂载（见 main.rs 中间件链），
    // 此处移除局部挂载避免重复审计。
}

/// 总账路由（path 前缀以 /subjects、/vouchers 区分）
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
        .route(
            "/vouchers/generate-no",
            get(voucher_handler::generate_voucher_no),
        )
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

/// 固定资产路由（path 前缀 /fixed-assets）
pub fn fixed_assets() -> Router<AppState> {
    Router::new()
        .route("/fixed-assets", get(fixed_asset_handler::list_assets))
        .route("/fixed-assets", post(fixed_asset_handler::create_asset))
        .route("/fixed-assets/:id", get(fixed_asset_handler::get_asset))
        .route("/fixed-assets/:id", put(fixed_asset_handler::update_asset))
        .route(
            "/fixed-assets/:id",
            delete(fixed_asset_handler::delete_asset),
        )
        .route(
            "/fixed-assets/:id/depreciate",
            post(fixed_asset_handler::depreciate_asset),
        )
        .route(
            "/fixed-assets/:id/dispose",
            post(fixed_asset_handler::dispose_asset),
        )
        .route(
            "/fixed-assets/:id/depreciation-records",
            get(fixed_asset_handler::list_depreciation_records),
        )
        .route(
            "/fixed-assets/batch-depreciate",
            post(fixed_asset_handler::batch_depreciate),
        )
        .route(
            "/fixed-assets/disposals",
            get(fixed_asset_handler::list_disposals),
        )
}

/// 预算管理路由（path 前缀 /budgets）
pub fn budgets() -> Router<AppState> {
    Router::new()
        .route("/budgets", get(budget_management_handler::list_budgets))
        .route("/budgets", post(budget_management_handler::create_budget))
        .route("/budgets/:id", get(budget_management_handler::get_budget))
        .route(
            "/budgets/:id",
            put(budget_management_handler::update_budget),
        )
        .route(
            "/budgets/:id",
            delete(budget_management_handler::delete_budget),
        )
        .route(
            "/budgets/:id/approve",
            post(budget_management_handler::approve_budget),
        )
        .route(
            "/budgets/adjust",
            post(budget_management_handler::adjust_budget),
        )
        .route(
            "/budgets/items",
            get(budget_management_handler::list_budget_items),
        )
        .route(
            "/budgets/items",
            post(budget_management_handler::create_budget_item),
        )
        .route(
            "/budgets/items/:id",
            get(budget_management_handler::get_budget_item),
        )
        .route(
            "/budgets/items/:id",
            put(budget_management_handler::update_budget_item),
        )
        .route(
            "/budgets/items/:id",
            delete(budget_management_handler::delete_budget_item),
        )
        .route("/budgets/plans", get(budget_management_handler::list_plans))
        .route(
            "/budgets/plans",
            post(budget_management_handler::create_plan),
        )
        .route(
            "/budgets/plans/:id",
            get(budget_management_handler::get_plan),
        )
        .route(
            "/budgets/plans/:id/approve",
            post(budget_management_handler::approve_plan),
        )
        .route(
            "/budgets/plans/:id/execute",
            post(budget_management_handler::execute_plan),
        )
        .route(
            "/budgets/plans/:id/executions",
            get(budget_management_handler::get_plan_executions),
        )
        .route(
            "/budgets/plans/:id/executions",
            post(budget_management_handler::create_execution),
        )
        .route(
            "/budgets/control/:plan_id",
            get(budget_management_handler::get_control),
        )
        .route(
            "/budgets/control/:plan_id/data",
            get(budget_management_handler::get_budget_control_data),
        )
}

/// 财务分析路由（path 前缀 /financial-analysis）
pub fn financial_analysis() -> Router<AppState> {
    Router::new()
        .route(
            "/financial-analysis/reports",
            get(financial_analysis_handler::list_reports),
        )
        .route(
            "/financial-analysis/reports",
            post(financial_analysis_handler::create_report),
        )
        .route(
            "/financial-analysis/reports/:id",
            get(financial_analysis_handler::get_report),
        )
        .route(
            "/financial-analysis/reports/:id/execute",
            post(financial_analysis_handler::execute_report),
        )
        .route(
            "/financial-analysis/indicators",
            get(financial_analysis_handler::get_indicators)
                .post(financial_analysis_handler::create_indicator),
        )
        .route(
            "/financial-analysis/trends",
            get(financial_analysis_handler::get_trends)
                .post(financial_analysis_handler::create_trend),
        )
}

/// 资金管理路由（path 前缀 /fund-management）
pub fn fund_management() -> Router<AppState> {
    Router::new()
        .route(
            "/fund-management/accounts",
            get(fund_management_handler::list_accounts),
        )
        .route(
            "/fund-management/accounts",
            post(fund_management_handler::create_account),
        )
        .route(
            "/fund-management/accounts/:id",
            get(fund_management_handler::get_account)
                .put(fund_management_handler::update_account)
                .delete(fund_management_handler::delete_account),
        )
        .route(
            "/fund-management/accounts/:id/deposit",
            post(fund_management_handler::deposit),
        )
        .route(
            "/fund-management/accounts/:id/withdraw",
            post(fund_management_handler::withdraw),
        )
        .route(
            "/fund-management/accounts/:id/freeze",
            post(fund_management_handler::freeze_funds),
        )
        .route(
            "/fund-management/accounts/:id/unfreeze",
            post(fund_management_handler::unfreeze_funds),
        )
        .route(
            "/fund-management/transfer",
            post(fund_management_handler::transfer),
        )
        .route(
            "/fund-management/transfers",
            get(fund_management_handler::list_transfer_records),
        )
        .route(
            "/fund-management/transfers/:id",
            get(fund_management_handler::get_transfer_record),
        )
}

/// AP 应付账款路由（path 前缀 /ap）
pub fn ap() -> Router<AppState> {
    Router::new()
        .route("/ap/invoices", get(ap_invoice_handler::list_ap_invoices))
        .route("/ap/invoices", post(ap_invoice_handler::create_ap_invoice))
        .route("/ap/invoices/:id", get(ap_invoice_handler::get_ap_invoice))
        .route(
            "/ap/invoices/:id",
            put(ap_invoice_handler::update_ap_invoice),
        )
        .route(
            "/ap/invoices/:id",
            delete(ap_invoice_handler::delete_ap_invoice),
        )
        .route(
            "/ap/invoices/:id/approve",
            post(ap_invoice_handler::approve_ap_invoice),
        )
        .route(
            "/ap/invoices/:id/cancel",
            post(ap_invoice_handler::cancel_ap_invoice),
        )
        .route(
            "/ap/invoices/auto-generate",
            post(ap_invoice_handler::auto_generate),
        )
        .route(
            "/ap/invoices/aging",
            get(ap_invoice_handler::get_aging_analysis),
        )
        .route(
            "/ap/invoices/balance",
            get(ap_invoice_handler::get_balance_summary),
        )
        .route(
            "/ap/invoices/statistics",
            get(ap_invoice_handler::get_statistics),
        )
        .route("/ap/payments", get(ap_payment_handler::list_payments))
        .route("/ap/payments", post(ap_payment_handler::create_payment))
        .route("/ap/payments/:id", get(ap_payment_handler::get_payment))
        .route("/ap/payments/:id", put(ap_payment_handler::update_payment))
        .route(
            "/ap/payments/:id/confirm",
            post(ap_payment_handler::confirm_payment),
        )
        .route(
            "/ap/payment-requests",
            get(ap_payment_request_handler::list_requests),
        )
        .route(
            "/ap/payment-requests",
            post(ap_payment_request_handler::create_request),
        )
        .route(
            "/ap/payment-requests/:id",
            get(ap_payment_request_handler::get_request),
        )
        .route(
            "/ap/payment-requests/:id",
            put(ap_payment_request_handler::update_request),
        )
        .route(
            "/ap/payment-requests/:id",
            delete(ap_payment_request_handler::delete_request),
        )
        .route(
            "/ap/payment-requests/:id/submit",
            post(ap_payment_request_handler::submit_request),
        )
        .route(
            "/ap/payment-requests/:id/approve",
            post(ap_payment_request_handler::approve_request),
        )
        .route(
            "/ap/payment-requests/:id/reject",
            post(ap_payment_request_handler::reject_request),
        )
        .route(
            "/ap/verifications",
            get(ap_verification_handler::list_verifications),
        )
        .route(
            "/ap/verifications/:id",
            get(ap_verification_handler::get_verification),
        )
        .route(
            "/ap/verifications/auto",
            post(ap_verification_handler::auto_verify),
        )
        .route(
            "/ap/verifications/manual",
            post(ap_verification_handler::manual_verify),
        )
        .route(
            "/ap/verifications/:id/cancel",
            post(ap_verification_handler::cancel_verification),
        )
        .route(
            "/ap/verifications/unverified/invoices",
            get(ap_verification_handler::get_unverified_invoices),
        )
        .route(
            "/ap/verifications/unverified/payments",
            get(ap_verification_handler::get_unverified_payments),
        )
        .route(
            "/ap/reconciliations",
            get(ap_reconciliation_handler::list_reconciliations),
        )
        .route(
            "/ap/reconciliations/:id",
            get(ap_reconciliation_handler::get_reconciliation),
        )
        .route(
            "/ap/reconciliations/generate",
            post(ap_reconciliation_handler::generate_reconciliation),
        )
        .route(
            "/ap/reconciliations/:id/confirm",
            post(ap_reconciliation_handler::confirm_reconciliation),
        )
        .route(
            "/ap/reconciliations/:id/dispute",
            post(ap_reconciliation_handler::dispute_reconciliation),
        )
        .route(
            "/ap/reconciliations/auto",
            post(ap_reconciliation_handler::auto_reconcile_all),
        )
        .route(
            "/ap/reconciliations/summary",
            get(ap_reconciliation_handler::get_supplier_summary),
        )
        .route(
            "/ap/invoices/:id/relations",
            get(ap_reconciliation_handler::get_invoice_relations),
        )
        .route(
            "/ap/reports/statistics",
            get(ap_report_handler::get_statistics_report),
        )
        .route(
            "/ap/reports/daily",
            get(ap_report_handler::get_daily_report),
        )
        .route(
            "/ap/reports/monthly",
            get(ap_report_handler::get_monthly_report),
        )
        .route(
            "/ap/reports/aging",
            get(ap_report_handler::get_aging_report),
        )
}

/// AR 应收账款路由（path 前缀 /ar）
pub fn ar() -> Router<AppState> {
    Router::new()
        .route("/ar/invoices", get(ar_invoice_handler::list_ar_invoices))
        .route("/ar/invoices", post(ar_invoice_handler::create_ar_invoice))
        .route("/ar/invoices/:id", get(ar_invoice_handler::get_ar_invoice))
        .route(
            "/ar/invoices/:id",
            put(ar_invoice_handler::update_ar_invoice),
        )
        .route(
            "/ar/invoices/:id",
            delete(ar_invoice_handler::delete_ar_invoice),
        )
        .route(
            "/ar/invoices/:id/approve",
            post(ar_invoice_handler::approve_ar_invoice),
        )
        .route(
            "/ar/invoices/:id/cancel",
            post(ar_invoice_handler::cancel_ar_invoice),
        )
        .route("/ar/payments", get(ar_payment_handler::list_payments))
        .route("/ar/payments", post(ar_payment_handler::create_payment))
        .route("/ar/payments/:id", get(ar_payment_handler::get_payment))
        .route("/ar/payments/:id", put(ar_payment_handler::update_payment))
        .route(
            "/ar/payments/:id/confirm",
            post(ar_payment_handler::confirm_payment),
        )
        .route(
            "/ar/verifications",
            get(ar_verification_handler::list_verifications),
        )
        .route(
            "/ar/verifications/:id",
            get(ar_verification_handler::get_verification),
        )
        .route(
            "/ar/verifications/auto",
            post(ar_verification_handler::auto_verify),
        )
        .route(
            "/ar/verifications/manual",
            post(ar_verification_handler::manual_verify),
        )
        .route(
            "/ar/verifications/:id/cancel",
            post(ar_verification_handler::cancel_verification),
        )
        .route(
            "/ar/verifications/unverified/invoices",
            get(ar_verification_handler::get_unverified_invoices),
        )
        .route(
            "/ar/verifications/unverified/payments",
            get(ar_verification_handler::get_unverified_payments),
        )
        .route(
            "/ar/reports/statistics",
            get(ar_report_handler::get_statistics_report),
        )
        .route(
            "/ar/reports/daily",
            get(ar_report_handler::get_daily_report),
        )
        .route(
            "/ar/reports/monthly",
            get(ar_report_handler::get_monthly_report),
        )
        .route(
            "/ar/reports/aging",
            get(ar_report_handler::get_aging_report),
        )
}

/// 应收对账增强路由（path 前缀 /ar-reconciliations-enhanced）
pub fn ar_reconciliations_enhanced() -> Router<AppState> {
    Router::new()
        .route(
            "/ar-reconciliations-enhanced/auto-match",
            post(ar_reconciliation_enhanced_handler::auto_match),
        )
        .route(
            "/ar-reconciliations-enhanced/aging-report",
            get(ar_reconciliation_enhanced_handler::aging_report),
        )
        .route(
            "/ar-reconciliations-enhanced/:id/details",
            get(ar_reconciliation_enhanced_handler::get_reconciliation_details),
        )
        .route(
            "/ar-reconciliations-enhanced/:id/confirm",
            post(ar_reconciliation_enhanced_handler::confirm_reconciliation),
        )
        .route(
            "/ar-reconciliations-enhanced/:id/dispute",
            post(ar_reconciliation_enhanced_handler::dispute_reconciliation),
        )
        .route(
            "/ar-reconciliations-enhanced/:id/pdf",
            get(ar_reconciliation_enhanced_handler::export_reconciliation_pdf),
        )
        .route(
            "/ar-reconciliations-enhanced/generate",
            post(ar_reconciliation_enhanced_handler::generate_reconciliation),
        )
        .route(
            "/ar-reconciliations-enhanced/confirmations",
            get(ar_reconciliation_enhanced_handler::list_confirmations),
        )
        .route(
            "/ar-reconciliations-enhanced/confirmations/:id/status",
            put(ar_reconciliation_enhanced_handler::update_confirmation_status),
        )
        .route(
            "/ar-reconciliations-enhanced/disputes",
            get(ar_reconciliation_enhanced_handler::list_disputes)
                .post(ar_reconciliation_enhanced_handler::create_dispute),
        )
        .route(
            "/ar-reconciliations-enhanced/disputes/:id",
            get(ar_reconciliation_enhanced_handler::get_dispute),
        )
        .route(
            "/ar-reconciliations-enhanced/disputes/:id/resolve",
            put(ar_reconciliation_enhanced_handler::resolve_dispute),
        )
}

/// 应收对账别名路由（path 前缀 /ar-reconciliation-alias）
pub fn ar_reconciliation_alias() -> Router<AppState> {
    Router::new()
        .route(
            "/ar-reconciliation-alias/auto-reconcile",
            post(ar_reconciliation_enhanced_handler::auto_match),
        )
        .route(
            "/ar-reconciliation-alias/auto-reconcile/results",
            get(ar_reconciliation_enhanced_handler::list_results),
        )
        .route(
            "/ar-reconciliation-alias/aging-analysis",
            get(ar_reconciliation_enhanced_handler::aging_report),
        )
        .route(
            "/ar-reconciliation-alias/:id/details",
            get(ar_reconciliation_enhanced_handler::get_reconciliation_details),
        )
        .route(
            "/ar-reconciliation-alias/:id/confirm/send",
            post(ar_reconciliation_enhanced_handler::send_confirmation),
        )
        .route(
            "/ar-reconciliation-alias/confirmations",
            get(ar_reconciliation_enhanced_handler::list_confirmations)
                .post(ar_reconciliation_enhanced_handler::create_confirmation),
        )
        .route(
            "/ar-reconciliation-alias/confirmations/:id/status",
            put(ar_reconciliation_enhanced_handler::update_confirmation_status),
        )
        .route(
            "/ar-reconciliation-alias/disputes",
            get(ar_reconciliation_enhanced_handler::list_disputes)
                .post(ar_reconciliation_enhanced_handler::create_dispute),
        )
        .route(
            "/ar-reconciliation-alias/disputes/:id",
            get(ar_reconciliation_enhanced_handler::get_dispute),
        )
        .route(
            "/ar-reconciliation-alias/disputes/:id/resolve",
            put(ar_reconciliation_enhanced_handler::resolve_dispute),
        )
}

/// 应收对账路由（path 前缀 /ar-reconciliations）
///
/// 批次 108 P1-6 修复：补齐 update/delete/send/confirm/dispute/close 6 端点，
/// 接入 service::ar::recon 中已实现但未挂载路由的方法。
/// - PUT    /:id           → update_reconciliation（仅草稿状态可更新）
/// - DELETE /:id           → delete_reconciliation（仅草稿状态可删除）
/// - POST   /:id/send      → send_reconciliation（draft → sent）
/// - POST   /:id/confirm   → confirm_reconciliation（sent → confirmed，复用 enhanced 版本）
/// - POST   /:id/dispute   → dispute_reconciliation（sent → disputed，复用 enhanced 版本）
/// - POST   /:id/close     → close_reconciliation（confirmed/disputed → closed）
pub fn ar_reconciliations() -> Router<AppState> {
    Router::new()
        .route(
            "/ar-reconciliations",
            get(ar_reconciliation_handler::list_reconciliations)
                .post(ar_reconciliation_handler::create_reconciliation),
        )
        .route(
            "/ar-reconciliations/:id",
            get(ar_reconciliation_handler::get_reconciliation)
                .put(ar_reconciliation_handler::update_reconciliation)
                .delete(ar_reconciliation_handler::delete_reconciliation),
        )
        .route(
            "/ar-reconciliations/:id/status",
            put(ar_reconciliation_handler::update_reconciliation_status),
        )
        .route(
            "/ar-reconciliations/:id/send",
            post(ar_reconciliation_handler::send_reconciliation),
        )
        .route(
            "/ar-reconciliations/:id/confirm",
            post(ar_reconciliation_handler::confirm_reconciliation),
        )
        .route(
            "/ar-reconciliations/:id/dispute",
            post(ar_reconciliation_handler::dispute_reconciliation),
        )
        .route(
            "/ar-reconciliations/:id/close",
            post(ar_reconciliation_handler::close_reconciliation),
        )
}

/// 多币种路由（path 前缀 /currencies）
pub fn currencies() -> Router<AppState> {
    Router::new()
        .route("/currencies", get(currency_handler::list_currencies))
        .route("/currencies/base", get(currency_handler::get_base_currency))
        .route(
            "/currencies/:id/set-base",
            post(currency_handler::set_base_currency),
        )
        .route(
            "/currencies/rates/history",
            get(currency_enhanced_handler::get_exchange_rate_history),
        )
        .route(
            "/currencies/convert",
            post(currency_enhanced_handler::convert_amount),
        )
        .route(
            "/currencies/sync-all",
            post(currency_enhanced_handler::sync_all_rates),
        )
        .route(
            "/currencies/supported",
            get(currency_enhanced_handler::get_supported_currencies),
        )
}

/// 汇率路由（path 前缀 /exchange-rates）
pub fn exchange_rates() -> Router<AppState> {
    Router::new()
        .route(
            "/exchange-rates",
            get(currency_handler::list_exchange_rates).post(currency_handler::create_exchange_rate),
        )
        .route(
            "/exchange-rates/query",
            get(currency_handler::get_exchange_rate),
        )
}

/// 财务域统一入口（仅保留 finance 自身路径：/accounting-periods, /reports/... 等）
///
/// 通过 mod.rs `.nest("/api/v1/erp/finance", ...)` 挂载，
/// 最终 path = `/api/v1/erp/finance/accounting-periods` 等。
pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(finance())
        // 显式使用 middleware 抑制未使用警告
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::rate_limit::rate_limit_by_ip,
        ))
}

/// 财务子模块路由（ap / ar / gl / fixed_assets / budgets 等）
///
/// 通过 mod.rs `.nest("/api/v1/erp", finance::sub_routes())` 直接挂载在 `/api/v1/erp` 层级，
/// 最终 path = `/api/v1/erp/ap/invoices`、`/api/v1/erp/gl/subjects` 等。
pub fn sub_routes() -> Router<AppState> {
    Router::new()
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
}
