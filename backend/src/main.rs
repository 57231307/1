mod bootstrap;
mod config;
mod constants; // BE-C: 全局常量（lib crate bingxi_backend::constants 的镜像引用，让 server bin 也能解析 crate::constants）
mod docs;
mod handlers;
mod middleware;
mod models;
mod observability;
mod routes;
mod search; // P9-8 Elasticsearch 集成（lib crate bingxi_backend::search 的镜像引用）
mod services;
mod utils;
mod websocket; // P3-2 WebSocket 实时通信（lib crate bingxi_backend::websocket 的镜像引用）

use std::net::SocketAddr;

use tracing::{info, warn};

use crate::bootstrap::service_bootstrap::BootstrapShutdownHandles;

/// 优雅停机信号监听（Ctrl+C / SIGTERM）。
///
/// 批次 114 P1-5：启动期 signal handler 安装失败改为优雅退出（原 `expect` 在 spawn 任务内 panic）。
/// Ctrl+C 信号处理器安装失败通常意味着运行环境异常（如非 TTY 容器），
/// 此时进程无法响应 Ctrl+C，但服务自身可继续运行；改为日志 + 优雅退出避免 panic 影响 runtime。
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "Ctrl+C 信号监听失败，进程将无法响应中断信号");
            std::process::exit(1);
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => {
                tracing::error!(error = %e, "SIGTERM 信号监听失败，进程将无法响应终止信号");
                std::process::exit(1);
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("系统收到关闭信号，开始优雅停机 (Graceful Shutdown)...");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ============================================================================
    // 1. 基础设施初始化：环境变量 / 配置 / 日志 / 健康检查启动时间
    // ============================================================================
    // 漏洞 #12 修复：在启动最早期加载 .env 文件，确保后续 is_production() 等
    // 环境变量判断能正确读到 APP_ENV（详见 infra_bootstrap::init_env_and_logging）。
    let settings = crate::bootstrap::infra_bootstrap::init_env_and_logging()?;

    // ============================================================================
    // 2. 构建 CORS 中间件层（基于配置白名单动态校验 Origin）
    // ============================================================================
    let cors = crate::bootstrap::middleware_bootstrap::build_cors_layer(
        settings.cors.allowed_origins.clone(),
    );

    // ============================================================================
    // 3. 尝试建立数据库连接，根据结果选择完整模式或 Setup 模式
    // ============================================================================
    let db_result = crate::bootstrap::infra_bootstrap::connect_database(&settings).await;

    // graceful shutdown 句柄（完整模式下填充，Setup 模式下保持 default）
    // L-30 修复（批次 372 v13 复审）：保留 OmniAuditEngine clone 用于 shutdown 后
    // 调用 OmniAuditEngine::shutdown()，避免审计引擎 detached task 泄漏
    // L-32 修复（批次 380 v13 复审）：保留 AuditLogService clone 用于 shutdown 后
    // 调用 AuditLogService::shutdown()，避免审计日志 detached task 泄漏
    let mut shutdown_handles = BootstrapShutdownHandles::default();

    let app = match db_result {
        Ok(db) => {
            info!("数据库连接成功，启动完整模式");

            // 完整模式：数据库迁移 → 服务创建 → 后台任务 → AppState 组装
            // 详见 service_bootstrap::bootstrap_full_mode（防御式迁移、SeaORM Migrator::up、
            // cookie_secret/webhook_secret 强度校验、OmniAuditEngine/AuditLogService/AuditCleanupService
            // 创建、慢查询采集/admin 缓存清理/JTI 黑名单清理/CRM 回收/FailoverMonitor/报表订阅调度
            // 后台任务启动、FailoverExecutor 构造、AppState 组装、事件总线 + 辅助核算维度 + ES 索引初始化）
            let (app_state, handles) =
                crate::bootstrap::service_bootstrap::bootstrap_full_mode(db, &settings).await?;
            shutdown_handles = handles;

            // 完整模式中间件链（从外到内）：
            // timeout → security headers → rate_limit → auth → omni_audit → csrf → permission
            // → request_validator → cors → trace → metrics → trace_context → audit_context
            // → body_limit → handler
            crate::bootstrap::middleware_bootstrap::apply_full_mode_layers(app_state, cors.clone())
        }
        Err(e) => {
            info!("数据库连接失败: {}", e);
            info!("启动初始化模式，提供数据库配置API");

            // Setup 模式：数据库未连接，仅暴露 /init/* 系列接口
            // TS-S-1 修复：高危初始化接口由 init_token_middleware 保护
            let router = crate::bootstrap::routes_bootstrap::create_init_router();

            // Setup 模式中间件链：TraceLayer + CORS + 安全头（无认证/权限/CSRF）
            crate::bootstrap::middleware_bootstrap::apply_init_mode_layers(router, cors.clone())
        }
    };

    // ============================================================================
    // 4. 条件注入 HSTS 头（仅 production 环境生效）
    // ============================================================================
    // P3 7-14 修复：原实现无条件注入，但 HTTP 模式下浏览器会忽略 HSTS 头，开发环境无效
    let app = crate::bootstrap::middleware_bootstrap::apply_hsts_if_production(app);

    // ============================================================================
    // 5. 启动 HTTP 服务器
    // ============================================================================
    let http_addr: SocketAddr =
        format!("{}:{}", settings.server.host, settings.server.port).parse()?;
    info!("HTTP 服务器监听地址：{}", http_addr);

    info!("===========================================");
    info!("系统启动完成，等待请求...");
    info!("HTTP 地址: {}", http_addr);
    info!("===========================================");

    // P2-12b 修复（批次 83 v1 复审）：启用 into_make_service_with_connect_info
    // 使 ConnectInfo<SocketAddr> 扩展可用，rate_limit/anti_brute_force 等 IP 提取链路可命中兜底
    let http_server = axum::serve(
        tokio::net::TcpListener::bind(http_addr).await?,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        shutdown_signal().await;
    });

    if let Err(e) = http_server.await {
        warn!("HTTP 服务器错误: {}", e);
    }

    // ============================================================================
    // 6. Graceful Shutdown：按依赖顺序关闭后台任务与服务
    // ============================================================================

    // L-27+L-28+L-29 修复（批次 373 v13 复审）：关闭事件总线所有 spawn task
    // abort Kafka 消费桥接 + 主事件监听器 + 库存财务桥接监听器，防止 detached task 泄漏
    crate::services::event_bus::shutdown_event_bus();

    // L-30 修复（批次 372 v13 复审）：关闭 OmniAuditEngine（mpsc channel + handle abort）
    // L-32 修复（批次 380 v13 复审）：关闭 AuditLogService（mpsc channel + handle abort）
    shutdown_handles.shutdown();

    // L-26 修复（批次 374 v13 复审）：关闭所有 main.rs 后台定时任务
    // abort admin缓存清理 + JTI黑名单清理 + 慢查询采集 + CRM回收 + FailoverMonitor + 报表订阅调度
    crate::bootstrap::service_bootstrap::shutdown_main_background_tasks();

    // L-26 修复（批次 374 v13 复审）：关闭 AppState 后台任务
    // abort 审计清理 + 用户吊销清理
    crate::utils::app_state::shutdown_app_state_background_tasks();

    Ok(())
}
