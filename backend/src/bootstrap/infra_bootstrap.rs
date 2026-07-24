//! 基础设施初始化（环境变量 / 配置 / 日志 / 数据库连接）
//!
//! 职责：在 main() 最早期完成 .env 加载、配置解析、日志系统初始化，
//! 并尝试建立数据库连接。调用方根据连接结果决定走完整模式还是 Setup 模式。

use std::time::Duration;

use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing::info;

use crate::config::settings::AppSettings;
use crate::utils::log_config::{self, LogConfig};

/// 初始化环境变量、健康检查启动时间、配置加载与增强日志系统。
///
/// 返回加载成功的 `AppSettings`，供后续启动流程使用。
pub fn init_env_and_logging() -> Result<AppSettings, Box<dyn std::error::Error>> {
    // 漏洞 #12 修复：在启动最早期加载 .env 文件，确保后续 is_production() 等
    // 环境变量判断能正确读到 APP_ENV。dotenvy::dotenv() 在 .env 文件不存在时返回 Err，
    // 这里用 .ok() 静默忽略（生产环境通常通过 systemd EnvironmentFile 注入变量）。
    // 安全说明：dotenvy 仅加载**未设置**的环境变量，不会覆盖系统/CI 中已显式注入的值，
    // 避免 .env 文件意外覆盖部署期的环境配置。
    dotenvy::dotenv().ok();

    // 初始化健康检查的启动时间（OnceLock 首次写入即锁定，确保 uptime 反映真实进程运行时间）
    // L-9 修复（批次 375 v13 复审）：移除 let _ = 吞错模式，直接调用
    // start_time_init 返回 Instant（非 Result），get_or_init 不会失败，无需错误处理
    crate::handlers::health_handler::start_time_init();

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

    Ok(settings)
}

/// 配置数据库连接池并尝试建立连接。
///
/// 返回 `DatabaseConnection` 或连接错误，由调用方决定走完整模式还是 Setup 模式。
pub async fn connect_database(
    settings: &AppSettings,
) -> Result<DatabaseConnection, sea_orm::DbErr> {
    // 配置数据库连接池
    let mut db_opts = ConnectOptions::new(settings.database.connection_string.clone());
    db_opts
        .max_connections(settings.database.max_connections)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(1800))
        .sqlx_logging(true)
        .sqlx_logging_level(tracing::log::LevelFilter::Debug);

    Database::connect(db_opts).await
}
