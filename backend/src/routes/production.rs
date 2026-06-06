//! 生产域路由
//!
//! 处理生产订单、MRP 物料需求计划、生产排程、产能、缺料预警、质量检验/标准、成本归集、
//! 缸号/染色批次/染色配方、坯布等生产与工艺相关接口。
//!
//! 路由设计说明：所有子 router 内部 path 都已加上各自独立前缀
//!（`/dye-batches`、`/greige-fabrics`、`/dye-recipes`、`/quality-inspection`、
//!  `/quality-standards`、`/cost-collections`、`/production-orders`、`/mrp`、
//!  `/mrp-history`、`/scheduling`、`/capacity`、`/material-shortage` 等），
//!  这样 `routes()` 入口用 `merge` 组合时不会出现 path+method 重叠，
//!  避免 axum 0.7 `Overlapping method route` panic。

use crate::utils::app_state::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::{
    capacity_handler, cost_collection_handler, dye_batch_handler, dye_recipe_handler,
    greige_fabric_handler, material_shortage_handler, missing_handlers, mrp_handler,
    production_order_handler, quality_inspection_handler, quality_standard_handler,
    scheduling_handler,
};

/// 缸号管理路由（path 前缀 /dye-batches）
pub fn dye_batches() -> Router<AppState> {
    Router::new()
        .route("/dye-batches", get(dye_batch_handler::list_dye_batches))
        .route("/dye-batches", post(dye_batch_handler::create_dye_batch))
        .route("/dye-batches/:id", get(dye_batch_handler::get_dye_batch))
        .route("/dye-batches/:id", put(dye_batch_handler::update_dye_batch))
        .route(
            "/dye-batches/:id",
            delete(dye_batch_handler::delete_dye_batch),
        )
        .route(
            "/dye-batches/:id/complete",
            post(dye_batch_handler::complete_dye_batch),
        )
        .route(
            "/dye-batches/by-color/:color_code",
            get(dye_batch_handler::get_dye_batches_by_color),
        )
        .route(
            "/dye-batches/export",
            get(dye_batch_handler::export_dye_batches),
        )
}

/// 坯布管理路由（path 前缀 /greige-fabrics）
pub fn greige_fabrics() -> Router<AppState> {
    Router::new()
        .route(
            "/greige-fabrics",
            get(greige_fabric_handler::list_greige_fabrics),
        )
        .route(
            "/greige-fabrics",
            post(greige_fabric_handler::create_greige_fabric),
        )
        .route(
            "/greige-fabrics/:id",
            get(greige_fabric_handler::get_greige_fabric),
        )
        .route(
            "/greige-fabrics/:id",
            put(greige_fabric_handler::update_greige_fabric),
        )
        .route(
            "/greige-fabrics/:id",
            delete(greige_fabric_handler::delete_greige_fabric),
        )
        .route(
            "/greige-fabrics/:id/stock-in",
            post(greige_fabric_handler::stock_in),
        )
        .route(
            "/greige-fabrics/:id/stock-out",
            post(greige_fabric_handler::stock_out),
        )
        .route(
            "/greige-fabrics/by-supplier/:supplier_id",
            get(greige_fabric_handler::get_greige_by_supplier),
        )
}

/// 染色配方路由（path 前缀 /dye-recipes）
pub fn dye_recipes() -> Router<AppState> {
    Router::new()
        .route("/dye-recipes", get(dye_recipe_handler::list_dye_recipes))
        .route("/dye-recipes", post(dye_recipe_handler::create_dye_recipe))
        .route("/dye-recipes/:id", get(dye_recipe_handler::get_dye_recipe))
        .route(
            "/dye-recipes/:id",
            put(dye_recipe_handler::update_dye_recipe),
        )
        .route(
            "/dye-recipes/:id",
            delete(dye_recipe_handler::delete_dye_recipe),
        )
        .route(
            "/dye-recipes/:id/approve",
            post(dye_recipe_handler::approve_recipe),
        )
        .route(
            "/dye-recipes/:id/submit",
            post(dye_recipe_handler::submit_dye_recipe),
        )
        .route(
            "/dye-recipes/:id/version",
            post(dye_recipe_handler::create_new_version),
        )
        .route(
            "/dye-recipes/by-color/:color_code",
            get(dye_recipe_handler::get_recipes_by_color),
        )
        .route(
            "/dye-recipes/:id/versions",
            get(dye_recipe_handler::get_recipe_versions),
        )
        .route(
            "/dye-recipes/export",
            get(dye_recipe_handler::export_dye_recipes),
        )
}

/// 质量检验路由（path 前缀 /quality-inspection）
///
/// 注意：原代码用 `/standards`、`/records`、`/defects` 等带前缀 path，已天然不冲突。
pub fn quality_inspection() -> Router<AppState> {
    Router::new()
        .route(
            "/quality-inspection/standards",
            get(quality_inspection_handler::list_standards),
        )
        .route(
            "/quality-inspection/standards",
            post(quality_inspection_handler::create_standard),
        )
        .route(
            "/quality-inspection/records",
            get(quality_inspection_handler::list_records),
        )
        .route(
            "/quality-inspection/records",
            post(quality_inspection_handler::create_record),
        )
        .route(
            "/quality-inspection/records/:id",
            get(quality_inspection_handler::get_record),
        )
        .route(
            "/quality-inspection/defects",
            get(quality_inspection_handler::list_defects),
        )
        .route(
            "/quality-inspection/defects/:id/process",
            post(quality_inspection_handler::process_defect),
        )
        .route(
            "/quality-inspection/defects/:id/handle",
            post(quality_inspection_handler::process_defect),
        )
}

/// 质量标准路由（path 前缀 /quality-standards）
pub fn quality_standards() -> Router<AppState> {
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

/// 成本归集路由（path 前缀 /cost-collections）
pub fn cost_collections() -> Router<AppState> {
    Router::new()
        .route(
            "/cost-collections",
            get(cost_collection_handler::list_collections),
        )
        .route(
            "/cost-collections",
            post(cost_collection_handler::create_collection),
        )
        .route(
            "/cost-collections/:id",
            get(cost_collection_handler::get_collection),
        )
        .route(
            "/cost-collections/:id",
            put(cost_collection_handler::update_collection),
        )
        .route(
            "/cost-collections/:id",
            delete(cost_collection_handler::delete_collection),
        )
        .route(
            "/cost-collections/:id/audit",
            post(cost_collection_handler::audit_collection),
        )
        .route(
            "/cost-collections/analysis/summary",
            get(cost_collection_handler::get_cost_analysis_summary),
        )
        .route(
            "/cost-collections/analysis/by-batch",
            get(cost_collection_handler::get_cost_by_batch),
        )
}

/// 生产订单路由（path 前缀 /production-orders）
pub fn production() -> Router<AppState> {
    Router::new()
        .route(
            "/production-orders/orders",
            get(production_order_handler::list_production_orders)
                .post(production_order_handler::create_production_order),
        )
        .route(
            "/production-orders/orders/:id",
            get(production_order_handler::get_production_order)
                .put(production_order_handler::update_production_order)
                .delete(production_order_handler::delete_production_order),
        )
        .route(
            "/production-orders/orders/:id/status",
            put(production_order_handler::update_production_order_status),
        )
        .route(
            "/production-orders/orders/:id/submit-approval",
            post(production_order_handler::submit_for_approval),
        )
        .route(
            "/production-orders/orders/:id/approve",
            post(production_order_handler::approve_production_order),
        )
        .route(
            "/production-orders/orders/:id/progress",
            post(production_order_handler::update_production_progress),
        )
        .route(
            "/production-orders/orders/:id/logs",
            get(production_order_handler::get_production_order_logs),
        )
}

/// MRP 物料需求计划路由（path 前缀 /mrp）
pub fn mrp() -> Router<AppState> {
    Router::new()
        .route("/mrp/calculate", post(mrp_handler::calculate_mrp))
        .route("/mrp/results", get(mrp_handler::get_mrp_results))
        .route("/mrp/requirements", get(mrp_handler::get_mrp_requirements))
        .route("/mrp/convert-orders", post(mrp_handler::convert_to_orders))
        .route("/mrp/products", get(mrp_handler::list_products_for_mrp))
}

/// MRP 历史记录路由（path 前缀 /mrp-history）
pub fn mrp_history() -> Router<AppState> {
    Router::new()
        .route("/mrp-history", get(missing_handlers::get_mrp_history))
        .route(
            "/mrp-history/:id",
            get(missing_handlers::get_mrp_history_detail),
        )
        .route(
            "/mrp-history/:id/cancel",
            put(mrp_handler::cancel_calculation),
        )
        .route(
            "/mrp-history/:id/export",
            get(mrp_handler::export_calculation),
        )
        .route(
            "/mrp-history/:calculation_id/materials/:material_id",
            get(mrp_handler::get_material_detail),
        )
}

/// 生产排程路由（path 前缀 /scheduling）
pub fn scheduling() -> Router<AppState> {
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

/// 产能分析路由（path 前缀 /capacity）
pub fn capacity() -> Router<AppState> {
    Router::new()
        .route(
            "/capacity/overview",
            get(capacity_handler::get_capacity_overview),
        )
        .route(
            "/capacity/summary",
            get(capacity_handler::get_capacity_overview),
        )
        .route(
            "/capacity/bottlenecks",
            get(capacity_handler::get_load_analysis),
        )
        .route("/capacity/trend", get(capacity_handler::get_load_analysis))
        .route(
            "/capacity/work-centers",
            get(capacity_handler::list_work_centers).post(capacity_handler::create_work_center),
        )
        .route(
            "/capacity/work-centers/:id",
            put(capacity_handler::update_work_center).delete(capacity_handler::delete_work_center),
        )
        .route(
            "/capacity/work-centers/:id/forecast",
            get(capacity_handler::forecast_capacity),
        )
        .route(
            "/capacity/work-centers/:id/available",
            get(capacity_handler::get_available_capacity),
        )
        .route(
            "/capacity/load-analysis",
            get(capacity_handler::get_load_analysis),
        )
        .route(
            "/capacity/overload-check",
            get(capacity_handler::check_capacity_overload),
        )
}

/// 缺料预警路由（path 前缀 /material-shortage）
pub fn material_shortage() -> Router<AppState> {
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

/// 生产域统一入口
///
/// 子 router path 已加独立前缀，merge 时 path+method 互不重叠。
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(dye_batches())
        .merge(greige_fabrics())
        .merge(dye_recipes())
        .merge(quality_inspection())
        .merge(quality_standards())
        .merge(cost_collections())
        .merge(production())
        .merge(mrp())
        .merge(mrp_history())
        .merge(scheduling())
        .merge(capacity())
        .merge(material_shortage())
}
