//! 系统域路由
//!
//! 处理仪表板、系统更新、BPM 工作流引擎、健康检查、初始化等系统级接口。

use crate::utils::app_state::AppState;
use axum::{
    routing::{get, post},
    Router,
};

use crate::handlers::{
    bpm_definition_handler, bpm_handler, dashboard_handler, health_handler, init_handler,
    system_update_handler,
};

/// 仪表板路由（nest 到 /api/v1/erp/dashboard）
pub fn dashboard() -> Router<AppState> {
    Router::new()
        .route("/overview", get(dashboard_handler::get_dashboard_overview))
        .route("/sales-stats", get(dashboard_handler::get_sales_statistics))
        .route(
            "/inventory-stats",
            get(dashboard_handler::get_inventory_statistics),
        )
        .route(
            "/low-stock-alerts",
            get(dashboard_handler::get_low_stock_alerts),
        )
}

/// 系统更新路由（nest 到 /api/v1/erp/system-update）
pub fn system_update() -> Router<AppState> {
    Router::new()
        .route("/check", get(system_update_handler::check_for_updates))
        .route("/update", post(system_update_handler::download_and_update))
        .route("/version", get(system_update_handler::get_version))
        .route("/status", get(system_update_handler::get_update_status))
        .route("/versions", get(system_update_handler::get_backup_versions))
        .route("/rollback", post(system_update_handler::rollback_version))
        .route(
            "/local-releases",
            get(system_update_handler::list_local_releases),
        )
        .route(
            "/local-update",
            post(system_update_handler::apply_local_update),
        )
        .route(
            "/local-check",
            get(system_update_handler::check_for_local_updates),
        )
        .route("/upload", post(system_update_handler::upload_and_update))
}

/// BPM 工作流引擎路由（nest 到 /api/v1/erp/bpm）
pub fn bpm() -> Router<AppState> {
    Router::new()
        .route("/process/start", post(bpm_handler::start_process))
        .route("/tasks/approve", post(bpm_handler::approve_task))
        .route("/tasks", get(bpm_handler::query_tasks))
        .route("/tasks/pending", get(bpm_handler::get_pending_tasks))
        .route("/tasks/completed", get(bpm_handler::get_completed_tasks))
        .route(
            "/business-relation",
            get(bpm_handler::get_business_relation),
        )
        .route(
            "/visualization/:instance_id",
            get(bpm_handler::get_process_visualization),
        )
        .route(
            "/instances/:instance_id/approval-chain",
            get(bpm_handler::get_approval_chain),
        )
        .route(
            "/instances/:instance_id/chain",
            get(bpm_handler::get_approval_chain),
        )
        .route(
            "/instances/:instance_id/detail",
            get(bpm_handler::get_instance_detail),
        )
        .route("/monitor/stats", get(bpm_handler::get_monitor_stats))
        .route(
            "/monitor/pending-tasks",
            get(bpm_handler::get_pending_tasks_for_monitor),
        )
        .route(
            "/monitor/instances",
            get(bpm_handler::list_instances_for_monitor),
        )
        .route("/tasks/:task_id/transfer", post(bpm_handler::transfer_task))
        .route("/tasks/:task_id/urge", post(bpm_handler::urge_task))
        .route("/approval/execute", post(bpm_handler::execute_approval))
        .route(
            "/definitions",
            get(bpm_definition_handler::list_process_definitions)
                .post(bpm_definition_handler::create_process_definition),
        )
        .route(
            "/definitions/:id",
            get(bpm_definition_handler::get_process_definition)
                .put(bpm_definition_handler::update_process_definition)
                .delete(bpm_definition_handler::delete_process_definition),
        )
        .route(
            "/definitions/:id/versions",
            get(bpm_definition_handler::list_versions).post(bpm_definition_handler::create_version),
        )
        .route(
            "/definitions/:id/versions/:version/activate",
            post(bpm_definition_handler::activate_version),
        )
        .route(
            "/versions/:version/activate",
            post(bpm_definition_handler::activate_version_by_id),
        )
        .route("/templates", get(bpm_definition_handler::list_templates))
        .route(
            "/templates/:template_id",
            get(bpm_definition_handler::list_templates)
                .delete(bpm_definition_handler::delete_process_definition),
        )
        .route(
            "/templates/:template_id/create",
            post(bpm_definition_handler::create_from_template),
        )
}

/// 健康检查路由（nest 到 /api/v1/erp/health）
pub fn health() -> Router<AppState> {
    Router::new()
        .route("/", get(health_handler::health_check))
        .route("/readiness", get(health_handler::readiness_check))
        .route("/liveness", get(health_handler::liveness_check))
}

/// 初始化路由（nest 到 /api/v1/erp/init）
pub fn init() -> Router<AppState> {
    Router::new()
        .route("/status", get(init_handler::get_init_status))
        .route(
            "/test-database",
            post(init_handler::test_database_connection),
        )
        .route("/initialize", post(init_handler::initialize_system))
        .route(
            "/initialize-with-db",
            post(init_handler::initialize_system_with_db),
        )
}

/// 系统域统一入口
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(dashboard())
        .merge(system_update())
        .merge(bpm())
        .merge(health())
        .merge(init())
}
