//! 系统域路由
//!
//! 处理仪表板、系统更新、BPM 工作流引擎、健康检查、初始化等系统级接口。
//!
//! 路由设计说明：所有子 router 内部 path 都已加上各自独立前缀
//!（`/dashboard`、`/system-update`、`/bpm`、`/health`、`/init`），
//! 同时规避同前缀下的 path+method 重叠（`system_update` 的 `/status` 与
//! `init` 的 `/status` 不冲突），避免 axum 0.7 `Overlapping method route` panic。

use crate::container::AppState;
use crate::middleware::init_token::init_token_middleware;
use crate::middleware::rate_limit::rate_limit_ai_endpoint;
use axum::{
    middleware,
    routing::{get, post},
    Router,
};

use crate::handlers::{
    ai_extend_handler, bpm_definition_handler, bpm_handler, dashboard_handler,
    export_approval_handler, health_handler, init_handler, system_update_handler,
};
use crate::websocket;

/// WebSocket 路由（/ws/ticket 签发票据 + /ws/notifications 通知推送）
pub fn ws() -> Router<AppState> {
    Router::new()
        .route(
            "/ws/notifications",
            get(websocket::notifications::ws_notifications_handler),
        )
        .route(
            "/ws/ticket",
            post(websocket::notifications::issue_ws_ticket_handler),
        )
}

/// 仪表板路由（path 前缀 /dashboard）；缺陷 4.1 修复：新增 `/dashboard/layout` GET/PUT 路由， 支持用户自定义卡片配置（顺序/可见性/尺寸）并持久化到 dashboard_layouts 表。 缺陷 4.2 修复：overview 端点返回数据后通过
/// WebSocket 广播 dashboard_update 事件， 前端订阅 ws 频道实时刷新卡片，绕过 5 分钟缓存延迟。 缺陷 4.3 修复：overview/sales-stats 端点使用 new_with_data_scope 注入角色数据范围， 普通员工仅看到自己订单数据，财务卡片仅财务角色可见。
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
        // 缺陷 4.1 修复：用户自定义仪表板卡片布局持久化
        .route(
            "/dashboard/layout",
            get(dashboard_handler::get_dashboard_layout)
                .put(dashboard_handler::save_dashboard_layout),
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

/// BPM 流程与任务路由（/bpm/process、/bpm/tasks、/bpm/business-relation、/bpm/visualization）
fn bpm_process_task_routes() -> Router<AppState> {
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
            "/bpm/tasks/:task_id/transfer",
            post(bpm_handler::transfer_task),
        )
        .route("/bpm/tasks/:task_id/urge", post(bpm_handler::urge_task))
        .route("/bpm/approval/execute", post(bpm_handler::execute_approval))
}

/// BPM 流程实例路由（/bpm/instances）
fn bpm_instance_routes() -> Router<AppState> {
    Router::new()
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
        // 批次 157d-3 新增：撤回流程实例
        .route(
            "/bpm/instances/:instance_id/cancel",
            post(bpm_handler::cancel_instance),
        )
}

/// BPM 监控路由（/bpm/monitor）
fn bpm_monitor_routes() -> Router<AppState> {
    Router::new()
        .route("/bpm/monitor/stats", get(bpm_handler::get_monitor_stats))
        .route(
            "/bpm/monitor/pending-tasks",
            get(bpm_handler::get_pending_tasks_for_monitor),
        )
        .route(
            "/bpm/monitor/instances",
            get(bpm_handler::list_instances_for_monitor),
        )
}

/// BPM 流程定义/版本/模板路由（/bpm/definitions、/bpm/versions、/bpm/templates）
fn bpm_definition_routes() -> Router<AppState> {
    Router::new()
        // 批次 67（P1 1-2 修复）：BPM 流程定义/版本/模板管理路由
        // 原 stub 占位未注册，现 service 层已实现真实逻辑
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
            get(bpm_definition_handler::list_versions)
                .post(bpm_definition_handler::create_version),
        )
        .route(
            "/bpm/definitions/:id/versions/:version/activate",
            post(bpm_definition_handler::activate_version),
        )
        .route(
            "/bpm/definitions/:id/template",
            post(bpm_definition_handler::save_as_template),
        )
        .route(
            "/bpm/versions/:version_id/activate",
            post(bpm_definition_handler::activate_version_by_id),
        )
        .route(
            "/bpm/templates",
            get(bpm_definition_handler::list_templates),
        )
        .route(
            "/bpm/templates/:template_id/create",
            post(bpm_definition_handler::create_from_template),
        )
}

/// BPM 工作流引擎路由（合并流程任务/实例/监控/定义版本模板）
pub fn bpm() -> Router<AppState> {
    Router::new()
        .merge(bpm_process_task_routes())
        .merge(bpm_instance_routes())
        .merge(bpm_monitor_routes())
        .merge(bpm_definition_routes())
}

/// 健康检查路由（path 前缀 /health）
pub fn health() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_handler::health_check))
        .route("/health/readiness", get(health_handler::readiness_check))
        .route("/health/liveness", get(health_handler::liveness_check))
}

/// 审计日志查询路由（/audit-logs：列表/详情/xlsx 导出/前端打印埋点/导出二次审计记录）
pub fn audit_logs() -> Router<AppState> {
    use crate::handlers::audit_log_handler;
    use axum::routing::post;
    Router::new()
        .route("/audit-logs", get(audit_log_handler::list_audit_logs))
        .route(
            "/audit-logs/export",
            get(audit_log_handler::export_audit_logs),
        )
        .route("/audit-logs/:id", get(audit_log_handler::get_audit_log))
        // V15 P1-5-3：前端打印审计埋点端点（POST，已认证用户均可上报）
        .route(
            "/audit-logs/record-print",
            post(audit_log_handler::record_print_event),
        )
        // V15 缺陷 10-4：审计日志导出二次审计记录查询（仅 admin/auditor）
        .route(
            "/audit-logs/export-logs",
            get(audit_log_handler::list_audit_log_export_logs),
        )
}

/// 慢查询审计路由（/slow-queries：列表/统计/手动采集/优化状态更新）
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
        // V15 P2 20.5-C：慢查询优化任务追踪端点
        .route(
            "/slow-queries/:id/optimization",
            axum::routing::put(slow_query_handler::update_slow_query_optimization),
        )
        // batch-17 P3：慢查询周报端点
        .route(
            "/slow-queries/report/weekly",
            get(slow_query_handler::get_weekly_report),
        )
}

/// 初始化路由（/init，高危接口需 X-Init-Token 头校验）
pub fn init() -> Router<AppState> {
    // 高危初始化接口子路由：必须通过 INIT_TOKEN 校验
    let protected = Router::new()
        .route("/init/initialize", post(init_handler::initialize_system))
        .route(
            "/init/initialize-with-db",
            post(init_handler::initialize_system_with_db),
        )
        .route(
            "/init/initialize-with-db-async",
            post(init_handler::initialize_system_with_db_async),
        )
        .layer(middleware::from_fn(init_token_middleware));

    // 只读/受限接口：不应用 init token 校验
    // - /status：无副作用，仅返回初始化状态，公开访问合理
    // - /test-database 与 /task-status：handler 内已有 admin 角色二次校验
    let public = Router::new()
        .route("/init/status", get(init_handler::get_init_status))
        .route(
            "/init/test-database",
            post(init_handler::test_database_connection),
        )
        .route("/init/task-status", get(init_handler::get_task_status));

    protected.merge(public)
}

/// AI 分析深化路由（/ai，16 端点：工艺优化/质量预测/看板健康检查）
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
        // V15 P1 1.3+8.1：工艺优化→化验室打样集成
        .route(
            "/ai/process-optimizations/:id/push-to-lab-dip",
            post(ai_extend_handler::push_to_lab_dip),
        )
        // V15 P1 8.2：工艺优化→生产执行集成
        .route(
            "/ai/process-optimizations/:id/link-to-production",
            post(ai_extend_handler::link_to_production),
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
        // V15 P1 2.1+8.3：质量预测实际结果回填（对账）
        .route(
            "/ai/quality-predictions/:id/actual-result",
            post(ai_extend_handler::record_actual_quality_result),
        )
        // V15 P2 14.2.3：回填实际结果和索赔金额（误判成本追踪）
        .route(
            "/ai/quality-predictions/:id/actual-grade",
            post(ai_extend_handler::record_actual_grade),
        )
        // 看板 / 健康检查
        .route("/ai/summary", get(ai_extend_handler::ai_summary))
        .route("/ai/health", get(ai_extend_handler::ai_health))
        // 缺陷 16.4-D4 修复：AI 端点专用速率限制（10 req/min/user）
        .layer(axum::middleware::from_fn(rate_limit_ai_endpoint))
}

/// V15 P0-S14 敏感数据导出二级审批路由（/export-approvals，8 端点）
pub fn export_approval() -> Router<AppState> {
    Router::new()
        .route(
            "/export-approvals",
            get(export_approval_handler::list_approval_requests)
                .post(export_approval_handler::create_approval_request),
        )
        .route(
            "/export-approvals/verify-token",
            get(export_approval_handler::verify_token),
        )
        // V15 P2-05 修复：放在 `:id` 之前避免被 `:id` 路由吞掉。
        .route(
            "/export-approvals/pending-for-me",
            get(export_approval_handler::list_pending_for_me),
        )
        .route(
            "/export-approvals/:id",
            get(export_approval_handler::get_approval_request),
        )
        .route(
            "/export-approvals/:id/approve",
            post(export_approval_handler::approve_request),
        )
        .route(
            "/export-approvals/:id/reject",
            post(export_approval_handler::reject_request),
        )
        .route(
            "/export-approvals/:id/cancel",
            post(export_approval_handler::cancel_request),
        )
}

/// 系统域统一入口（子 router path 已加独立前缀，merge 安全）
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(dashboard())
        .merge(system_update())
        .merge(bpm())
        .merge(health())
        .merge(init())
        .merge(ai())
        .merge(export_approval())
        .merge(ws())
        .merge(audit_logs())
        .merge(slow_queries())
}
