#![allow(warnings)]
mod config;
mod grpc;
mod handlers;
mod middleware;
mod models;
mod routes;
mod services;
mod utils;

use axum::http::{HeaderValue, Method, Request};
use axum::{
    routing::{get, post},
    Json, Router,
};
use sea_orm::Database;
use std::io::Write;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, warn, Level, Span};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::settings::AppSettings;
use crate::middleware::auth::auth_middleware;
use crate::middleware::permission::permission_middleware;
use crate::middleware::request_validator::request_validator_middleware;
use crate::routes::create_router;
use crate::services::init_service::{DatabaseConfig, InitService};

#[derive(Debug, serde::Serialize)]
struct InitStatusResponse {
    initialized: bool,
    message: String,
    mode: String,
}

async fn get_init_status() -> Json<InitStatusResponse> {
    let progress = crate::services::init_service::INIT_PROGRESS.read().unwrap();
    if progress.status == "completed" {
        Json(InitStatusResponse {
            initialized: true,
            message: "系统已初始化，请重启后端服务".to_string(),
            mode: "setup_completed".to_string(),
        })
    } else {
        Json(InitStatusResponse {
            initialized: false,
            message: "系统未初始化，请先配置数据库".to_string(),
            mode: "setup".to_string(),
        })
    }
}

async fn test_database_connection(
    Json(payload): Json<DatabaseConfig>,
) -> Result<
    Json<crate::handlers::init_handler::TestDatabaseResponse>,
    (
        axum::http::StatusCode,
        Json<crate::handlers::init_handler::ErrorResponse>,
    ),
> {
    match InitService::test_database(&payload).await {
        Ok(_) => Ok(Json(crate::handlers::init_handler::TestDatabaseResponse {
            success: true,
            message: "数据库连接成功".to_string(),
        })),
        Err(e) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(crate::handlers::init_handler::ErrorResponse {
                error: "database_connection_failed".to_string(),
                message: e.to_string(),
            }),
        )),
    }
}

async fn initialize_with_db(
    Json(payload): Json<crate::handlers::init_handler::InitWithDbRequest>,
) -> Result<
    Json<crate::services::init_service::InitializationResult>,
    (
        axum::http::StatusCode,
        Json<crate::handlers::init_handler::ErrorResponse>,
    ),
> {
    match InitService::initialize_with_db(
        &payload.db_config,
        &payload.admin_username,
        &payload.admin_password,
    )
    .await
    {
        Ok(result) => Ok(Json(result)),
        Err(e) => {
            let error = match e {
                crate::services::init_service::InitError::AlreadyInitialized => {
                    "already_initialized"
                }
                crate::services::init_service::InitError::HashError(_) => "hash_error",
                crate::services::init_service::InitError::DatabaseError(_) => "database_error",
                crate::services::init_service::InitError::UserNotFound => "user_not_found",
                crate::services::init_service::InitError::ConfigError(_) => "config_error",
            };

            let message = match e {
                crate::services::init_service::InitError::AlreadyInitialized => {
                    "系统已经初始化，不能重复初始化".to_string()
                }
                crate::services::init_service::InitError::HashError(msg) => {
                    format!("密码加密失败: {}", msg)
                }
                crate::services::init_service::InitError::DatabaseError(msg) => msg,
                crate::services::init_service::InitError::UserNotFound => "用户不存在".to_string(),
                crate::services::init_service::InitError::ConfigError(msg) => {
                    format!("配置错误: {}", msg)
                }
            };

            Err((
                axum::http::StatusCode::BAD_REQUEST,
                Json(crate::handlers::init_handler::ErrorResponse {
                    error: error.to_string(),
                    message,
                }),
            ))
        }
    }
}

async fn get_init_progress() -> Json<crate::services::init_service::InitProgress> {
    let progress = crate::services::init_service::INIT_PROGRESS
        .read()
        .unwrap()
        .clone();
    Json(progress)
}

fn create_init_router() -> Router {
    Router::new().nest(
        "/api/v1/erp",
        Router::new()
            .route("/init/status", get(get_init_status))
            .route("/init/progress", get(get_init_progress))
            .route("/init/test-database", post(test_database_connection))
            .route("/init/initialize-with-db", post(initialize_with_db)),
    )
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
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
    // 加载 .env 文件中的环境变量
    dotenvy::dotenv().ok();

    let settings = AppSettings::new()?;

    let _log_level = settings.log.level.parse::<Level>()?;
    let log_dir = &settings.log.dir;
    std::fs::create_dir_all(log_dir)?;

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

    info!("配置加载成功");
    info!(
        "服务器地址：{}:{}",
        settings.server.host, settings.server.port
    );
    info!("gRPC 地址：{}:{}", settings.grpc.host, settings.grpc.port);
    info!("日志目录：{}", settings.log.dir);

    let allowed_origins: Vec<HeaderValue> = settings
        .cors
        .allowed_origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect();

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
            axum::http::header::HeaderName::from_static("x-requested-with"),
        ])
        .allow_credentials(false)
        .max_age(Duration::from_secs(86400)); // 24小时

    let db_result = Database::connect(&settings.database.connection_string).await;
    
    // 如果数据库连接成功，额外检查数据库中是否已有数据（如存在用户记录）
    // 若无数据，则仍认为系统未初始化，进入引导模式
    let mut db_initialized = false;
    let mut active_db = None;
    if let Ok(db) = db_result {
        let service = InitService::new(Arc::new(db.clone()));
        let (initialized, _) = service.check_initialized().await;
        db_initialized = initialized;
        active_db = Some(db);
    }

    let app = if db_initialized {
        let db = active_db.unwrap();
        info!("数据库连接成功且已初始化，启动完整模式");

        std::io::stdout().flush().ok();
        std::io::stderr().flush().ok();

            let app_state = crate::utils::app_state::AppState::new(
                Arc::new(db),
                settings.auth.jwt_secret.clone(),
            );
            let app_state_clone = app_state.clone();
            create_router(app_state)
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
                .layer(cors.clone())
                .layer(axum::middleware::from_fn(request_validator_middleware))
                .layer(axum::middleware::from_fn_with_state(app_state_clone.clone(), auth_middleware))
                .layer(axum::middleware::from_fn_with_state(app_state_clone, permission_middleware))
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::X_CONTENT_TYPE_OPTIONS,
                    HeaderValue::from_static("nosniff"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::X_FRAME_OPTIONS,
                    HeaderValue::from_static("DENY"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::X_XSS_PROTECTION,
                    HeaderValue::from_static("1; mode=block"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::CONTENT_SECURITY_POLICY,
                    HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self';"),
                ))
                // .layer(SetResponseHeaderLayer::overriding(
                //     axum::http::header::STRICT_TRANSPORT_SECURITY,
                //     HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
                // ))
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::REFERRER_POLICY,
                    HeaderValue::from_static("strict-origin-when-cross-origin"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::HeaderName::from_static("permissions-policy"),
                    HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
                ))
    } else {
        info!("数据库连接失败或数据库为空");
        info!("启动初始化模式，提供引导配置");

        create_init_router()
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
                        ),
                )
                .layer(cors.clone())
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::X_CONTENT_TYPE_OPTIONS,
                    HeaderValue::from_static("nosniff"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::X_FRAME_OPTIONS,
                    HeaderValue::from_static("DENY"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::X_XSS_PROTECTION,
                    HeaderValue::from_static("1; mode=block"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::CONTENT_SECURITY_POLICY,
                    HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self';"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::STRICT_TRANSPORT_SECURITY,
                    HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::REFERRER_POLICY,
                    HeaderValue::from_static("strict-origin-when-cross-origin"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::HeaderName::from_static("permissions-policy"),
                    HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
                ))
    };

    let http_addr: SocketAddr =
        format!("{}:{}", settings.server.host, settings.server.port).parse()?;
    info!("HTTP 服务器监听地址：{}", http_addr);

    info!("===========================================");
    info!("系统启动完成，等待请求...");
    info!("===========================================");

    axum::serve(tokio::net::TcpListener::bind(http_addr).await?, app)
        .with_graceful_shutdown(async {
            shutdown_signal().await;
        })
        .await?;

    Ok(())
}
