//! 租户域路由
//!
//! 处理租户管理、租户配置、租户计费等 SaaS 多租户相关接口。

use crate::utils::app_state::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::{tenant_billing_handler, tenant_config_handler, tenant_handler};

/// 租户管理路由（nest 到 /api/v1/erp/tenants）
pub fn tenants() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(tenant_handler::list_tenants).post(tenant_handler::create_tenant),
        )
        .route("/:id", get(tenant_handler::get_tenant))
        .route("/:id/status", put(tenant_handler::update_tenant_status))
}

/// 租户配置路由（nest 到 /api/v1/erp/tenants/config）
pub fn tenant_config() -> Router<AppState> {
    Router::new()
        .route(
            "/settings",
            get(tenant_config_handler::list_configs).post(tenant_config_handler::set_config),
        )
        .route(
            "/settings/:key",
            delete(tenant_config_handler::delete_config),
        )
        .route(
            "/plans",
            get(tenant_config_handler::list_plans).post(tenant_config_handler::create_plan),
        )
        .route("/plans/:id", get(tenant_config_handler::get_plan))
        .route("/usage", get(tenant_config_handler::get_usage_statistics))
}

/// 租户计费路由（nest 到 /api/v1/erp/tenants/billing）
pub fn tenant_billing() -> Router<AppState> {
    Router::new()
        .route("/plan", get(tenant_billing_handler::get_current_plan))
        .route("/upgrade", post(tenant_billing_handler::upgrade_plan))
        .route("/usage", get(tenant_billing_handler::get_usage))
        .route("/invoices", get(tenant_billing_handler::list_invoices))
        .route("/renew", post(tenant_billing_handler::renew_subscription))
}

/// 租户域统一入口
///
/// FE-A-2 修复（2026-06-26 第二次审计第二优先级）：
/// 原实现 `routes()` 用 `.merge(tenant_config()) + .merge(tenant_billing())`，
/// 前端按 `/tenants/billing/*`、`/tenants/config/*` 调用，但 merge 后路径是
/// `/tenants/plan`、`/tenants/settings` 等，缺少 billing/config 子前缀。
/// 改为 nest 加独立子前缀，路径与前端一致：
/// - `/tenants/`、`/tenants/:id` 等（保留 merge）
/// - `/tenants/config/settings`、`/tenants/config/plans` 等
/// - `/tenants/billing/plan`、`/tenants/billing/upgrade` 等
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(tenants())
        .nest("/config", tenant_config())
        .nest("/billing", tenant_billing())
}
