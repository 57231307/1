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
    ai_extend_handler, bpm_definition_handler, bpm_handler, dashboard_handler, health_handler,
    init_handler, system_update_handler,
};
use crate::websocket;

/// WebSocket 路由（path 前缀 /ws）
///
/// 关键路径：通知模块 WebSocket
/// - `/ws/notifications`：通知实时推送（鉴权通过 URL query token）
pub fn ws() -> Router<AppState> {
    Router::new()
        .route(
            "/ws/notifications",
            // 修复：原 `websocket::ws_notifications_handler` 启动时 panic（不存在），
            // 实际路径为 `websocket::notifications::ws_notifications_handler`
            // （backend/src/websocket/mod.rs:6 已声明 `pub mod notifications;`）
            get(websocket::notifications::ws_notifications_handler),
        )
}

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

/// 审计日志查询路由（path 前缀 /audit-logs）
///
/// P13 批 1 P3-2 增强版：
/// - GET /audit-logs          列表（分页 + 多维筛选）
/// - GET /audit-logs/:id      详情
/// - GET /audit-logs/export   CSV 导出
pub fn audit_logs() -> Router<AppState> {
    use crate::handlers::audit_log_handler;
    Router::new()
        .route("/audit-logs", get(audit_log_handler::list_audit_logs))
        .route(
            "/audit-logs/export",
            get(audit_log_handler::export_audit_logs),
        )
        .route("/audit-logs/:id", get(audit_log_handler::get_audit_log))
}

/// 慢查询审计路由（path 前缀 /slow-queries）
///
/// P13 批 1 B-慢查询审计：
/// - GET    /slow-queries          列表（分页 + 多维筛选）
/// - GET    /slow-queries/stats    TOP 10 聚合统计
/// - POST   /slow-queries/refresh  手动触发一次采集
pub fn slow_queries() -> Router<AppState> {
    use crate::handlers::slow_query_handler;
    Router::new()
        .route("/slow-queries", get(slow_query_handler::list_slow_queries))
        .route(
            "/slow-queries/stats",
            get(slow_query_handler::get_slow_query_stats),
        )
        .route(
            "/slow-queries/refresh",
            axum::routing::post(slow_query_handler::refresh_slow_queries),
        )
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

/// AI 分析深化路由（path 前缀 /ai）— P2-4
///
/// 16 个端点：
/// - 工艺优化 5（创建/列表/详情/应用反馈/删除）+ 1（按色号+布类历史）+ 1（批量）= 7
/// - 质量预测 5（创建/列表/详情/确认/删除）+ 1（按产品历史）+ 1（批量）= 7
/// - 看板 / 健康检查 = 2
pub fn ai() -> Router<AppState> {
    Router::new()
        // 工艺优化
        .route(
            "/ai/process-optimizations",
            get(ai_extend_handler::list_process_optimizations)
                .post(ai_extend_handler::create_process_optimization),
        )
        .route(
            "/ai/process-optimizations/batch",
            post(ai_extend_handler::batch_create_process_optimizations),
        )
        .route(
            "/ai/process-optimizations/by-color",
            get(ai_extend_handler::list_process_optimizations_by_color),
        )
        .route(
            "/ai/process-optimizations/:id",
            get(ai_extend_handler::get_process_optimization)
                .delete(ai_extend_handler::delete_process_optimization),
        )
        .route(
            "/ai/process-optimizations/:id/apply",
            post(ai_extend_handler::apply_process_optimization),
        )
        // 质量预测
        .route(
            "/ai/quality-predictions",
            get(ai_extend_handler::list_quality_predictions)
                .post(ai_extend_handler::create_quality_prediction),
        )
        .route(
            "/ai/quality-predictions/batch",
            post(ai_extend_handler::batch_create_quality_predictions),
        )
        .route(
            "/ai/quality-predictions/by-product",
            get(ai_extend_handler::list_quality_predictions_by_product),
        )
        .route(
            "/ai/quality-predictions/:id",
            get(ai_extend_handler::get_quality_prediction)
                .delete(ai_extend_handler::delete_quality_prediction),
        )
        .route(
            "/ai/quality-predictions/:id/acknowledge",
            post(ai_extend_handler::acknowledge_quality_prediction),
        )
        // 看板 / 健康检查
        .route("/ai/summary", get(ai_extend_handler::ai_summary))
        .route("/ai/health", get(ai_extend_handler::ai_health))
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
        .merge(ai())
        .merge(ws())
}
