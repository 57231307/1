//! 分析与高级功能域路由
//!
//! 处理双计量单位、辅助核算、业务追溯、扫码、报表引擎/增强、AI 分析、报表、审计、
//! 登录安全、邮件、导入导出、Webhook、API 密钥、数据权限、消息通知、用户通知偏好、
//! 交易管理、Advanced 分析、页面访问统计、跟踪等高级功能与系统级横切接口。
//!
//! 路由设计说明：`routes()` 入口用 `merge` + `nest` 混合策略。
//! - 内部 path 唯一（无 `GET /`、无 `GET /templates` 重复）的子 router 走 `merge`
//! - 内部 path 有重复（`GET /`、相同 `GET /templates` 等）的子 router 走 `nest` 加独立前缀
//! - nest 后的最终 path 与前端保持一致（如 `/reports/enhanced/...`）

use crate::utils::app_state::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::{
    advanced, ai_analysis_handler, api_key_handler, assist_accounting_handler,
    audit_enhanced_handler, barcode_scanner_handler, business_trace_handler,
    data_permission_handler, dual_unit_converter_handler, email_handler, import_export_handler,
    login_security_handler, notification_handler, report_engine_handler, report_enhanced_handler,
    tracking_handler, user_notification_setting_handler, webhook_handler,
    webhook_integration_handler,
};

/// 双计量单位路由
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

/// 辅助核算路由
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

/// 业务追溯路由
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

/// 扫码出库路由
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

/// 报表增强路由（内部 path 已加 /templates、/fields 等子前缀）
///
/// 整个 router 在 `routes()` 中用 `nest("/reports/enhanced", ...)` 装配，
/// 最终 path = `/api/v1/erp/reports/enhanced/...`（与前端一致）。
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
            get(report_enhanced_handler::subscriptions::list)
                .post(report_enhanced_handler::subscriptions::create),
        )
        .route(
            "/subscriptions/:id",
            get(report_enhanced_handler::subscriptions::get)
                .put(report_enhanced_handler::subscriptions::update)
                .delete(report_enhanced_handler::subscriptions::delete),
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

/// 导入路由
pub fn imports() -> Router<AppState> {
    Router::new()
        .route("/csv", post(import_export_handler::import_csv))
        .route("/excel", post(import_export_handler::import_excel))
        .route(
            "/templates/download/:import_type",
            get(import_export_handler::download_template),
        )
}

/// 导出路由
pub fn exports() -> Router<AppState> {
    Router::new()
        .route("/csv/:export_type", get(import_export_handler::export_csv))
        .route(
            "/excel/:export_type",
            get(import_export_handler::export_excel_type),
        )
}

/// 审计日志路由
pub fn audit() -> Router<AppState> {
    Router::new()
        .route("/logs", get(audit_enhanced_handler::list_audit_logs))
        .route(
            "/logs/export",
            get(audit_enhanced_handler::export_audit_logs),
        )
}

/// 登录安全路由
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

/// 邮件路由
pub fn emails() -> Router<AppState> {
    Router::new()
        .route("/send", post(email_handler::send_email))
        .route(
            "/email-templates",
            get(email_handler::list).post(email_handler::create),
        )
        .route(
            "/email-templates/:id",
            get(email_handler::get)
                .put(email_handler::update)
                .delete(email_handler::delete),
        )
        .route("/email-records", get(email_handler::get_email_records))
        .route(
            "/email-statistics",
            get(email_handler::get_email_statistics),
        )
}

/// Webhook 集成路由（内部 path 保留 `/`、`/callback` 等，nest 装配时再加前缀）
///
/// 最终 path = `/api/v1/erp/webhooks/integrations/...`（与前端一致）。
pub fn webhook_integrations() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(webhook_integration_handler::list_integrations)
                .post(webhook_integration_handler::create_integration),
        )
        .route(
            "/integration/:id",
            put(webhook_integration_handler::test_integration)
                .delete(webhook_integration_handler::delete_integration),
        )
        .route(
            "/callback",
            post(webhook_integration_handler::handle_generic_callback),
        )
        .route(
            "/test-integration/:id",
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

/// AI 智能分析路由
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

/// 报表引擎路由
pub fn reports() -> Router<AppState> {
    Router::new()
        .route(
            "/report-templates",
            get(report_engine_handler::list_templates),
        )
        .route("/execute", get(report_engine_handler::execute_report))
        .route("/export", get(report_engine_handler::export_report))
        .route("/aggregate", post(report_engine_handler::aggregate_report))
        .route(
            "/cache/clear",
            post(report_engine_handler::clear_report_cache),
        )
}

/// Webhook 路由
///
/// 注：main 上 webhook_handler 仅实现 `list_webhooks` / `create_webhook` / `delete_webhook`
/// 三类基础操作；`retry_webhook` / `get_webhook_logs` / `test_webhook` 在 main 上未实现，
/// 因此相关路由暂不挂载，避免编译期 E0425。
pub fn webhooks() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(webhook_handler::list_webhooks).post(webhook_handler::create_webhook),
        )
        .route("/:id", delete(webhook_handler::delete_webhook))
}

/// API 密钥路由
///
/// 注：main 上 api_key_handler 没有 `delete_api_key`（撤销 = 删除），仅有 `revoke_api_key`，
/// 因此 `DELETE /api-key/:id` 直接复用 `revoke_api_key`。
pub fn api_keys() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(api_key_handler::list_api_keys).post(api_key_handler::create_api_key),
        )
        .route("/api-key/:id", delete(api_key_handler::revoke_api_key))
}

/// 数据权限路由
///
/// 注：main 上 data_permission_handler 仅实现基础的 `list_data_permissions` /
/// `get_data_permission` / `set_data_permission` / `delete_data_permission` /
/// `list_scope_types` / `list_role_data_permissions`，未提供 grant/revoke 角色 / 用户级别
/// 的接口，因此这些路由暂不挂载，避免编译期 E0425。
pub fn data_permissions() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(data_permission_handler::list_data_permissions)
                .post(data_permission_handler::set_data_permission),
        )
        .route(
            "/:id",
            get(data_permission_handler::get_data_permission)
                .delete(data_permission_handler::delete_data_permission),
        )
        .route(
            "/scope-types",
            get(data_permission_handler::list_scope_types),
        )
        .route(
            "/roles/:role_id",
            get(data_permission_handler::list_role_data_permissions),
        )
}

/// 消息通知路由
///
/// 注：main 上 notification_handler 未提供 `create_notification` / `update_notification`，
/// 因此这两类路由暂不挂载，避免编译期 E0425。
pub fn notifications() -> Router<AppState> {
    Router::new()
        .route("/", get(notification_handler::list_notifications))
        .route(
            "/notification/:id",
            get(notification_handler::get_notification)
                .delete(notification_handler::delete_notification),
        )
        .route(
            "/notification/:id/read",
            post(notification_handler::mark_as_read),
        )
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
}

/// 用户通知偏好路由
///
/// 注：main 上 user_notification_setting_handler 未提供 `reset_to_default`，
/// 因此 `POST /reset` 路由暂不挂载，避免编译期 E0425。
pub fn user_notification_settings() -> Router<AppState> {
    Router::new().route(
        "/",
        get(user_notification_setting_handler::get_setting)
            .put(user_notification_setting_handler::update_setting),
    )
}

/// 交易管理路由（高级查询）
///
/// 注：实际定义在 `advanced/` 子模块（拆分到 `advanced/reorder.rs` 和 `advanced/decide.rs`），
/// 路由挂在 `/advanced` 域下更合适；此处保留独立 `/trading/...` 入口以兼容旧前端调用。
pub fn trading() -> Router<AppState> {
    Router::new()
        .route(
            "/purchase-contracts",
            get(advanced::list_purchase_contracts),
        )
        .route("/sales-contracts", get(advanced::list_sales_contracts))
        .route("/sales-prices", get(advanced::list_sales_prices))
        .route("/purchase-prices", get(advanced::list_purchase_prices))
        .route("/sales-returns", get(advanced::list_sales_returns))
}

/// Advanced 分析路由（nest 到 /api/v1/erp/advanced）
///
/// 内部 path 与前端 `/advanced/ai/...`、`/advanced/reports/...`、
/// `/advanced/tenants/...` 完全一致。
pub fn advanced() -> Router<AppState> {
    Router::new()
        .route("/ai/sales-forecast", post(advanced::sales_forecast))
        .route(
            "/ai/inventory-optimization",
            post(advanced::inventory_optimization),
        )
        .route("/ai/anomaly-detection", post(advanced::anomaly_detection))
        .route("/ai/recommendations", post(advanced::recommendations))
        .route("/reports/templates", get(advanced::list_report_templates))
        .route("/reports/execute", post(advanced::execute_report))
        .route("/reports/export", post(advanced::export_report))
        .route(
            "/tenants",
            get(advanced::list_tenants).post(advanced::create_tenant),
        )
        .route(
            "/tenants/:id",
            get(advanced::get_tenant).put(advanced::update_tenant),
        )
}

/// 跟踪路由
///
/// 注：main 上 tracking_handler 仅提供 `track_page_view` 基础接口（用于 POST /page-view），
/// 其余 `get_page_view_stats` / `get_page_view_stats_by_day` / `get_popular_pages` /
/// `record_behavior` / `get_funnel_analysis` / `get_user_path` 在 main 上未实现，
/// 因此这些统计 / 行为分析路由暂不挂载，避免编译期 E0425。
pub fn tracking() -> Router<AppState> {
    Router::new().route("/page-view", post(tracking_handler::track_page_view))
}

/// 分析域统一入口
///
/// - 无 path 重复的子 router 走 `merge`
/// - 有 path 重复（`GET /`、`GET /templates` 等）的子 router 走 `nest` 加独立前缀，
///   这样最终 path 与前端保持一致（如 `/reports/enhanced/...`、`/webhooks/...`）。
/// - `advanced()` 内部 path 以 `/reports`、`/ai` 等开头，nest 到 `/advanced`
///   后最终 path = `/advanced/reports/...`、`/advanced/ai/...`，与前端调用一致。
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(dual_unit())
        .nest("/assist-accounting", assist_accounting())
        .nest("/business-trace", business_trace())
        .nest("/scanner", scanner())
        .merge(imports())
        .merge(exports())
        .merge(audit())
        .merge(security())
        .merge(emails())
        .merge(ai())
        .merge(reports())
        .nest("/trading", trading())
        .merge(tracking())
        // 内部有 `GET /` 重复的子 router 走 nest 加独立前缀
        .nest("/reports/enhanced", reports_enhanced())
        .nest("/webhooks/integrations", webhook_integrations())
        .nest("/webhooks", webhooks())
        .nest("/api-keys", api_keys())
        .nest("/data-permissions", data_permissions())
        .nest("/notifications", notifications())
        .nest("/user-notification-settings", user_notification_settings())
        .nest("/advanced", advanced())
}
