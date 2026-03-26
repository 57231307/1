mod config;
mod grpc;
mod handlers;
mod middleware;
mod models;
mod routes;
mod services;
mod utils;

use axum::http::Request;
use sea_orm::Database;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, warn, Level, Span};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::settings::AppSettings;
use crate::grpc::service::proto::auth_service_server::AuthServiceServer;
use crate::grpc::service::proto::user_service_server::UserServiceServer;
use crate::grpc::service::GrpcUserService;
use crate::middleware::auth::auth_middleware;
use crate::routes::create_router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载配置
    let settings = AppSettings::new()?;

    // 初始化日志（根据配置文件设置日志级别和目录）
    let _log_level = settings.log.level.parse::<Level>()?;
    let log_dir = &settings.log.dir;

    // 创建日志目录（如果不存在）
    std::fs::create_dir_all(log_dir)?;

    // 使用日志轮转：每天轮转一次，保留 7 天
    let file_appender = RollingFileAppender::new(Rotation::DAILY, log_dir, "bingxi_backend.log");

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("bingxi_backend={}", settings.log.level).into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(file_appender)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false),
        )
        .init();

    info!("===========================================");
    info!("启动秉羲管理系统 Rust 版");
    info!("运行环境：{}", settings.env);
    info!("===========================================");

    // 加载配置
    info!("配置加载成功");
    info!(
        "服务器地址：{}:{}",
        settings.server.host, settings.server.port
    );
    info!("gRPC 地址：{}:{}", settings.grpc.host, settings.grpc.port);
    info!("日志目录：{}", settings.log.dir);

    // 连接数据库
    let db = Database::connect(&settings.database.connection_string)
        .await
        .expect("数据库连接失败");
    info!("数据库连接成功");

    // 运行数据库迁移
    // TODO: 实现迁移逻辑

    // 创建 CORS 层
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 创建路由，添加请求追踪中间件
    let db_arc = Arc::new(db);
    let app = create_router(db_arc.clone())
        .layer(
            TraceLayer::new_for_http()
                .on_request(|request: &Request<_>, _span: &Span| {
                    info!(
                        method = %request.method(),
                        uri = %request.uri(),
                        "开始处理请求"
                    );
                })
                .on_response(
                    |response: &axum::response::Response, latency: Duration, _span: &Span| {
                        info!(
                            status = %response.status(),
                            latency_ms = %latency.as_millis(),
                            "请求完成"
                        );
                    },
                )
                .on_failure(
                    |error: ServerErrorsFailureClass, latency: Duration, _span: &Span| {
                        warn!("请求失败：{:?} (耗时: {}ms)", error, latency.as_millis());
                    },
                ),
        )
        .layer(cors)
        .layer(axum::middleware::from_fn(auth_middleware));

    // 启动 HTTP 服务器
    let http_addr: SocketAddr =
        format!("{}:{}", settings.server.host, settings.server.port).parse()?;
    info!("HTTP 服务器监听地址：{}", http_addr);

    // 创建 gRPC 服务
    let grpc_user_service = GrpcUserService::new(db_arc.clone(), settings.auth.jwt_secret.clone());
    let grpc_auth_service = grpc_user_service.clone();

    // 创建 gRPC 路由
    let grpc_router = tonic::transport::Server::builder()
        .add_service(UserServiceServer::new(grpc_user_service))
        .add_service(AuthServiceServer::new(grpc_auth_service));

    // 启动 gRPC 服务器
    let grpc_addr: SocketAddr = format!("{}:{}", settings.grpc.host, settings.grpc.port).parse()?;
    info!("gRPC 服务器监听地址：{}", grpc_addr);

    info!("===========================================");
    info!("系统启动完成，等待请求...");
    info!("===========================================");

    // 并发运行 HTTP 和 gRPC 服务器
    let http_future = axum::serve(tokio::net::TcpListener::bind(http_addr).await?, app);
    let grpc_future = grpc_router.serve(grpc_addr);

    tokio::select! {
        result = http_future => {
            if let Err(e) = result {
                info!("HTTP 服务器错误：{}", e);
            }
        }
        result = grpc_future => {
            if let Err(e) = result {
                info!("gRPC 服务器错误：{}", e);
            }
        }
    }

    Ok(())
}
