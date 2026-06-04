//! 生产域路由
//!
//! 处理生产订单、MRP 物料需求计划、生产排程、产能、缺料预警、质量检验/标准、成本归集、
//! 缸号/染色批次/染色配方、坯布等生产与工艺相关接口。

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

/// 缸号管理路由（nest 到 /api/v1/erp/dye-batches）
pub fn dye_batches() -> Router<AppState> {
    Router::new()
        .route("/", get(dye_batch_handler::list_dye_batches))
        .route("/", post(dye_batch_handler::create_dye_batch))
        .route("/:id", get(dye_batch_handler::get_dye_batch))
        .route("/:id", put(dye_batch_handler::update_dye_batch))
        .route("/:id", delete(dye_batch_handler::delete_dye_batch))
        .route("/:id/complete", post(dye_batch_handler::complete_dye_batch))
        .route(
            "/by-color/:color_code",
            get(dye_batch_handler::get_dye_batches_by_color),
        )
        .route("/export", get(dye_batch_handler::export_dye_batches))
}

/// 坯布管理路由（nest 到 /api/v1/erp/greige-fabrics）
pub fn greige_fabrics() -> Router<AppState> {
    Router::new()
        .route("/", get(greige_fabric_handler::list_greige_fabrics))
        .route("/", post(greige_fabric_handler::create_greige_fabric))
        .route("/:id", get(greige_fabric_handler::get_greige_fabric))
        .route("/:id", put(greige_fabric_handler::update_greige_fabric))
        .route("/:id", delete(greige_fabric_handler::delete_greige_fabric))
        .route("/:id/stock-in", post(greige_fabric_handler::stock_in))
        .route("/:id/stock-out", post(greige_fabric_handler::stock_out))
        .route(
            "/by-supplier/:supplier_id",
            get(greige_fabric_handler::get_greige_by_supplier),
        )
}

/// 染色配方路由（nest 到 /api/v1/erp/dye-recipes）
pub fn dye_recipes() -> Router<AppState> {
    Router::new()
        .route("/", get(dye_recipe_handler::list_dye_recipes))
        .route("/", post(dye_recipe_handler::create_dye_recipe))
        .route("/:id", get(dye_recipe_handler::get_dye_recipe))
        .route("/:id", put(dye_recipe_handler::update_dye_recipe))
        .route("/:id", delete(dye_recipe_handler::delete_dye_recipe))
        .route("/:id/approve", post(dye_recipe_handler::approve_recipe))
        .route("/:id/submit", post(dye_recipe_handler::submit_dye_recipe))
        .route("/:id/version", post(dye_recipe_handler::create_new_version))
        .route(
            "/by-color/:color_code",
            get(dye_recipe_handler::get_recipes_by_color),
        )
        .route(
            "/:id/versions",
            get(dye_recipe_handler::get_recipe_versions),
        )
        .route("/export", get(dye_recipe_handler::export_dye_recipes))
}

/// 质量检验路由（nest 到 /api/v1/erp/quality-inspection）
pub fn quality_inspection() -> Router<AppState> {
    Router::new()
        .route(
            "/standards",
            get(quality_inspection_handler::list_standards),
        )
        .route(
            "/standards",
            post(quality_inspection_handler::create_standard),
        )
        .route("/records", get(quality_inspection_handler::list_records))
        .route("/records", post(quality_inspection_handler::create_record))
        .route("/records/:id", get(quality_inspection_handler::get_record))
        .route("/defects", get(quality_inspection_handler::list_defects))
        .route(
            "/defects/:id/process",
            post(quality_inspection_handler::process_defect),
        )
        .route(
            "/defects/:id/handle",
            post(quality_inspection_handler::process_defect),
        )
}

/// 质量标准路由（nest 到 /api/v1/erp/quality-standards）
pub fn quality_standards() -> Router<AppState> {
    Router::new()
        .route("/", get(quality_standard_handler::list_standards))
        .route("/", post(quality_standard_handler::create_standard))
        .route("/:id", get(quality_standard_handler::get_standard))
        .route("/:id", put(quality_standard_handler::update_standard))
        .route("/:id", delete(quality_standard_handler::delete_standard))
        .route(
            "/:id/versions",
            get(quality_standard_handler::list_versions),
        )
        .route(
            "/:id/versions",
            post(quality_standard_handler::create_version_history),
        )
        .route(
            "/:id/approve",
            post(quality_standard_handler::approve_standard),
        )
        .route(
            "/:id/publish",
            post(quality_standard_handler::publish_standard),
        )
}

/// 成本归集路由（nest 到 /api/v1/erp/cost-collections）
pub fn cost_collections() -> Router<AppState> {
    Router::new()
        .route("/", get(cost_collection_handler::list_collections))
        .route("/", post(cost_collection_handler::create_collection))
        .route("/:id", get(cost_collection_handler::get_collection))
        .route("/:id", put(cost_collection_handler::update_collection))
        .route("/:id", delete(cost_collection_handler::delete_collection))
        .route(
            "/:id/audit",
            post(cost_collection_handler::audit_collection),
        )
        .route(
            "/analysis/summary",
            get(cost_collection_handler::get_cost_analysis_summary),
        )
        .route(
            "/analysis/by-batch",
            get(cost_collection_handler::get_cost_by_batch),
        )
}

/// 生产订单路由（nest 到 /api/v1/erp/production）
pub fn production() -> Router<AppState> {
    Router::new()
        .route(
            "/orders",
            get(production_order_handler::list_production_orders)
                .post(production_order_handler::create_production_order),
        )
        .route(
            "/orders/:id",
            get(production_order_handler::get_production_order)
                .put(production_order_handler::update_production_order)
                .delete(production_order_handler::delete_production_order),
        )
        .route(
            "/orders/:id/status",
            put(production_order_handler::update_production_order_status),
        )
        .route(
            "/orders/:id/submit-approval",
            post(production_order_handler::submit_for_approval),
        )
        .route(
            "/orders/:id/approve",
            post(production_order_handler::approve_production_order),
        )
        .route(
            "/orders/:id/progress",
            post(production_order_handler::update_production_progress),
        )
        .route(
            "/orders/:id/logs",
            get(production_order_handler::get_production_order_logs),
        )
}

/// MRP 物料需求计划路由（nest 到 /api/v1/erp/mrp）
pub fn mrp() -> Router<AppState> {
    Router::new()
        .route("/calculate", post(mrp_handler::calculate_mrp))
        .route("/results", get(mrp_handler::get_mrp_results))
        .route("/requirements", get(mrp_handler::get_mrp_requirements))
        .route("/convert-orders", post(mrp_handler::convert_to_orders))
        .route("/products", get(mrp_handler::list_products_for_mrp))
}

/// MRP 历史记录路由（nest 到 /api/v1/erp/mrp/history）
pub fn mrp_history() -> Router<AppState> {
    Router::new()
        .route("/", get(missing_handlers::get_mrp_history))
        .route("/:id", get(missing_handlers::get_mrp_history_detail))
        .route("/:id/cancel", put(mrp_handler::cancel_calculation))
        .route("/:id/export", get(mrp_handler::export_calculation))
        .route(
            "/:calculation_id/materials/:material_id",
            get(mrp_handler::get_material_detail),
        )
}

/// 生产排程路由（nest 到 /api/v1/erp/scheduling）
pub fn scheduling() -> Router<AppState> {
    Router::new()
        .route("/auto-schedule", post(scheduling_handler::auto_schedule))
        .route("/gantt", get(scheduling_handler::get_gantt_data))
        .route("/conflicts", get(scheduling_handler::detect_conflicts))
        .route("/tasks", get(scheduling_handler::list_scheduled_orders))
        .route(
            "/tasks/:id/adjust",
            put(scheduling_handler::adjust_schedule_task),
        )
        .route("/:id", put(scheduling_handler::adjust_schedule))
        .route(
            "/work-orders",
            get(scheduling_handler::list_scheduled_orders),
        )
        .route("/history", get(scheduling_handler::get_schedule_history))
        .route("/results/:id", get(scheduling_handler::get_schedule_result))
        .route(
            "/results/:id/confirm",
            post(scheduling_handler::confirm_schedule_result),
        )
}

/// 产能分析路由（nest 到 /api/v1/erp/capacity）
pub fn capacity() -> Router<AppState> {
    Router::new()
        .route("/overview", get(capacity_handler::get_capacity_overview))
        .route("/summary", get(capacity_handler::get_capacity_overview))
        .route("/bottlenecks", get(capacity_handler::get_load_analysis))
        .route("/trend", get(capacity_handler::get_load_analysis))
        .route(
            "/work-centers",
            get(capacity_handler::list_work_centers).post(capacity_handler::create_work_center),
        )
        .route(
            "/work-centers/:id",
            put(capacity_handler::update_work_center).delete(capacity_handler::delete_work_center),
        )
        .route(
            "/work-centers/:id/forecast",
            get(capacity_handler::forecast_capacity),
        )
        .route(
            "/work-centers/:id/available",
            get(capacity_handler::get_available_capacity),
        )
        .route("/load-analysis", get(capacity_handler::get_load_analysis))
        .route(
            "/overload-check",
            get(capacity_handler::check_capacity_overload),
        )
}

/// 缺料预警路由（nest 到 /api/v1/erp/material-shortage）
pub fn material_shortage() -> Router<AppState> {
    Router::new()
        .route(
            "/alerts",
            get(material_shortage_handler::list_shortage_alerts),
        )
        .route(
            "/list",
            get(material_shortage_handler::list_shortage_alerts),
        )
        .route(
            "/check",
            post(material_shortage_handler::check_material_shortage),
        )
        .route(
            "/summary",
            get(material_shortage_handler::get_shortage_summary),
        )
        .route(
            "/threshold",
            get(material_shortage_handler::get_threshold_config)
                .post(material_shortage_handler::save_threshold_config),
        )
        .route(
            "/replenishment",
            get(material_shortage_handler::get_replenishment_suggestions),
        )
        .route(
            "/:id/status",
            put(material_shortage_handler::update_shortage_status),
        )
}

/// 生产域统一入口
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
