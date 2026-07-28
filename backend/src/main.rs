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

use axum::Router;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};

use crate::bootstrap::service_bootstrap::BootstrapShutdownHandles;
use crate::config::settings::AppSettings;

/// 优雅停机信号监听（Ctrl+C / SIGTERM）。
/// 批次 114 P1-5：signal handler 安装失败改为优雅退出，避免 spawn 任务内 panic 影响 runtime。
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
    let settings = crate::bootstrap::infra_bootstrap::init_env_and_logging()?;
    let cors = crate::bootstrap::middleware_bootstrap::build_cors_layer(
        settings.cors.allowed_origins.clone(),
    );

    let (app, mut shutdown_handles) = build_app(&settings, cors).await?;
    let app = crate::bootstrap::middleware_bootstrap::apply_hsts_if_production(app);

    start_http_server(app, &settings.server.host, &settings.server.port).await?;
    shutdown_resources(&mut shutdown_handles);

    Ok(())
}

/// 根据数据库连接结果构建完整模式或 Setup 模式路由，返回路由和 shutdown 句柄。
async fn build_app(
    settings: &AppSettings,
    cors: CorsLayer,
) -> Result<(Router, BootstrapShutdownHandles), Box<dyn std::error::Error>> {
    let db_result = crate::bootstrap::infra_bootstrap::connect_database(settings).await;
    let mut shutdown_handles = BootstrapShutdownHandles::default();

    let app = match db_result {
        Ok(db) => {
            info!("数据库连接成功，启动完整模式");
            // 完整模式：迁移 → 服务创建 → 后台任务 → AppState 组装（见 bootstrap_full_mode）
            let (app_state, handles) =
                crate::bootstrap::service_bootstrap::bootstrap_full_mode(db, settings).await?;
            shutdown_handles = handles;
            crate::bootstrap::middleware_bootstrap::apply_full_mode_layers(app_state, cors.clone())
        }
        Err(e) => {
            info!("数据库连接失败: {}", e);
            info!("启动初始化模式，提供数据库配置API");
            // Setup 模式：仅暴露 /init/* 接口，TS-S-1 由 init_token_middleware 保护
            let router = crate::bootstrap::routes_bootstrap::create_init_router();
            crate::bootstrap::middleware_bootstrap::apply_init_mode_layers(router, cors.clone())
        }
    };

    Ok((app, shutdown_handles))
}

/// 启动 HTTP 服务器（含 graceful shutdown 信号监听）。
async fn start_http_server(
    app: Router,
    host: &str,
    port: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let http_addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    info!("HTTP 服务器监听地址：{}", http_addr);
    info!("===========================================");
    info!("系统启动完成，等待请求...");
    info!("HTTP 地址: {}", http_addr);
    info!("===========================================");

    // P2-12b 修复：into_make_service_with_connect_info 使 ConnectInfo 可用
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

    Ok(())
}

/// 按依赖顺序关闭后台任务与服务（事件总线/审计/定时任务/AppState）。
fn shutdown_resources(shutdown_handles: &mut BootstrapShutdownHandles) {
    // L-27+L-28+L-29 修复：关闭事件总线所有 spawn task，防止 detached task 泄漏
    crate::services::event_bus::shutdown_event_bus();

    // L-30/L-32 修复：关闭 OmniAuditEngine + AuditLogService（mpsc channel + handle abort）
    shutdown_handles.shutdown();

    // L-26 修复：关闭所有 main.rs 后台定时任务
    crate::bootstrap::service_bootstrap::shutdown_main_background_tasks();

    // L-26 修复：关闭 AppState 后台任务（审计清理 + 用户吊销清理）
    crate::utils::app_state::shutdown_app_state_background_tasks();
}
