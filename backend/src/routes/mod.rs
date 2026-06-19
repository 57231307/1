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

use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
#[cfg(feature = "swagger")]
use utoipa_swagger_ui::SwaggerUi;

use crate::handlers::{
    import_export_handler, material_shortage_handler, print_handler, quality_standard_handler,
    scheduling_handler, system_update_handler, user_handler,
};
use crate::middleware::sql_injection_audit::sql_injection_audit_middleware;
use crate::services::metrics_service::create_metrics_router;
use crate::utils::app_state::AppState;

// 14 个业务域子模块
// 注：源文件名为 static.rs，但 `static` 是 Rust 关键字，用 #[path] 重映射为 static_routes 模块名
pub mod analytics;
pub mod auth;
pub mod catalog;
pub mod crm;
pub mod failover;
pub mod finance;
pub mod iam;
pub mod inventory;
pub mod production;
pub mod purchase;
pub mod sales;
// 销售报价单模块（Week 1）
pub mod quotations;
// 定制订单全流程跟踪模块（P0-3）
pub mod custom_order;
// 色卡仓储管理模块（P0-4）
pub mod color_card;
// 面料多色号定价扩展路由（P0-5）
pub mod color_price;
#[path = "static.rs"]
pub mod static_routes;
pub mod system;
pub mod tenant;
pub mod v1;

/// 缺料预警路由（从 production 域提升到根级，path 前缀 /material-shortage）
fn material_shortage_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/material-shortage/alerts",
            get(material_shortage_handler::list_shortage_alerts),
        )
        .route(
            "/material-shortage/list",
            get(material_shortage_handler::list_shortage_alerts),
        )
        .route(
            "/material-shortage/check",
            post(material_shortage_handler::check_material_shortage),
        )
        .route(
            "/material-shortage/summary",
            get(material_shortage_handler::get_shortage_summary),
        )
        .route(
            "/material-shortage/threshold",
            get(material_shortage_handler::get_threshold_config)
                .post(material_shortage_handler::save_threshold_config),
        )
        .route(
            "/material-shortage/replenishment",
            get(material_shortage_handler::get_replenishment_suggestions),
        )
        .route(
            "/material-shortage/:id/status",
            put(material_shortage_handler::update_shortage_status),
        )
}

/// 生产排程路由（从 production 域提升到根级，path 前缀 /scheduling）
fn scheduling_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/scheduling/auto-schedule",
            post(scheduling_handler::auto_schedule),
        )
        .route("/scheduling/gantt", get(scheduling_handler::get_gantt_data))
        .route(
            "/scheduling/conflicts",
            get(scheduling_handler::detect_conflicts),
        )
        .route(
            "/scheduling/tasks",
            get(scheduling_handler::list_scheduled_orders),
        )
        .route(
            "/scheduling/tasks/:id/adjust",
            put(scheduling_handler::adjust_schedule_task),
        )
        .route("/scheduling/:id", put(scheduling_handler::adjust_schedule))
        .route(
            "/scheduling/work-orders",
            get(scheduling_handler::list_scheduled_orders),
        )
        .route(
            "/scheduling/history",
            get(scheduling_handler::get_schedule_history),
        )
        .route(
            "/scheduling/results/:id",
            get(scheduling_handler::get_schedule_result),
        )
        .route(
            "/scheduling/results/:id/confirm",
            post(scheduling_handler::confirm_schedule_result),
        )
}

/// 质量标准路由（从 production 域提升到根级，path 前缀 /quality-standards）
fn quality_standards_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/quality-standards",
            get(quality_standard_handler::list_standards),
        )
        .route(
            "/quality-standards",
            post(quality_standard_handler::create_standard),
        )
        .route(
            "/quality-standards/:id",
            get(quality_standard_handler::get_standard),
        )
        .route(
            "/quality-standards/:id",
            put(quality_standard_handler::update_standard),
        )
        .route(
            "/quality-standards/:id",
            delete(quality_standard_handler::delete_standard),
        )
        .route(
            "/quality-standards/:id/versions",
            get(quality_standard_handler::list_versions),
        )
        .route(
            "/quality-standards/:id/versions",
            post(quality_standard_handler::create_version_history),
        )
        .route(
            "/quality-standards/:id/approve",
            post(quality_standard_handler::approve_standard),
        )
        .route(
            "/quality-standards/:id/publish",
            post(quality_standard_handler::publish_standard),
        )
}

/// 用户中心路由（path 前缀 /user）
fn user_profile_routes() -> Router<AppState> {
    Router::new().route("/user/profile", get(user_handler::get_current_user_profile))
}

/// 系统更新补充路由（path 前缀 /system-update）
fn system_update_extra_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/system-update/current-version",
            get(system_update_handler::get_version),
        )
        .route(
            "/system-update/tasks",
            get(system_update_handler::get_update_status),
        )
        .route(
            "/system-update/backups",
            get(system_update_handler::get_backup_versions),
        )
}

/// 打印模板路由（path 前缀 /print-templates）
fn print_templates_routes() -> Router<AppState> {
    Router::new()
        .route("/print-templates", get(print_handler::list_print_templates))
        .route(
            "/print-templates/:id",
            get(print_handler::get_print_template),
        )
}

/// 数据导入路由（path 前缀 /data-import）
fn data_import_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/data-import/templates",
            get(import_export_handler::list_import_templates),
        )
        .route(
            "/data-import/tasks",
            get(import_export_handler::list_import_tasks),
        )
}

/// 产品分类路由别名（path 前缀 /product-categories，指向 categories handler）
fn product_categories_alias() -> Router<AppState> {
    Router::new()
        .route(
            "/product-categories",
            get(crate::handlers::product_category_handler::list),
        )
        .route(
            "/product-categories",
            post(crate::handlers::product_category_handler::create),
        )
        .route(
            "/product-categories/tree",
            get(crate::handlers::product_category_handler::get_product_category_tree),
        )
        .route(
            "/product-categories/:id",
            get(crate::handlers::product_category_handler::get),
        )
        .route(
            "/product-categories/:id",
            put(crate::handlers::product_category_handler::update),
        )
        .route(
            "/product-categories/:id",
            delete(crate::handlers::product_category_handler::delete),
        )
}

/// 供应商选择路由别名（path 前缀 /suppliers/select）
fn suppliers_select_alias() -> Router<AppState> {
    Router::new().route(
        "/suppliers/select",
        get(crate::handlers::supplier_handler::list_suppliers),
    )
}

/// P9-8 搜索 API 路由（path 前缀 /search）
fn search_routes() -> Router<AppState> {
    Router::new()
        .route("/search/sales-orders", get(search_api::search_sales_orders))
        .route("/search/customers", get(search_api::search_customers))
        .route("/search/products", get(search_api::search_products))
}

/// 构建 ERP 根域子路由（共享 `/api/v1/erp` 前缀）
///
/// 共享同一前缀的四个域（iam / catalog / analytics / system）必须先 merge 再整体 nest，
/// 否则连续 `.nest("/api/v1/erp", ...)` 会因后注册路由被前一个覆盖。
///
/// **重要**：返回类型必须显式标注 `Router<AppState>`，否则编译器会把类型
/// 锁定为 `Router<()>`，导致后续 nest 时类型不匹配。
fn build_erp_root_router() -> Router<AppState> {
    Router::<AppState>::new()
        .merge(iam::routes())
        .merge(catalog::routes())
        .merge(analytics::routes())
        .merge(system::routes())
        // 从 production 域提升的路由
        .merge(material_shortage_routes())
        .merge(scheduling_routes())
        .merge(quality_standards_routes())
        // 新增的路由
        .merge(user_profile_routes())
        .merge(system_update_extra_routes())
        .merge(print_templates_routes())
        .merge(data_import_routes())
        .merge(product_categories_alias())
        .merge(suppliers_select_alias())
        // P9-8 搜索 API
        .merge(search_routes())
}

/// 构建基础设施路由（静态资源 + 指标 + API 文档）
///
/// 与业务域不同，这三类基础设施不挂在 `/api/v1/erp` 之下，而是顶层独立路径：
/// - `/static/*` `/bingxi_frontend.*`
/// - `/metrics`
/// - `/swagger-ui` `/api-docs/openapi.json`
fn build_infrastructure_routes() -> Router<AppState> {
    let router = Router::<AppState>::new()
        // 静态资源（CSS / JS / WASM / 字体等）
        .merge(static_routes::static_assets_handler())
        // Prometheus 指标（/metrics）
        .merge(create_metrics_router());

    #[cfg(feature = "swagger")]
    let router = router.merge(
        SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", crate::docs::ApiDoc::openapi()),
    );

    router
}

/// 创建主路由
///
/// 将 14 个业务域子路由 + 监控 / 文档 / 静态资源拼装为统一入口。
/// 顶层只保留路径装配与 SQL 注入审计中间件挂载，所有具体路由定义下沉到子文件。
///
/// **重要**：返回 `Router<()>`，因为函数末尾通过 `with_state(state)` 把状态
/// 注入到所有内部子路由中；返回 `Router<()>` 是 axum 0.7 中 `axum::serve` 的
/// 唯一可接受类型（`Service<IncomingStream>` 只为 `Router<()>` 实现）。
pub fn create_router(state: AppState) -> Router<()> {
    Router::<AppState>::new()
        // ---- 14 个业务域（合并前缀 + 独立前缀）----
        .nest("/api/v1/erp", build_erp_root_router())
        .nest("/api/v1/erp/auth", auth::routes())
        .nest("/api/v1/erp/inventory", inventory::routes())
        .nest("/api/v1/erp/sales", sales::routes())
        // 销售报价单路由（Week 1）
        .nest("/api/v1/erp/quotations", quotations::routes())
        // 定制订单全流程跟踪路由（P0-3）
        // 修复：原 mod.rs 中 `pub mod custom_order;` 已声明但 create_router 未挂载，
        // 导致 /api/v1/erp/custom-orders 全部 404。补齐 nest 调用。
        .nest("/api/v1/erp/custom-orders", custom_order::routes())
        // 主备隔离路由（P0-2）
        .merge(failover::failover_routes())
        // 色卡仓储管理路由（P0-4）
        .nest("/api/v1/erp/color-cards", color_card::routes())
        // 面料多色号定价扩展路由（P0-5）
        .nest("/api/v1/erp/color-prices", color_price::routes())
        .nest("/api/v1/erp/purchase", purchase::routes())
        .nest("/api/v1/erp/finance", finance::routes(state.clone()))
        .nest("/api/v1/erp", finance::sub_routes())
        .nest("/api/v1/erp/production", production::routes())
        .nest("/api/v1/erp/crm", crm::routes())
        .nest("/api/v1/erp/tenants", tenant::routes())
        // v1 占位入口
        .nest("/api/v1", v1::routes())
        // ---- 基础设施（静态 / 指标 / API 文档）----
        .merge(build_infrastructure_routes())
        // ---- 中间件 ----
        // SQL 注入审计（命中危险模式直接 400，不进入 handler）
        // 注：security_headers 由 main.rs 的 SetResponseHeaderLayer 统一处理（含 CSP/HSTS/XFO/X-CTO/Referrer-Policy/XSS）
        .layer(middleware::from_fn(sql_injection_audit_middleware))
        // ---- 全局状态注入（必须最后调用）----
        .with_state(state)
}
