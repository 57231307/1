mod config;
mod docs;
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
use tower_http::cors::CorsLayer;
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
    Json(InitStatusResponse {
        initialized: false,
        message: "系统未初始化，请先配置数据库".to_string(),
        mode: "setup".to_string(),
    })
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

fn create_init_router() -> Router {
    Router::new().nest(
        "/api/v1/erp",
        Router::new()
            .route("/init/status", get(get_init_status))
            .route("/init/test-database", post(test_database_connection))
            .route("/init/initialize-with-db", post(initialize_with_db)), // reset-password路由已在 routes/mod.rs 中配置
                                                                          // .route("/init/reset-password", post(crate::handlers::init_handler::reset_admin_password)),
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
    info!("启动面料管理 Rust 版");
    info!("运行环境：{}", settings.env);
    info!("===========================================");

    info!("配置加载成功");
    info!(
        "服务器地址：{}:{}",
        settings.server.host, settings.server.port
    );
    info!("gRPC 地址：{}:{}", settings.grpc.host, settings.grpc.port);
    info!("日志目录：{}", settings.log.dir);

    let allowed_origins = settings.cors.allowed_origins.clone();
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::predicate(
            move |origin: &HeaderValue, _request_parts: &axum::http::request::Parts| {
                // 动态验证 Origin 是否在白名单中
                let origin_str = origin.to_str().unwrap_or("");

                // 拒绝通配符，仅允许精确匹配
                allowed_origins.iter().any(|allowed| allowed == origin_str)
            },
        ))
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
        .allow_credentials(true) // 因为改成了 Cookie 鉴权，必须设置为 true
        .max_age(Duration::from_secs(86400)); // 24小时

    // 配置数据库连接池
    let mut db_opts = sea_orm::ConnectOptions::new(settings.database.connection_string.clone());
    db_opts
        .max_connections(settings.database.max_connections)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(1800))
        .sqlx_logging(true)
        .sqlx_logging_level(tracing::log::LevelFilter::Debug);

    let db_result = Database::connect(db_opts).await;

    let (app, grpc_db_opt) = match db_result {
        Ok(db) => {
            info!("数据库连接成功，启动完整模式");

            // 执行 SeaORM Migration 增加 TOTP 字段及性能优化索引
            use sea_orm::ConnectionTrait;
            let sql = "
                ALTER TABLE users ADD COLUMN IF NOT EXISTS totp_secret VARCHAR(255); 
                ALTER TABLE users ADD COLUMN IF NOT EXISTS is_totp_enabled BOOLEAN NOT NULL DEFAULT FALSE;
                
                -- 为常用查询添加索引
                CREATE INDEX IF NOT EXISTS idx_sales_order_customer ON sales_orders(customer_id); 
                CREATE INDEX IF NOT EXISTS idx_purchase_order_supplier ON purchase_orders(supplier_id); 
                CREATE INDEX IF NOT EXISTS idx_inventory_product ON inventory_stocks(product_id, warehouse_id);
            ";
            if let Err(e) = db.execute_unprepared(sql).await {
                warn!("执行 Migration 失败: {}", e);
            } else {
                info!("成功执行 Migration (TOTP 字段及性能索引)");
            }

            std::io::stdout().flush().ok();
            std::io::stderr().flush().ok();

            let cookie_secret = settings.auth.cookie_secret.clone().unwrap_or_else(|| {
                tracing::warn!("警告: 未配置 auth.cookie_secret，系统正在使用降级的 jwt_secret 作为替代（这存在安全风险）");
                settings.auth.jwt_secret.clone()
            });

            if cookie_secret.len() < 32 {
                tracing::warn!("配置警告: 用于 Cookie 加密的密钥长度不足 32 字节。系统将自动进行补齐以启动服务，但请在生产环境中配置至少 32 字节的强密钥！");
            }
            let db = Arc::new(db);
            let grpc_db = db.clone();
            let omni_audit = Arc::new(crate::services::omni_audit_service::OmniAuditEngine::new(
                db.clone(),
            )?);

            let app_state = crate::utils::app_state::AppState::with_secrets_and_cors(
                db,
                omni_audit,
                settings.auth.jwt_secret.clone(),
                settings.auth.previous_jwt_secret.clone(),
                cookie_secret,
                settings.cors.allowed_origins.clone(),
            );
            let app_state_clone = app_state.clone();
            let app_state_clone2 = app_state.clone();
            let app_state_clone3 = app_state.clone();
            crate::services::event_bus::start_event_listener(app_state.db.clone()).await;
            let app = create_router(app_state)
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
                // 中间件执行顺序：auth_middleware（最后注册、最外层、先执行）→ permission_middleware → request_validator → 处理器
                .layer(axum::middleware::from_fn_with_state(app_state_clone, auth_middleware))
                .layer(axum::middleware::from_fn_with_state(app_state_clone2, permission_middleware))
                .layer(axum::middleware::from_fn_with_state(app_state_clone3, request_validator_middleware))
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
                    HeaderValue::from_static("default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; connect-src 'self' ws: wss:; font-src 'self' data:; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'; upgrade-insecure-requests;"),
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
                .layer(axum::middleware::from_fn(crate::middleware::timeout::timeout_middleware));
            (app, Some(grpc_db))
        }
        Err(e) => {
            info!("数据库连接失败: {}", e);
            info!("启动初始化模式，提供数据库配置API");

            (
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
                        HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; connect-src 'self' ws: wss:; font-src 'self' data:; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none';"),
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
                    )),
                None,
            )
        }
    };

    let http_addr: SocketAddr =
        format!("{}:{}", settings.server.host, settings.server.port).parse()?;
    info!("HTTP 服务器监听地址：{}", http_addr);

    // 启动gRPC服务（如果数据库连接成功）
    let grpc_handle = if let Some(grpc_db) = grpc_db_opt {
        let grpc_addr: SocketAddr =
            format!("{}:{}", settings.grpc.host, settings.grpc.port).parse()?;
        let grpc_jwt_secret = settings.auth.jwt_secret.clone();

        // 预先检查端口是否可用，提供更清晰的错误信息
        match tokio::net::TcpListener::bind(grpc_addr).await {
            Ok(listener) => {
                let bound_addr = listener.local_addr()?;
                drop(listener); // 释放端口，让 gRPC 服务器使用

                Some(tokio::spawn(async move {
                    let user_service = crate::grpc::service::GrpcUserService::new(
                        grpc_db.clone(),
                        grpc_jwt_secret.clone(),
                    );
                    let management_services =
                        crate::grpc::management_services::GrpcManagementServices::new(
                            grpc_db.clone(),
                        );

                    let grpc_server = tonic::transport::Server::builder()
                        .add_service(crate::grpc::service::proto::user_service_server::UserServiceServer::new(user_service.clone()))
                        .add_service(crate::grpc::service::proto::auth_service_server::AuthServiceServer::new(user_service))
                        .add_service(crate::grpc::service::proto::purchase_contract_service_server::PurchaseContractServiceServer::new(management_services.clone()))
                        .add_service(crate::grpc::service::proto::sales_contract_service_server::SalesContractServiceServer::new(management_services.clone()))
                        .add_service(crate::grpc::service::proto::fixed_asset_service_server::FixedAssetServiceServer::new(management_services.clone()))
                        .add_service(crate::grpc::service::proto::budget_management_service_server::BudgetManagementServiceServer::new(management_services));

                    info!("gRPC 服务器监听地址：{}", bound_addr);
                    if let Err(e) = grpc_server.serve(bound_addr).await {
                        warn!("gRPC 服务器运行错误: {} (地址: {})", e, bound_addr);
                    }
                }))
            }
            Err(e) => {
                warn!("gRPC 端口 {} 不可用: {}，跳过 gRPC 服务启动", grpc_addr, e);
                None
            }
        }
    } else {
        info!("数据库未连接，跳过gRPC服务启动");
        None
    };

    info!("===========================================");
    info!("系统启动完成，等待请求...");
    info!("HTTP 地址: {}", http_addr);
    if grpc_handle.is_some() {
        info!("gRPC 服务: 已启用");
    } else {
        info!("gRPC 服务: 未启用");
    }
    info!("===========================================");

    let http_server = axum::serve(tokio::net::TcpListener::bind(http_addr).await?, app)
        .with_graceful_shutdown(async {
            shutdown_signal().await;
        });

    if let Some(grpc) = grpc_handle {
        tokio::select! {
            result = http_server => {
                if let Err(e) = result {
                    warn!("HTTP 服务器错误: {}", e);
                }
            }
            _ = grpc => {
                info!("gRPC 服务器已关闭");
            }
        }
    } else {
        if let Err(e) = http_server.await {
            warn!("HTTP 服务器错误: {}", e);
        }
    }

    Ok(())
}
