//! 系统域路由
//!
//! 处理仪表板、系统更新、BPM 工作流引擎、健康检查、初始化等系统级接口。
//!
//! 路由设计说明：所有子 router 内部 path 都已加上各自独立前缀
//!（`/dashboard`、`/system-update`、`/bpm`、`/health`、`/init`），
//! 同时规避同前缀下的 path+method 重叠（`system_update` 的 `/status` 与
//! `init` 的 `/status` 不冲突），避免 axum 0.7 `Overlapping method route` panic。

use crate::utils::app_state::AppState;
use axum::{
    routing::{get, post},
    Router,
};

use crate::handlers::{
    bpm_definition_handler, bpm_handler, dashboard_handler, health_handler, init_handler,
    system_update_handler,
};

/// 仪表板路由（path 前缀 /dashboard）
pub fn dashboard() -> Router<AppState> {
    Router::new()
        .route(
            "/dashboard/overview",
            get(dashboard_handler::get_dashboard_overview),
        )
        .route(
            "/dashboard/sales-stats",
            get(dashboard_handler::get_sales_statistics),
        )
        .route(
            "/dashboard/inventory-stats",
            get(dashboard_handler::get_inventory_statistics),
        )
        .route(
            "/dashboard/low-stock-alerts",
            get(dashboard_handler::get_low_stock_alerts),
        )
}

/// 系统更新路由（path 前缀 /system-update）
pub fn system_update() -> Router<AppState> {
    Router::new()
        .route(
            "/system-update/check",
            get(system_update_handler::check_for_updates),
        )
        .route(
            "/system-update/update",
            post(system_update_handler::download_and_update),
        )
        .route(
            "/system-update/version",
            get(system_update_handler::get_version),
        )
        // 注意：原 `/status` 与 init() 的 `/status` 冲突，已重命名为 `/update-status`
        .route(
            "/system-update/update-status",
            get(system_update_handler::get_update_status),
        )
        .route(
            "/system-update/versions",
            get(system_update_handler::get_backup_versions),
        )
        .route(
            "/system-update/rollback",
            post(system_update_handler::rollback_version),
        )
        .route(
            "/system-update/local-releases",
            get(system_update_handler::list_local_releases),
        )
        .route(
            "/system-update/local-update",
            post(system_update_handler::apply_local_update),
        )
        .route(
            "/system-update/local-check",
            get(system_update_handler::check_for_local_updates),
        )
        .route(
            "/system-update/upload",
            post(system_update_handler::upload_and_update),
        )
}

/// BPM 工作流引擎路由（path 前缀 /bpm）
///
/// 注意：内部 path 已加 `/process`、`/tasks`、`/instances` 等子前缀，
/// 避免与 system_update() 的 `/status` 冲突；同时也避免与 health()、init() 同前缀。
pub fn bpm() -> Router<AppState> {
    Router::new()
        .route("/bpm/process/start", post(bpm_handler::start_process))
        .route("/bpm/tasks/approve", post(bpm_handler::approve_task))
        .route("/bpm/tasks", get(bpm_handler::query_tasks))
        .route("/bpm/tasks/pending", get(bpm_handler::get_pending_tasks))
        .route(
            "/bpm/tasks/completed",
            get(bpm_handler::get_completed_tasks),
        )
        .route(
            "/bpm/business-relation",
            get(bpm_handler::get_business_relation),
        )
        .route(
            "/bpm/visualization/:instance_id",
            get(bpm_handler::get_process_visualization),
        )
        .route(
            "/bpm/instances/:instance_id/approval-chain",
            get(bpm_handler::get_approval_chain),
        )
        .route(
            "/bpm/instances/:instance_id/chain",
            get(bpm_handler::get_approval_chain),
        )
        .route(
            "/bpm/instances/:instance_id/detail",
            get(bpm_handler::get_instance_detail),
        )
        .route("/bpm/monitor/stats", get(bpm_handler::get_monitor_stats))
        .route(
            "/bpm/monitor/pending-tasks",
            get(bpm_handler::get_pending_tasks_for_monitor),
        )
        .route(
            "/bpm/monitor/instances",
            get(bpm_handler::list_instances_for_monitor),
        )
        .route(
            "/bpm/tasks/:task_id/transfer",
            post(bpm_handler::transfer_task),
        )
        .route("/bpm/tasks/:task_id/urge", post(bpm_handler::urge_task))
        .route("/bpm/approval/execute", post(bpm_handler::execute_approval))
        .route(
            "/bpm/definitions",
            get(bpm_definition_handler::list_process_definitions)
                .post(bpm_definition_handler::create_process_definition),
        )
        .route(
            "/bpm/definitions/:id",
            get(bpm_definition_handler::get_process_definition)
                .put(bpm_definition_handler::update_process_definition)
                .delete(bpm_definition_handler::delete_process_definition),
        )
        .route(
            "/bpm/definitions/:id/versions",
            get(bpm_definition_handler::list_versions).post(bpm_definition_handler::create_version),
        )
        .route(
            "/bpm/definitions/:id/versions/:version/activate",
            post(bpm_definition_handler::activate_version),
        )
        .route(
            "/bpm/versions/:version/activate",
            post(bpm_definition_handler::activate_version_by_id),
        )
        .route(
            "/bpm/templates",
            get(bpm_definition_handler::list_templates),
        )
        .route(
            "/bpm/templates/:template_id",
            get(bpm_definition_handler::list_templates)
                .delete(bpm_definition_handler::delete_process_definition),
        )
        .route(
            "/bpm/templates/:template_id/create",
            post(bpm_definition_handler::create_from_template),
        )
}

/// 健康检查路由（path 前缀 /health）
pub fn health() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_handler::health_check))
        .route("/health/readiness", get(health_handler::readiness_check))
        .route("/health/liveness", get(health_handler::liveness_check))
}

/// 初始化路由（path 前缀 /init）
pub fn init() -> Router<AppState> {
    Router::new()
        .route("/init/status", get(init_handler::get_init_status))
        .route(
            "/init/test-database",
            post(init_handler::test_database_connection),
        )
        .route("/init/initialize", post(init_handler::initialize_system))
        .route(
            "/init/initialize-with-db",
            post(init_handler::initialize_system_with_db),
        )
        .route(
            "/init/initialize-with-db-async",
            post(init_handler::initialize_system_with_db_async),
        )
        .route("/init/task-status", get(init_handler::get_task_status))
}

/// 系统域统一入口
///
/// 子 router path 已加独立前缀，merge 时 path+method 互不重叠。
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(dashboard())
        .merge(system_update())
        .merge(bpm())
        .merge(health())
        .merge(init())
}
