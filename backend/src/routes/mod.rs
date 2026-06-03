//! 路由总装配入口
//!
//! 本文件由 14 个业务域子 routes 文件 + 必要的基础设施（Prometheus 指标、Swagger UI、静态资源）
//! 拼装而成，原始 2659 行的单体 mod.rs 已拆分至子文件。
//!
//! 路径规划（与原 mod.rs 完全等价）：
//! - `/api/v1/erp/auth`        -> auth::routes()
//! - `/api/v1/erp`             -> iam / catalog / analytics / system 四域合并（共享同一前缀，必须用 merge）
//! - `/api/v1/erp/inventory`   -> inventory::routes()
//! - `/api/v1/erp/sales`       -> sales::routes()
//! - `/api/v1/erp/purchase`    -> purchase::routes()
//! - `/api/v1/erp/finance`     -> finance::routes(state)  唯一需要传 state 的域
//! - `/api/v1/erp/production`  -> production::routes()
//! - `/api/v1/erp/crm`         -> crm::routes()
//! - `/api/v1/erp/tenants`     -> tenant::routes()
//! - `/api/v1`                 -> v1::routes()  占位
//! - `/static/*` `/bingxi_frontend.*` -> static_routes::static_assets_handler()
//! - `/metrics`                -> create_metrics_router()
//! - `/swagger-ui` `/api-docs/openapi.json` -> SwaggerUi
//!
//! 中间件（仅 SQL 注入审计）：
//!   - security_headers（6 个安全响应头）已由 main.rs 通过 SetResponseHeaderLayer 统一设置，
//!     不在本文件重复挂载（避免重复 layer 导致 header 覆盖异常）。
//!   - sql_injection_audit_middleware（新）挂载在 Router 链最外层（axum 后注册 = 外层）。

use axum::{middleware, Router};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::middleware::sql_injection_audit::sql_injection_audit_middleware;
use crate::services::metrics_service::create_metrics_router;
use crate::utils::app_state::AppState;

// 14 个业务域子模块
// 注：源文件名为 static.rs，但 `static` 是 Rust 关键字，用 #[path] 重映射为 static_routes 模块名
pub mod analytics;
pub mod auth;
pub mod catalog;
pub mod crm;
pub mod finance;
pub mod iam;
pub mod inventory;
pub mod production;
pub mod purchase;
pub mod sales;
#[path = "static.rs"]
pub mod static_routes;
pub mod system;
pub mod tenant;
pub mod v1;

/// 创建主路由
///
/// 将 14 个业务域子路由 + 监控 / 文档 / 静态资源拼装为统一入口。
/// 顶层只保留路径装配与 SQL 注入审计中间件挂载，所有具体路由定义下沉到子文件。
pub fn create_router(state: AppState) -> Router {
    // 共享 /api/v1/erp 同一前缀的四个域（iam / catalog / analytics / system）
    // 必须先 merge 再整体 nest，否则连续 .nest("/api/v1/erp", ...) 会因后注册路由被前一个覆盖
    let erp_root = iam::routes()
        .merge(catalog::routes())
        .merge(analytics::routes())
        .merge(system::routes());

    Router::new()
        // ---- 14 个业务域 ----
        .nest("/api/v1/erp", erp_root)
        .nest("/api/v1/erp/auth", auth::routes())
        .nest("/api/v1/erp/inventory", inventory::routes())
        .nest("/api/v1/erp/sales", sales::routes())
        .nest("/api/v1/erp/purchase", purchase::routes())
        .nest("/api/v1/erp/finance", finance::routes(state.clone()))
        .nest("/api/v1/erp/production", production::routes())
        .nest("/api/v1/erp/crm", crm::routes())
        .nest("/api/v1/erp/tenants", tenant::routes())
        // v1 占位入口
        .nest("/api/v1", v1::routes())
        // ---- 基础设施 ----
        // 静态资源（CSS / JS / WASM / 字体等）
        .merge(static_routes::static_assets_handler())
        // Prometheus 指标（/metrics）
        .merge(create_metrics_router())
        // Swagger UI（/swagger-ui + /api-docs/openapi.json）
        .merge(
            SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", crate::docs::ApiDoc::openapi()),
        )
        // ---- 中间件 ----
        // SQL 注入审计（命中危险模式直接 400，不进入 handler）
        // 注：security_headers 由 main.rs 的 SetResponseHeaderLayer 统一处理（含 CSP/HSTS/XFO/X-CTO/Referrer-Policy/XSS）
        .layer(middleware::from_fn(sql_injection_audit_middleware))
        // ---- 全局状态注入（必须最后调用）----
        .with_state(state)
}
