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

use crate::container::AppState;
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

/// 财务支付与发票路由（/payments、/invoices）
fn finance_payment_invoice_routes() -> Router<AppState> {
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
}

/// 会计期间路由（/accounting-periods）
fn accounting_period_routes() -> Router<AppState> {
    Router::new()
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
}

/// 财务报表路由（/reports）
fn finance_report_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/reports/balance-sheet",
            get(finance_report_handler::get_balance_sheet),
        )
        // V15 P0 5-1 修复：资产负债表导出
        .route(
            "/reports/balance-sheet/export",
            get(finance_report_handler::export_balance_sheet),
        )
        .route(
            "/reports/income-statement",
            get(finance_report_handler::get_income_statement),
        )
        // V15 P0 5-1 修复：利润表导出
        .route(
            "/reports/income-statement/export",
            get(finance_report_handler::export_income_statement),
        )
        .route(
            "/reports/cash-flow",
            get(finance_report_handler::get_cash_flow_statement),
        )
        // V15 P0 5-1 修复：现金流量表导出
        .route(
            "/reports/cash-flow/export",
            get(finance_report_handler::export_cash_flow_statement),
        )
        .route(
            "/reports/trial-balance",
            get(finance_report_handler::get_trial_balance),
        )
        // V15 P0 5-1 修复：试算平衡表导出
        .route(
            "/reports/trial-balance/export",
            get(finance_report_handler::export_trial_balance),
        )
        .route(
            "/reports/general-ledger/:code",
            get(finance_report_handler::get_general_ledger),
        )
        // V15 P0 5-1 修复：总账导出
        .route(
            "/reports/general-ledger/export",
            get(finance_report_handler::export_general_ledger),
        )
        .route(
            "/reports/subsidiary-ledger",
            get(finance_report_handler::get_subsidiary_ledger),
        )
        // V15 P0 5-1 修复：明细账导出
        .route(
            "/reports/subsidiary-ledger/export",
            get(finance_report_handler::export_subsidiary_ledger),
        )
        .route(
            "/reports/drill-down",
            get(finance_report_handler::drill_down_report),
        )
}

/// 财务审计路由（/audit）
fn finance_audit_routes() -> Router<AppState> {
    Router::new()
        .route("/audit/track", post(omni_audit_handler::track_event))
        .route("/audit/stats", get(omni_audit_handler::get_dashboard_stats))
        .route("/audit/search", get(omni_audit_handler::search_logs))
}

/// 财务主路由（合并支付/发票/期间/报表/审计）
pub fn finance() -> Router<AppState> {
    // P0 8-1 修复：omni_audit_middleware 已全局挂载（见 main.rs 中间件链），
    // 此处移除局部挂载避免重复审计。
    Router::new()
        .merge(finance_payment_invoice_routes())
        .merge(accounting_period_routes())
        .merge(finance_report_routes())
        .merge(finance_audit_routes())
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
        // 批次 400 修复（规则 0/8/14）：接入科目余额刷新 API
        .route(
            "/subjects/:id/refresh-balance",
            post(account_subject_handler::refresh_subject_balance),
        )
        // V15 P0 5-1 修复：会计科目导出
        .route(
            "/subjects/export",
            get(account_subject_handler::export_subjects),
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
        // V15 修复（A0）：会计凭证打印，返回 docx 成品（规则 3 合规）
        .route(
            "/vouchers/:id/print",
            get(crate::handlers::print_handler::voucher_print_docx),
        )
        // V15 P0 5-1 修复：凭证导出
        .route("/vouchers/export", get(voucher_handler::export_vouchers))
}

/// 固定资产路由（path 前缀 /fixed-assets）
pub fn fixed_assets() -> Router<AppState> {
    Router::new()
        .route("/fixed-assets", get(fixed_asset_handler::list_assets))
        .route("/fixed-assets", post(fixed_asset_handler::create_asset))
        // V15 P0-S12 修复（Batch 475e）：固定资产导出端点（必须在 /:id 之前注册，避免 axum matchit 把 "export" 当 :id 匹配）
        .route(
            "/fixed-assets/export",
            get(fixed_asset_handler::export_assets),
        )
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
        // V15 P1 17.8-D5：资产减值测试
        .route(
            "/fixed-assets/impairment-tests",
            post(fixed_asset_handler::create_impairment_test),
        )
        .route(
            "/fixed-assets/impairment-tests/:asset_id",
            get(fixed_asset_handler::get_impairment_tests),
        )
        .route(
            "/fixed-assets/impairment-tests/:id/approve",
            put(fixed_asset_handler::approve_impairment_test),
        )
        // V15 P1 17.8-D6：折旧政策变更
        .route(
            "/fixed-assets/depreciation-policy-changes",
            post(fixed_asset_handler::create_depreciation_policy_change),
        )
        .route(
            "/fixed-assets/depreciation-policy-changes/:asset_id",
            get(fixed_asset_handler::get_depreciation_policy_changes),
        )
        .route(
            "/fixed-assets/depreciation-policy-changes/:id/approve",
            put(fixed_asset_handler::approve_depreciation_policy_change),
        )
        .route(
            "/fixed-assets/:id/print",
            get(print_handler::fixed_asset_print_docx),
        )
        .route(
            "/fixed-assets/count/:id/print",
            get(print_handler::fixed_asset_count_print_docx),
        )
}

/// 预算主数据 + 调整路由（/budgets、/budgets/adjust）
fn budget_master_routes() -> Router<AppState> {
    Router::new()
        .route("/budgets", get(budget_management_handler::list_budgets))
        .route("/budgets", post(budget_management_handler::create_budget))
        // V15 P0-S12 修复（Batch 475e）：预算导出端点（必须在 /:id 之前注册，避免 axum matchit 把 "export" 当 :id 匹配）
        .route(
            "/budgets/export",
            get(budget_management_handler::export_budget_items),
        )
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
            "/budgets/adjust/:id/approve",
            post(budget_management_handler::approve_adjustment),
        )
        .route(
            "/budgets/adjust/:id/reject",
            post(budget_management_handler::reject_adjustment),
        )
        // V15 P1 17.7-D5：预算版本管理
        .route(
            "/budgets/versions",
            post(budget_management_handler::create_budget_version),
        )
        .route(
            "/budgets/versions/:plan_id",
            get(budget_management_handler::get_budget_versions),
        )
        .route(
            "/budgets/versions/:id/approve",
            put(budget_management_handler::approve_budget_version),
        )
}

/// 预算明细项路由（/budgets/items）
fn budget_item_routes() -> Router<AppState> {
    Router::new()
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
}

/// 预算计划路由（/budgets/plans）
fn budget_plan_routes() -> Router<AppState> {
    Router::new()
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
            "/budgets/plans/:id/reject",
            post(budget_management_handler::reject_plan),
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
}

/// 预算控制路由（/budgets/control）
fn budget_control_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/budgets/control/:plan_id",
            get(budget_management_handler::get_control),
        )
        .route(
            "/budgets/control/:plan_id/data",
            get(budget_management_handler::get_budget_control_data),
        )
}

/// 预算管理路由（合并主数据/明细项/计划/控制）
pub fn budgets() -> Router<AppState> {
    Router::new()
        .merge(budget_master_routes())
        .merge(budget_item_routes())
        .merge(budget_plan_routes())
        .merge(budget_control_routes())
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
        .route(
            "/financial-analysis/cash-flow-ratios",
            post(financial_analysis_handler::calculate_cash_flow_ratios),
        )
        .route(
            "/financial-analysis/trend-analysis",
            get(financial_analysis_handler::get_trend_analysis),
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
        // V15 P1 17.6-D5：调拨审批流
        .route(
            "/fund-management/transfers/pending",
            get(fund_management_handler::get_pending_transfers),
        )
        .route(
            "/fund-management/transfers/:id/approve",
            post(fund_management_handler::approve_transfer),
        )
        .route(
            "/fund-management/transfers/:id/reject",
            post(fund_management_handler::reject_transfer),
        )
        // V15 P1 17.6-D6：资金日报/月报
        .route(
            "/fund-management/reports/daily",
            get(fund_management_handler::get_fund_daily_report),
        )
        .route(
            "/fund-management/reports/monthly",
            get(fund_management_handler::get_fund_monthly_report),
        )
}

/// AP 应付账款路由：聚合各资源子路由（path 前缀互不重叠，merge 安全）
pub fn ap() -> Router<AppState> {
    Router::new()
        .merge(ap_invoice_routes())
        .merge(ap_payment_routes())
        .merge(ap_payment_request_routes())
        .merge(ap_verification_routes())
        .merge(ap_reconciliation_routes())
        .merge(ap_report_routes())
}

/// AP 应付发票路由（path 前缀 /ap/invoices）
fn ap_invoice_routes() -> Router<AppState> {
fn ap_invoice_routes() -> Router<AppState> {
    Router::new()
        .route("/ap/invoices", get(ap_invoice_handler::list_ap_invoices))
        .route("/ap/invoices", post(ap_invoice_handler::create_ap_invoice))
        // V15 P0-S12 修复（Batch 475e）：应付发票导出端点（必须在 /:id 之前注册，避免 axum matchit 把 "export" 当 :id 匹配）
        .route(
            "/ap/invoices/export",
            get(ap_invoice_handler::export_ap_invoices),
        )
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
        .route(
            "/ap/invoices/:id/print",
            get(print_handler::ap_invoice_print_docx),
        )
}

/// AP 付款路由（path 前缀 /ap/payments）
fn ap_payment_routes() -> Router<AppState> {
fn ap_payment_routes() -> Router<AppState> {
    Router::new()
        .route("/ap/payments", get(ap_payment_handler::list_payments))
        .route("/ap/payments", post(ap_payment_handler::create_payment))
        .route("/ap/payments/:id", get(ap_payment_handler::get_payment))
        .route("/ap/payments/:id", put(ap_payment_handler::update_payment))
        .route(
            "/ap/payments/:id/confirm",
            post(ap_payment_handler::confirm_payment),
        )
        .route(
            "/ap/payments/:id/print",
            get(print_handler::ap_payment_print_docx),
        )
}

/// AP 付款申请路由（path 前缀 /ap/payment-requests）
fn ap_payment_request_routes() -> Router<AppState> {
fn ap_payment_request_routes() -> Router<AppState> {
    Router::new()
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
            "/ap/payment-requests/:id/print",
            get(print_handler::ap_payment_request_print_docx),
        )
}

/// AP 核销路由（path 前缀 /ap/verifications）
fn ap_verification_routes() -> Router<AppState> {
    Router::new()
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
}

/// AP 对账路由（path 前缀 /ap/reconciliations 与 /ap/invoices/:id/relations）
fn ap_reconciliation_routes() -> Router<AppState> {
fn ap_reconciliation_routes() -> Router<AppState> {
    Router::new()
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
            "/ap/reconciliation/:id/print",
            get(print_handler::ap_reconciliation_print_docx),
        )
}

/// AP 报表路由（path 前缀 /ap/reports）
fn ap_report_routes() -> Router<AppState> {
    Router::new()
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

/// AR 应收发票路由（/ar/invoices）
fn ar_invoice_routes() -> Router<AppState> {
    Router::new()
        .route("/ar/invoices", get(ar_invoice_handler::list_ar_invoices))
        .route("/ar/invoices", post(ar_invoice_handler::create_ar_invoice))
        // V15 P0-S12 修复（Batch 475e）：应收发票导出端点（必须在 /:id 之前注册，避免 axum matchit 把 "export" 当 :id 匹配）
        .route(
            "/ar/invoices/export",
            get(ar_invoice_handler::export_ar_invoices),
        )
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
}

/// AR 应收收款路由（/ar/payments）
fn ar_payment_routes() -> Router<AppState> {
    Router::new()
        .route("/ar/payments", get(ar_payment_handler::list_payments))
        .route("/ar/payments", post(ar_payment_handler::create_payment))
        .route("/ar/payments/:id", get(ar_payment_handler::get_payment))
        .route("/ar/payments/:id", put(ar_payment_handler::update_payment))
        .route(
            "/ar/payments/:id/confirm",
            post(ar_payment_handler::confirm_payment),
        )
        .route(
            "/ar/payments/:id/cancel",
            post(ar_payment_handler::cancel_payment),
        )
        .route(
            "/ar/collections/:id/print",
            get(print_handler::ar_collection_print_docx),
        )
}

/// AR 核销路由（/ar/verifications）
fn ar_verification_routes() -> Router<AppState> {
    Router::new()
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
}

/// AR 报表路由（/ar/reports）
fn ar_report_routes() -> Router<AppState> {
    Router::new()
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
        .route(
            "/ar/reports/aging/by-salesperson",
            get(ar_report_handler::get_aging_by_salesperson),
        )
}

/// AR 应收账款路由（合并发票/收款/核销/报表）
pub fn ar() -> Router<AppState> {
    Router::new()
        .merge(ar_invoice_routes())
        .merge(ar_payment_routes())
        .merge(ar_verification_routes())
        .merge(ar_report_routes())
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

/// 应收对账路由（/ar-reconciliations，含 update/delete/send/confirm/dispute/close 端点）
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
        .route(
            "/ar-reconciliations/:id/print",
            get(print_handler::ar_reconciliation_print_docx),
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
        // 打印路由
        .route(
            "/exchange-rates/:id/print",
            get(print_handler::foreign_exchange_verification_print_docx),
        )
}

/// 财务域统一入口（通过 mod.rs nest 至 /api/v1/erp/finance）
pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(finance())
        // 显式使用 middleware 抑制未使用警告
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::rate_limit::rate_limit_by_ip,
        ))
}

/// 财务子模块路由（ap/ar/gl/fixed_assets/budgets 等，nest 至 /api/v1/erp）
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
        .merge(period_report_snapshots())
        .merge(aging_alert_rules())
}

/// 期末报表快照路由（/period-report-snapshots）
fn period_report_snapshots() -> Router<AppState> {
    Router::new()
        .route(
            "/period-report-snapshots",
            get(crate::handlers::period_report_snapshot_handler::list_snapshots)
                .post(crate::handlers::period_report_snapshot_handler::create_snapshot),
        )
        .route(
            "/period-report-snapshots/:id",
            get(crate::handlers::period_report_snapshot_handler::get_snapshot),
        )
        .route(
            "/period-report-snapshots/:id/verify",
            get(crate::handlers::period_report_snapshot_handler::verify_snapshot),
        )
}

/// 账龄预警规则路由（/aging-alert-rules）
fn aging_alert_rules() -> Router<AppState> {
    Router::new()
        .route(
            "/aging-alert-rules",
            get(crate::handlers::aging_alert_rule_handler::list_rules)
                .post(crate::handlers::aging_alert_rule_handler::create_rule),
        )
        .route(
            "/aging-alert-rules/:id",
            get(crate::handlers::aging_alert_rule_handler::get_rule)
                .put(crate::handlers::aging_alert_rule_handler::update_rule)
                .delete(crate::handlers::aging_alert_rule_handler::delete_rule),
        )
        .merge(asset_categories())
}

fn asset_categories() -> Router<AppState> {
    Router::new()
        .route(
            "/asset-categories",
            get(crate::handlers::asset_category_handler::list_categories)
                .post(crate::handlers::asset_category_handler::create_category),
        )
        .route(
            "/asset-categories/:id",
            get(crate::handlers::asset_category_handler::get_category)
                .put(crate::handlers::asset_category_handler::update_category)
                .delete(crate::handlers::asset_category_handler::delete_category),
        )
}
