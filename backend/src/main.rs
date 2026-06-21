mod cache; // P12 批 1：Redis 缓存层（lib crate bingxi_backend::cache 的镜像引用，让 server bin 也能解析 use crate::cache）
mod config;
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

use axum::http::{HeaderValue, Method, Request};
use axum::{
    routing::{get, post},
    Json, Router,
};
use sea_orm::Database;
use std::io::Write;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::cors::CorsLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, warn, Span};

use crate::config::settings::AppSettings;
use crate::middleware::auth::auth_middleware;
use crate::middleware::csrf::csrf_middleware;
use crate::middleware::permission::permission_middleware;
use crate::middleware::request_validator::request_validator_middleware;
use crate::routes::create_router;
use crate::services::init_service::{DatabaseConfig, InitService};
use crate::utils::log_config::{self, LogConfig};
use crate::utils::response::ApiResponse;

#[derive(Debug, serde::Serialize)]
struct InitStatusResponse {
    initialized: bool,
    message: String,
    mode: String,
}

#[derive(Debug, serde::Serialize)]
struct InitErrorResponse {
    error: String,
    message: String,
}

/// Setup 模式下的初始化成功标志。
///
/// 当数据库尚未连接成功时（例如：用户首次部署、或者刚刚迁移到一台新机器），
/// 后端会进入「Setup 模式」，仅暴露 `/init/*` 系列接口，不连接数据库。
/// 在该模式下，原始实现的 `get_init_status` 永远返回 `initialized: false`，
/// 会导致前端在 `initialize_with_db` 成功并跳转到登录页时，被路由守卫判定为
/// "系统未初始化" 再次拉回 setup 页面，形成跳转循环。
///
/// 修复方案：使用一个进程级的可变标志位记录本进程内是否已经成功完成初始化。
/// 注意：完整模式（数据库已连接）下不走此分支，因此对正常启动流程零影响。
static SETUP_MODE_INITIALIZED: std::sync::OnceLock<Arc<Mutex<bool>>> = std::sync::OnceLock::new();

fn setup_initialized_flag() -> Arc<Mutex<bool>> {
    SETUP_MODE_INITIALIZED
        .get_or_init(|| Arc::new(Mutex::new(false)))
        .clone()
}

async fn get_init_status() -> Json<InitStatusResponse> {
    // 优先使用内存中的初始化成功标志（处理「setup 模式内完成初始化」的场景）
    // P9-1: 用 unwrap_or_else + 日志替代裸 unwrap，锁中毒有 P9-1 中文提示
    let arc = setup_initialized_flag();
    let guard = arc.lock().unwrap_or_else(|e| {
        tracing::error!(error = %e, "P9-1: setup 初始化标志锁中毒");
        panic!("P9-1: setup 初始化标志锁中毒: {e}")
    });
    let initialized = *guard;
    if initialized {
        return Json(InitStatusResponse {
            initialized: true,
            message: "系统已初始化".to_string(),
            mode: "setup".to_string(),
        });
    }
    Json(InitStatusResponse {
        initialized: false,
        message: "系统未初始化，请先配置数据库".to_string(),
        mode: "setup".to_string(),
    })
}

async fn test_database_connection(
    Json(payload): Json<DatabaseConfig>,
) -> Result<
    Json<ApiResponse<crate::handlers::init_handler::TestDatabaseResponse>>,
    (axum::http::StatusCode, Json<InitErrorResponse>),
> {
    match InitService::test_database(&payload).await {
        Ok(_) => Ok(Json(ApiResponse::success_with_message(
            crate::handlers::init_handler::TestDatabaseResponse {
                success: true,
                message: "数据库连接成功".to_string(),
            },
            "数据库连接测试成功",
        ))),
        Err(e) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(InitErrorResponse {
                error: "database_connection_failed".to_string(),
                message: e.to_string(),
            }),
        )),
    }
}

async fn initialize_with_db(
    Json(payload): Json<crate::handlers::init_handler::InitWithDbRequest>,
) -> Result<
    Json<ApiResponse<crate::services::init_service::InitializationResult>>,
    (axum::http::StatusCode, Json<InitErrorResponse>),
> {
    match InitService::initialize_with_db(
        &payload.db_config,
        &payload.admin_username,
        &payload.admin_password,
    )
    .await
    {
        Ok(result) => {
            // 标记 setup 模式下的初始化已完成，便于 `get_init_status`
            // 在同一进程内返回 initialized = true，避免前端在跳转登录页时
            // 被路由守卫再次拉回 setup 页面。
            // P9-1: 用 unwrap_or_else 替代裸 unwrap，锁中毒有 P9-1 中文提示
            let arc = setup_initialized_flag();
            let mut guard = arc.lock().unwrap_or_else(|e| {
                tracing::error!(error = %e, "P9-1: setup 初始化标志锁中毒");
                panic!("P9-1: setup 初始化标志锁中毒: {e}")
            });
            *guard = true;
            Ok(Json(ApiResponse::success_with_message(
                result,
                "系统初始化成功",
            )))
        }
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
                Json(InitErrorResponse {
                    error: error.to_string(),
                    message,
                }),
            ))
        }
    }
}

fn create_init_router() -> Router<()> {
    Router::<()>::new().nest(
        "/api/v1/erp",
        Router::<()>::new()
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

    let log_level = settings.log.level.clone();
    let log_dir = settings.log.dir.clone();

    // 初始化增强日志系统
    let log_config = LogConfig {
        log_dir: log_dir.clone(),
        log_level: log_level.clone(),
    };
    log_config::init_enhanced_logging(&log_config)?;

    info!("===========================================");
    info!("启动面料管理 Rust 版");
    info!("运行环境：{}", settings.env);
    info!("===========================================");

    info!("配置加载成功");
    info!(
        "服务器地址：{}:{}",
        settings.server.host, settings.server.port
    );
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

    let app = match db_result {
        Ok(db) => {
            info!("数据库连接成功，启动完整模式");

            // 执行 SeaORM Migration 增加 TOTP 字段及性能优化索引
            // 防御式迁移：使用 IF EXISTS / DO 块确保表不存在时不会阻断服务启动
            use sea_orm::ConnectionTrait;
            let sql = "
                DO $$
                BEGIN
                    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'users') THEN
                        ALTER TABLE users ADD COLUMN IF NOT EXISTS totp_secret VARCHAR(255);
                        ALTER TABLE users ADD COLUMN IF NOT EXISTS is_totp_enabled BOOLEAN NOT NULL DEFAULT FALSE;
                    END IF;
                    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'sales_orders') THEN
                        CREATE INDEX IF NOT EXISTS idx_sales_order_customer ON sales_orders(customer_id);
                    END IF;
                    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'purchase_order') THEN
                        CREATE INDEX IF NOT EXISTS idx_purchase_order_supplier ON purchase_order(supplier_id);
                    END IF;
                    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'inventory_stocks') THEN
                        CREATE INDEX IF NOT EXISTS idx_inventory_product ON inventory_stocks(product_id, warehouse_id);
                    END IF;
                END $$;
            ";
            if let Err(e) = db.execute_unprepared(sql).await {
                warn!("执行 Migration 失败: {}", e);
            } else {
                info!("成功执行 Migration (TOTP 字段及性能索引)");
            }

            // P0-A 数据库迁移根治：启动时执行全部迁移（m0001-m0028）
            // 修复策略：移除 Some(5) 上限限制，让 Migrator::up 跑完所有 migration，
            // 避免 m0019_add_missing_columns 等关键 schema 修复被漏掉。
            // 全新部署时按编号顺序完整执行；已部署时 SeaORM 按名称去重。
            use migration::{Migrator, MigratorTrait};
            tracing::info!("启动时执行数据库迁移（全部 m0001-m0028）...");
            if let Err(e) = Migrator::up(&db, None).await {
                tracing::warn!("启动时迁移失败: {}，将在初始化时重试", e);
            } else {
                tracing::info!("数据库迁移执行完成");
            }

            std::io::stdout().flush().ok();
            std::io::stderr().flush().ok();

            // Wave B-2 修复（B2-1）：强制要求独立的 cookie_secret 配置，禁止降级复用 jwt_secret
            // 安全原因：JWT 与 Cookie 使用相同密钥会同时暴露两个攻击面（签名伪造 + Cookie 加密泄露），
            // 违反最小权限原则，且多副本部署时若运维误改 JWT 会同步影响 Cookie 加密强度。
            // 强制要求通过环境变量 COOKIE_SECRET 或配置项 auth.cookie_secret 显式注入。
            let cookie_secret = match settings.auth.cookie_secret.clone() {
                Some(secret) => secret,
                None => {
                    eprintln!("FATAL: COOKIE_SECRET 环境变量或 auth.cookie_secret 配置必须显式设置");
                    eprintln!("FATAL: 出于安全考虑，禁止降级复用 AUTH__JWT_SECRET 作为 Cookie 加密密钥");
                    eprintln!("FATAL: 请使用 `openssl rand -hex 32` 生成至少 32 字节的强随机密钥");
                    eprintln!("FATAL: 并通过环境变量 COOKIE_SECRET 或 config.yaml 的 auth.cookie_secret 字段注入");
                    std::process::exit(1);
                }
            };

            if cookie_secret.len() < 32 {
                eprintln!("FATAL: COOKIE_SECRET 长度不足 32 字节（当前: {} 字节）", cookie_secret.len());
                eprintln!("FATAL: 出于安全考虑，禁止以补 0 / 截断等方式弱化 Cookie 加密密钥");
                eprintln!("FATAL: 请使用 `openssl rand -hex 32` 生成至少 32 字节（64 个十六进制字符）的强随机密钥");
                eprintln!("FATAL: 并通过环境变量 COOKIE_SECRET 或 config.yaml 的 auth.cookie_secret 字段注入");
                std::process::exit(1);
            }
            let db = Arc::new(db);

            let omni_audit = Arc::new(crate::services::omni_audit_service::OmniAuditEngine::new(
                db.clone(),
            )?);
            let audit_cleanup = Arc::new(
                crate::services::audit_cleanup_service::AuditCleanupService::new(db.clone(), 999),
            );

            // P13 批 1 B-慢查询审计：启动后台采集任务（默认 5 分钟间隔）。
            // 部署-3 修复：增加 slow_query.enabled 配置开关。
            // 关闭时（CI 容器 / 未安装 pg_stat_statements 扩展的数据库）完全跳过采集任务。
            // 开启时（默认）按配置间隔采集，失败仅记录日志，不阻断 main 启动。
            if settings.slow_query.enabled {
                let slow_collector = Arc::new(
                    crate::services::slow_query_collector::SlowQueryCollector::new(
                        db.clone(),
                        settings.slow_query.threshold_ms,
                        settings.slow_query.limit_rows,
                    ),
                );
                slow_collector
                    .clone()
                    .start_collect_task(settings.slow_query.interval_secs);
                info!(
                    "慢查询采集任务已启动（间隔 {} 秒，阈值 {}ms）",
                    settings.slow_query.interval_secs, settings.slow_query.threshold_ms
                );
            } else {
                info!("慢查询采集任务已禁用（slow_query.enabled=false）");
            }

            let app_state = match crate::utils::app_state::AppState::with_secrets_and_cors(
                db,
                omni_audit,
                audit_cleanup,
                settings.auth.jwt_secret.clone(),
                settings.auth.previous_jwt_secret.clone(),
                cookie_secret,
                settings.cors.allowed_origins.clone(),
            ) {
                Ok(state) => state,
                Err(e) => {
                    return Err(format!("初始化应用全局状态失败: {}", e).into());
                }
            };
            let app_state_clone = app_state.clone();
            let app_state_clone2 = app_state.clone();
            let app_state_clone3 = app_state.clone();
            let app_state_clone4 = app_state.clone();
            let app_state_clone5 = app_state.clone();
            crate::services::event_bus::start_event_listener(app_state.db.clone()).await;
            crate::services::event_bus::init_event_bus_with_kafka_config(&settings.kafka).await;
            let app = create_router(app_state)
                // P3.2：审计上下文（必须在 trace_context 之内层挂载，
                // 即在 .layer() 链中位于 trace_context 之前；这样请求先经过
                // trace_context 注入 trace_id，再进入 audit_context 读取并补充 IP/UA）
                .layer(axum::middleware::from_fn(
                    crate::middleware::audit_context::audit_context_middleware,
                ))
                // P3.3：分布式追踪上下文（最最外层，确保下游都能拿到 trace_id）
                .layer(axum::middleware::from_fn(
                    crate::middleware::trace_context::trace_context_middleware,
                ))
                // P3.2：Prometheus 指标中间件（外层，记录所有请求的 method/route/status/耗时）
                .layer(axum::middleware::from_fn_with_state(
                    app_state_clone4,
                    crate::middleware::metrics::metrics_middleware,
                ))
                .layer(
                    TraceLayer::new_for_http()
                        .on_request(|request: &Request<_>, _span: &Span| {
                            let client_ip = request
                                .headers()
                                .get("x-forwarded-for")
                                .or_else(|| request.headers().get("x-real-ip"))
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("unknown")
                                .to_string();
                            let user_agent = request
                                .headers()
                                .get("user-agent")
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("unknown")
                                .to_string();
                            let origin = request
                                .headers()
                                .get("origin")
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("none")
                                .to_string();
                            info!(
                                method = %request.method(),
                                uri = %request.uri(),
                                client_ip = %client_ip,
                                user_agent = %user_agent,
                                origin = %origin,
                                "开始处理请求"
                            );
                        })
                        .on_response(
                            |response: &axum::response::Response, latency: Duration, _span: &Span| {
                                let status = response.status();
                                if status.is_success() {
                                    info!(
                                        status = %status,
                                        latency_ms = %latency.as_millis(),
                                        "请求完成"
                                    );
                                } else {
                                    warn!(
                                        status = %status,
                                        latency_ms = %latency.as_millis(),
                                        "请求异常"
                                    );
                                }
                            },
                        )
                        .on_failure(
                            |error: ServerErrorsFailureClass, latency: Duration, _span: &Span| {
                                warn!("请求失败：{:?} (耗时: {}ms)", error, latency.as_millis());
                            },
                        ),
                )
                .layer(cors.clone())
                // 中间件执行顺序：auth_middleware（最后注册、最外层、先执行）→ csrf_middleware → permission_middleware → request_validator → 处理器
                .layer(axum::middleware::from_fn_with_state(app_state_clone3, request_validator_middleware))
                .layer(axum::middleware::from_fn_with_state(app_state_clone2, permission_middleware))
                .layer(axum::middleware::from_fn_with_state(app_state_clone5, csrf_middleware))
                .layer(axum::middleware::from_fn_with_state(app_state_clone, auth_middleware))
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
            app
        }
        Err(e) => {
            info!("数据库连接失败: {}", e);
            info!("启动初始化模式，提供数据库配置API");

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
                    ))
        }
    };

    let http_addr: SocketAddr =
        format!("{}:{}", settings.server.host, settings.server.port).parse()?;
    info!("HTTP 服务器监听地址：{}", http_addr);

    info!("===========================================");
    info!("系统启动完成，等待请求...");
    info!("HTTP 地址: {}", http_addr);
    info!("===========================================");

    let http_server = axum::serve(tokio::net::TcpListener::bind(http_addr).await?, app)
        .with_graceful_shutdown(async {
            shutdown_signal().await;
        });

    if let Err(e) = http_server.await {
        warn!("HTTP 服务器错误: {}", e);
    }

    Ok(())
}
