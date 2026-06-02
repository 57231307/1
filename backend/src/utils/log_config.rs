use std::fs;
use std::path::Path;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// 日志配置
pub struct LogConfig {
    pub log_dir: String,
    pub log_level: String,
}

/// 初始化增强日志系统
pub fn init_enhanced_logging(config: &LogConfig) -> Result<(), Box<dyn std::error::Error>> {
    // 创建日志目录
    let log_dir = Path::new(&config.log_dir);
    fs::create_dir_all(log_dir)?;

    // 创建子目录
    let audit_dir = log_dir.join("audit");
    let security_dir = log_dir.join("security");
    let performance_dir = log_dir.join("performance");
    fs::create_dir_all(&audit_dir)?;
    fs::create_dir_all(&security_dir)?;
    fs::create_dir_all(&performance_dir)?;

    // 主日志文件
    let main_appender = RollingFileAppender::new(
        Rotation::DAILY,
        log_dir,
        "bingxi_backend.log",
    );

    // 资金操作日志
    let financial_appender = RollingFileAppender::new(
        Rotation::DAILY,
        &audit_dir,
        "financial_audit.log",
    );

    // 权限变更日志
    let permission_appender = RollingFileAppender::new(
        Rotation::DAILY,
        &audit_dir,
        "permission_audit.log",
    );

    // 安全事件日志
    let security_appender = RollingFileAppender::new(
        Rotation::DAILY,
        &security_dir,
        "security_audit.log",
    );

    // 数据库操作日志
    let database_appender = RollingFileAppender::new(
        Rotation::DAILY,
        &audit_dir,
        "database_audit.log",
    );

    // 性能监控日志
    let performance_appender = RollingFileAppender::new(
        Rotation::DAILY,
        &performance_dir,
        "performance_audit.log",
    );

    // 业务操作日志
    let business_appender = RollingFileAppender::new(
        Rotation::DAILY,
        &audit_dir,
        "business_audit.log",
    );

    // 系统健康日志
    let health_appender = RollingFileAppender::new(
        Rotation::DAILY,
        &performance_dir,
        "system_health.log",
    );

    // 错误日志
    let error_appender = RollingFileAppender::new(
        Rotation::DAILY,
        log_dir,
        "error.log",
    );

    // 创建过滤器
    let main_filter = tracing_subscriber::filter::filter_fn(|meta| {
        !meta.target().starts_with("financial_audit")
            && !meta.target().starts_with("permission_audit")
            && !meta.target().starts_with("security_audit")
            && !meta.target().starts_with("database_audit")
            && !meta.target().starts_with("performance_audit")
            && !meta.target().starts_with("business_audit")
            && !meta.target().starts_with("system_health")
    });

    let financial_filter = tracing_subscriber::filter::filter_fn(|meta| {
        meta.target().starts_with("financial_audit")
    });

    let permission_filter = tracing_subscriber::filter::filter_fn(|meta| {
        meta.target().starts_with("permission_audit")
    });

    let security_filter = tracing_subscriber::filter::filter_fn(|meta| {
        meta.target().starts_with("security_audit")
    });

    let database_filter = tracing_subscriber::filter::filter_fn(|meta| {
        meta.target().starts_with("database_audit")
    });

    let performance_filter = tracing_subscriber::filter::filter_fn(|meta| {
        meta.target().starts_with("performance_audit")
    });

    let business_filter = tracing_subscriber::filter::filter_fn(|meta| {
        meta.target().starts_with("business_audit")
    });

    let health_filter = tracing_subscriber::filter::filter_fn(|meta| {
        meta.target().starts_with("system_health")
    });

    let error_filter = tracing_subscriber::filter::filter_fn(|meta| {
        *meta.level() == tracing::Level::ERROR
    });

    // 主日志层
    let main_layer = tracing_subscriber::fmt::layer()
        .with_writer(main_appender)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .with_filter(main_filter);

    // 资金操作日志层
    let financial_layer = tracing_subscriber::fmt::layer()
        .with_writer(financial_appender)
        .with_ansi(false)
        .with_target(true)
        .with_filter(financial_filter);

    // 权限变更日志层
    let permission_layer = tracing_subscriber::fmt::layer()
        .with_writer(permission_appender)
        .with_ansi(false)
        .with_target(true)
        .with_filter(permission_filter);

    // 安全事件日志层
    let security_layer = tracing_subscriber::fmt::layer()
        .with_writer(security_appender)
        .with_ansi(false)
        .with_target(true)
        .with_filter(security_filter);

    // 数据库操作日志层
    let database_layer = tracing_subscriber::fmt::layer()
        .with_writer(database_appender)
        .with_ansi(false)
        .with_target(true)
        .with_filter(database_filter);

    // 性能监控日志层
    let performance_layer = tracing_subscriber::fmt::layer()
        .with_writer(performance_appender)
        .with_ansi(false)
        .with_target(true)
        .with_filter(performance_filter);

    // 业务操作日志层
    let business_layer = tracing_subscriber::fmt::layer()
        .with_writer(business_appender)
        .with_ansi(false)
        .with_target(true)
        .with_filter(business_filter);

    // 系统健康日志层
    let health_layer = tracing_subscriber::fmt::layer()
        .with_writer(health_appender)
        .with_ansi(false)
        .with_target(true)
        .with_filter(health_filter);

    // 错误日志层
    let error_layer = tracing_subscriber::fmt::layer()
        .with_writer(error_appender)
        .with_ansi(false)
        .with_target(true)
        .with_filter(error_filter);

    // 控制台输出层（仅开发环境）
    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .with_target(true)
        .with_filter(tracing_subscriber::filter::filter_fn(|meta| {
            // 在生产环境禁用控制台输出
            std::env::var("ENV").unwrap_or_else(|_| "development".to_string()) != "production"
        }));

    // 初始化订阅者
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            format!("bingxi_backend={},tower_http=debug", config.log_level).into()
        }))
        .with(main_layer)
        .with(financial_layer)
        .with(permission_layer)
        .with(security_layer)
        .with(database_layer)
        .with(performance_layer)
        .with(business_layer)
        .with(health_layer)
        .with(error_layer)
        .with(console_layer)
        .init();

    tracing::info!("增强日志系统初始化完成");
    tracing::info!("日志目录: {}", config.log_dir);
    tracing::info!("审计日志: {}/audit/", config.log_dir);
    tracing::info!("安全日志: {}/security/", config.log_dir);
    tracing::info!("性能日志: {}/performance/", config.log_dir);

    Ok(())
}
