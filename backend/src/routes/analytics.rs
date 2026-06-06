//! 分析与高级功能域路由
//!
//! 处理双计量单位、辅助核算、业务追溯、扫码、报表引擎/增强、AI 分析、报表、审计、
//! 登录安全、邮件、导入导出、Webhook、API 密钥、数据权限、消息通知、用户通知偏好、
//! 交易管理、Advanced 分析、页面访问统计、跟踪等高级功能与系统级横切接口。

use crate::utils::app_state::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::{
    advanced_handler, ai_analysis_handler, api_key_handler, assist_accounting_handler,
    audit_enhanced_handler, barcode_scanner_handler, business_trace_handler,
    data_permission_handler, dual_unit_converter_handler, email_handler, import_export_handler,
    login_security_handler, notification_handler, report_engine_handler, report_enhanced_handler,
    tracking_handler, user_notification_setting_handler, webhook_handler,
    webhook_integration_handler,
};

/// 双计量单位路由（nest 到 /api/v1/erp/dual-unit）
pub fn dual_unit() -> Router<AppState> {
    Router::new()
        .route(
            "/convert",
            post(dual_unit_converter_handler::convert_dual_unit),
        )
        .route(
            "/validate",
            post(dual_unit_converter_handler::validate_dual_unit),
        )
}

/// 辅助核算路由（nest 到 /api/v1/erp/assist-accounting）
pub fn assist_accounting() -> Router<AppState> {
    Router::new()
        .route(
            "/dimensions",
            get(assist_accounting_handler::list_assist_dimensions),
        )
        .route(
            "/records",
            get(assist_accounting_handler::query_assist_records),
        )
        .route(
            "/records/business",
            get(assist_accounting_handler::get_assist_records_by_business),
        )
        .route(
            "/records/five-dimension/:five_dimension_id",
            get(assist_accounting_handler::get_assist_records_by_five_dimension),
        )
        .route(
            "/summary",
            get(assist_accounting_handler::get_assist_summary),
        )
}

/// 业务追溯路由（nest 到 /api/v1/erp/business-trace）
pub fn business_trace() -> Router<AppState> {
    Router::new()
        .route(
            "/five-dimension/:five_dimension_id",
            get(business_trace_handler::get_trace_by_five_dimension),
        )
        .route("/forward", get(business_trace_handler::forward_trace))
        .route("/backward", get(business_trace_handler::backward_trace))
        .route(
            "/snapshot/:trace_chain_id",
            post(business_trace_handler::create_trace_snapshot),
        )
}

/// 扫码出库路由（nest 到 /api/v1/erp/scanner）
pub fn scanner() -> Router<AppState> {
    Router::new()
        .route(
            "/scan-to-ship",
            get(barcode_scanner_handler::scan_to_ship_get)
                .post(barcode_scanner_handler::scan_to_ship_post),
        )
        .route(
            "/scan-inventory",
            get(barcode_scanner_handler::scan_inventory),
        )
        .route("/history", get(barcode_scanner_handler::scan_history))
        .route(
            "/scan-statistics",
            get(barcode_scanner_handler::scan_statistics),
        )
}

/// 报表增强路由（nest 到 /api/v1/erp/reports/enhanced）
pub fn reports_enhanced() -> Router<AppState> {
    Router::new()
        .route(
            "/fields/:template_type",
            get(report_enhanced_handler::get_available_fields),
        )
        .route(
            "/templates",
            get(report_enhanced_handler::list_report_templates)
                .post(report_enhanced_handler::create_report_template),
        )
        .route(
            "/templates/:id",
            get(report_enhanced_handler::get_report_template)
                .put(report_enhanced_handler::update_report_template)
                .delete(report_enhanced_handler::delete_report_template),
        )
        .route(
            "/templates/:id/execute",
            post(report_enhanced_handler::execute_custom_report),
        )
        .route(
            "/templates/:id/export",
            post(report_enhanced_handler::export_template),
        )
        .route(
            "/templates/:id/preview",
            get(report_enhanced_handler::preview_template),
        )
        .route("/export/pdf", post(report_enhanced_handler::export_pdf))
        .route("/export/excel", post(report_enhanced_handler::export_excel))
        .route(
            "/subscriptions",
            get(report_enhanced_handler::list_subscriptions)
                .post(report_enhanced_handler::create_subscription),
        )
        .route(
            "/subscriptions/:id",
            get(report_enhanced_handler::get_subscription)
                .put(report_enhanced_handler::update_subscription)
                .delete(report_enhanced_handler::delete_subscription),
        )
        .route(
            "/subscriptions/:id/toggle",
            post(report_enhanced_handler::toggle_subscription),
        )
        .route(
            "/subscriptions/:id/trigger",
            post(report_enhanced_handler::trigger_subscription),
        )
        .route(
            "/subscriptions/:id/send",
            post(report_enhanced_handler::send_subscription_now),
        )
}

/// 导入路由（nest 到 /api/v1/erp/import）
pub fn imports() -> Router<AppState> {
    Router::new()
        .route("/csv", post(import_export_handler::import_csv))
        .route("/excel", post(import_export_handler::import_excel))
        .route(
            "/templates/download/:import_type",
            get(import_export_handler::download_template),
        )
}

/// 导出路由（nest 到 /api/v1/erp/export）
pub fn exports() -> Router<AppState> {
    Router::new()
        .route("/csv/:export_type", get(import_export_handler::export_csv))
        .route(
            "/excel/:export_type",
            get(import_export_handler::export_excel_type),
        )
}

/// 审计日志路由（nest 到 /api/v1/erp/audit）
pub fn audit() -> Router<AppState> {
    Router::new()
        .route("/logs", get(audit_enhanced_handler::list_audit_logs))
        .route(
            "/logs/export",
            get(audit_enhanced_handler::export_audit_logs),
        )
}

/// 登录安全路由（nest 到 /api/v1/erp/security）
pub fn security() -> Router<AppState> {
    Router::new()
        .route("/login-logs", get(login_security_handler::list_login_logs))
        .route(
            "/lock-status",
            get(login_security_handler::check_lock_status),
        )
        .route("/unlock", post(login_security_handler::unlock_account))
        .route(
            "/login-statistics",
            get(login_security_handler::get_login_statistics),
        )
        .route("/stats", get(login_security_handler::get_login_statistics))
        .route(
            "/security-alerts",
            get(login_security_handler::get_security_alerts),
        )
        .route("/alerts", get(login_security_handler::get_security_alerts))
        .route(
            "/alerts/:id/resolve",
            post(login_security_handler::resolve_alert),
        )
        .route(
            "/locked-accounts",
            get(login_security_handler::get_locked_accounts),
        )
        .route(
            "/locked-accounts/:id/unlock",
            post(login_security_handler::unlock_account_by_id),
        )
        .route(
            "/login-logs/export",
            get(login_security_handler::export_login_logs),
        )
}

/// 邮件路由（nest 到 /api/v1/erp/emails）
pub fn emails() -> Router<AppState> {
    Router::new()
        .route("/send", post(email_handler::send_email))
        .route(
            "/templates",
            get(email_handler::list_templates).post(email_handler::create_template),
        )
        .route(
            "/templates/:id",
            get(email_handler::get_template)
                .put(email_handler::update_template)
                .delete(email_handler::delete_template),
        )
        .route("/records", get(email_handler::get_email_records))
        .route(
            "/email-statistics",
            get(email_handler::get_email_statistics),
        )
}

/// Webhook 集成路由（nest 到 /api/v1/erp/webhooks/integrations）
pub fn webhook_integrations() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(webhook_integration_handler::list_integrations)
                .post(webhook_integration_handler::create_integration),
        )
        .route(
            "/:id",
            put(webhook_integration_handler::test_integration)
                .delete(webhook_integration_handler::delete_integration),
        )
        .route(
            "/callback",
            post(webhook_integration_handler::handle_generic_callback),
        )
        .route(
            "/test/:id",
            post(webhook_integration_handler::test_integration),
        )
        .route(
            "/wechat/send",
            post(webhook_integration_handler::send_wechat_message),
        )
        .route(
            "/dingtalk/send",
            post(webhook_integration_handler::send_dingtalk_message),
        )
}

/// AI 智能分析路由（nest 到 /api/v1/erp/ai）
pub fn ai() -> Router<AppState> {
    Router::new()
        .route("/forecast-sales", get(ai_analysis_handler::forecast_sales))
        .route(
            "/optimize-inventory",
            get(ai_analysis_handler::optimize_inventory),
        )
        .route(
            "/detect-anomalies",
            get(ai_analysis_handler::detect_anomalies),
        )
        .route(
            "/recommendations",
            get(ai_analysis_handler::get_recommendations),
        )
}

/// 报表引擎路由（nest 到 /api/v1/erp/reports）
pub fn reports() -> Router<AppState> {
    Router::new()
        .route("/templates", get(report_engine_handler::list_templates))
        .route("/execute", get(report_engine_handler::execute_report))
        .route("/export", get(report_engine_handler::export_report))
        .route("/aggregate", post(report_engine_handler::aggregate_report))
        .route(
            "/cache/clear",
            post(report_engine_handler::clear_report_cache),
        )
}

/// Webhook 路由（nest 到 /api/v1/erp/webhooks）
pub fn webhooks() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(webhook_handler::list_webhooks).post(webhook_handler::create_webhook),
        )
        .route("/:id", delete(webhook_handler::delete_webhook))
}

/// API 密钥路由（nest 到 /api/v1/erp/api-keys）
pub fn api_keys() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(api_key_handler::list_api_keys).post(api_key_handler::create_api_key),
        )
        .route("/:id/revoke", post(api_key_handler::revoke_api_key))
}

/// 数据权限路由（nest 到 /api/v1/erp/data-permissions）
pub fn data_permissions() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(data_permission_handler::list_data_permissions)
                .post(data_permission_handler::set_data_permission),
        )
        .route(
            "/scope-types",
            get(data_permission_handler::list_scope_types),
        )
        .route(
            "/roles/:role_id",
            get(data_permission_handler::list_role_data_permissions),
        )
        .route(
            "/roles/:role_id/:resource_type",
            get(data_permission_handler::get_data_permission)
                .delete(data_permission_handler::delete_data_permission),
        )
}

/// 消息通知路由（nest 到 /api/v1/erp/notifications）
pub fn notifications() -> Router<AppState> {
    Router::new()
        .route("/", get(notification_handler::list_notifications))
        .route("/unread-count", get(notification_handler::get_unread_count))
        .route("/read-all", post(notification_handler::mark_all_as_read))
        .route(
            "/batch-read",
            post(notification_handler::batch_mark_as_read),
        )
        .route(
            "/settings",
            get(notification_handler::get_settings).put(notification_handler::update_setting),
        )
        .route("/:id", get(notification_handler::get_notification))
        .route("/:id/read", post(notification_handler::mark_as_read))
        .route("/:id", delete(notification_handler::delete_notification))
}

/// 用户通知偏好设置路由（nest 到 /api/v1/erp/user/notification-setting）
pub fn user_notification_settings() -> Router<AppState> {
    Router::new().route(
        "/",
        get(user_notification_setting_handler::get_setting)
            .put(user_notification_setting_handler::update_setting),
    )
}

/// 交易管理路由（nest 到 /api/v1/erp/trading）
pub fn trading() -> Router<AppState> {
    Router::new()
        .route(
            "/purchase-contracts",
            get(advanced_handler::list_purchase_contracts)
                .post(advanced_handler::create_purchase_contract),
        )
        .route(
            "/purchase-contracts/:id",
            get(advanced_handler::get_purchase_contract)
                .put(advanced_handler::update_purchase_contract)
                .delete(advanced_handler::delete_purchase_contract),
        )
        .route(
            "/purchase-contracts/:id/approve",
            post(advanced_handler::approve_purchase_contract),
        )
        .route(
            "/purchase-contracts/:id/execute",
            post(advanced_handler::execute_purchase_contract),
        )
        .route(
            "/purchase-prices",
            get(advanced_handler::list_purchase_prices)
                .post(advanced_handler::create_purchase_price),
        )
        .route(
            "/purchase-prices/:id",
            put(advanced_handler::update_purchase_price)
                .delete(advanced_handler::delete_purchase_price),
        )
        .route(
            "/purchase-prices/:id/approve",
            post(advanced_handler::approve_purchase_price),
        )
        .route(
            "/sales-contracts",
            get(advanced_handler::list_sales_contracts)
                .post(advanced_handler::create_sales_contract),
        )
        .route(
            "/sales-contracts/:id",
            get(advanced_handler::get_sales_contract)
                .put(advanced_handler::update_sales_contract)
                .delete(advanced_handler::delete_sales_contract),
        )
        .route(
            "/sales-contracts/:id/approve",
            post(advanced_handler::approve_sales_contract),
        )
        .route(
            "/sales-prices",
            get(advanced_handler::list_sales_prices).post(advanced_handler::create_sales_price),
        )
        .route(
            "/sales-prices/:id",
            put(advanced_handler::update_sales_price).delete(advanced_handler::delete_sales_price),
        )
        .route(
            "/sales-prices/:id/approve",
            post(advanced_handler::approve_sales_price),
        )
        .route(
            "/sales-returns",
            get(advanced_handler::list_sales_returns).post(advanced_handler::create_sales_return),
        )
        .route(
            "/sales-returns/:id",
            get(advanced_handler::get_sales_return)
                .put(advanced_handler::update_sales_return)
                .delete(advanced_handler::delete_sales_return),
        )
}

/// Advanced 分析路由（nest 到 /api/v1/erp/advanced）
pub fn advanced() -> Router<AppState> {
    Router::new()
        .route("/ai/sales-forecast", post(advanced_handler::sales_forecast))
        .route(
            "/ai/inventory-optimization",
            post(advanced_handler::inventory_optimization),
        )
        .route(
            "/ai/anomaly-detection",
            post(advanced_handler::anomaly_detection),
        )
        .route(
            "/ai/recommendations",
            post(advanced_handler::recommendations),
        )
        .route(
            "/reports/templates",
            get(advanced_handler::list_report_templates),
        )
        .route("/reports/execute", post(advanced_handler::execute_report))
        .route("/reports/export", post(advanced_handler::export_report))
        .route(
            "/tenants",
            get(advanced_handler::list_tenants).post(advanced_handler::create_tenant),
        )
        .route(
            "/tenants/:id",
            get(advanced_handler::get_tenant).put(advanced_handler::update_tenant),
        )
}

/// 页面访问统计路由（nest 到 /api/tracking）
pub fn tracking() -> Router<AppState> {
    Router::new().route("/page-view", post(tracking_handler::track_page_view))
}

/// 分析域统一入口
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(dual_unit())
        .merge(assist_accounting())
        .merge(business_trace())
        .merge(scanner())
        .merge(reports_enhanced())
        .merge(imports())
        .merge(exports())
        .merge(audit())
        .merge(security())
        .merge(emails())
        .merge(webhook_integrations())
        .merge(ai())
        .merge(reports())
        .merge(webhooks())
        .merge(api_keys())
        .merge(data_permissions())
        .merge(notifications())
        .merge(user_notification_settings())
        .merge(trading())
        .merge(advanced())
        .merge(tracking())
}
