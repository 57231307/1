use std::fs;
use std::path::Path;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    layer::{Layer, SubscriberExt},
    util::SubscriberInitExt,
    Registry,
};

/// 日志配置
pub struct LogConfig {
    pub log_dir: String,
    pub log_level: String,
}

/// 装箱后的日志层（类型擦除，便于组合异构 layer）
type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync>;

/// 初始化增强日志系统
pub fn init_enhanced_logging(config: &LogConfig) -> Result<(), Box<dyn std::error::Error>> {
    if is_kubernetes_environment() {
        init_stdout_json_logging(config);
        return Ok(());
    }

    init_host_logging(config)?;
    Ok(())
}

fn is_kubernetes_environment() -> bool {
    std::env::var("KUBERNETES_SERVICE_HOST").is_ok()
}

fn init_stdout_json_logging(config: &LogConfig) {
    // V15 P1 20.8-A：K8s 环境 stdout 使用 JSON 格式，便于 Loki/ELK 直接采集
    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(true)
        .json();

    tracing_subscriber::registry()
        .with(create_env_filter(config))
        .with(console_layer)
        .init();

    tracing::info!("增强日志系统初始化完成 (K8s 环境: stdout JSON 模式)");
}

fn init_host_logging(config: &LogConfig) -> Result<(), Box<dyn std::error::Error>> {
    let log_dir = Path::new(&config.log_dir);
    create_log_directories(log_dir)?;

    let (main_layer, error_layer) = create_main_layers(log_dir)?;
    let audit_layers = create_audit_layers(log_dir)?;
    let performance_layers = create_performance_layers(log_dir)?;
    let security_layer = create_security_layer(log_dir)?;
    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .with_target(true)
        .boxed();

    // V15 P1 CI 修复：使用 Vec<BoxedLayer> 替代 tuple，避免异构 tuple 的 Layer<S> trait bound 不满足
    // Vec<L> 实现 Layer<S>（当 L: Layer<S>），且 BoxedLayer = Box<dyn Layer<Registry> + Send + Sync>
    // 各层均通过 .boxed() 类型擦除为 BoxedLayer，统一存入 Vec 后一次性 .with()
    let layers: Vec<BoxedLayer> = vec![
        create_env_filter(config).boxed(),
        main_layer,
        error_layer,
        audit_layers,
        performance_layers,
        security_layer,
        console_layer,
    ];

    tracing_subscriber::registry().with(layers).init();

    log_initialization_info(config);
    Ok(())
}

fn create_log_directories(log_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(log_dir)?;
    fs::create_dir_all(log_dir.join("audit"))?;
    fs::create_dir_all(log_dir.join("security"))?;
    fs::create_dir_all(log_dir.join("performance"))?;
    Ok(())
}

fn create_env_filter(config: &LogConfig) -> tracing_subscriber::EnvFilter {
    tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("bingxi_backend={},tower_http=debug", config.log_level).into())
}

fn create_main_layers(
    log_dir: &Path,
) -> Result<(BoxedLayer, BoxedLayer), Box<dyn std::error::Error>> {
    let main_appender = RollingFileAppender::new(Rotation::DAILY, log_dir, "bingxi_backend.log");
    let error_appender = RollingFileAppender::new(Rotation::DAILY, log_dir, "error.log");

    // V15 P1 20.8-A：文件层使用 JSON 格式，便于 ELK/Loki 直接索引结构化字段
    let main_layer = tracing_subscriber::fmt::layer()
        .with_writer(main_appender)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .json()
        .boxed();

    let error_layer = tracing_subscriber::fmt::layer()
        .with_writer(error_appender)
        .with_ansi(false)
        .with_target(true)
        .json()
        .boxed();

    Ok((main_layer, error_layer))
}

fn create_audit_layers(log_dir: &Path) -> Result<BoxedLayer, Box<dyn std::error::Error>> {
    let audit_dir = log_dir.join("audit");
    let financial_appender =
        RollingFileAppender::new(Rotation::DAILY, &audit_dir, "financial_audit.log");
    let permission_appender =
        RollingFileAppender::new(Rotation::DAILY, &audit_dir, "permission_audit.log");
    let database_appender =
        RollingFileAppender::new(Rotation::DAILY, &audit_dir, "database_audit.log");
    let business_appender =
        RollingFileAppender::new(Rotation::DAILY, &audit_dir, "business_audit.log");

    // V15 P1 20.8-A：审计层使用 JSON 格式，便于 ELK/Loki 直接索引结构化字段
    let financial_layer: BoxedLayer = tracing_subscriber::fmt::layer()
        .with_writer(financial_appender)
        .with_ansi(false)
        .with_target(true)
        .json()
        .boxed();
    let permission_layer: BoxedLayer = tracing_subscriber::fmt::layer()
        .with_writer(permission_appender)
        .with_ansi(false)
        .with_target(true)
        .json()
        .boxed();
    let database_layer: BoxedLayer = tracing_subscriber::fmt::layer()
        .with_writer(database_appender)
        .with_ansi(false)
        .with_target(true)
        .json()
        .boxed();
    let business_layer: BoxedLayer = tracing_subscriber::fmt::layer()
        .with_writer(business_appender)
        .with_ansi(false)
        .with_target(true)
        .json()
        .boxed();

    Ok(vec![
        financial_layer,
        permission_layer,
        database_layer,
        business_layer,
    ]
    .boxed())
}

fn create_performance_layers(log_dir: &Path) -> Result<BoxedLayer, Box<dyn std::error::Error>> {
    let performance_dir = log_dir.join("performance");
    let performance_appender =
        RollingFileAppender::new(Rotation::DAILY, &performance_dir, "performance_audit.log");
    let health_appender =
        RollingFileAppender::new(Rotation::DAILY, &performance_dir, "system_health.log");

    // V15 P1 20.8-A：性能层使用 JSON 格式
    let performance_layer: BoxedLayer = tracing_subscriber::fmt::layer()
        .with_writer(performance_appender)
        .with_ansi(false)
        .with_target(true)
        .json()
        .boxed();
    let health_layer: BoxedLayer = tracing_subscriber::fmt::layer()
        .with_writer(health_appender)
        .with_ansi(false)
        .with_target(true)
        .json()
        .boxed();

    Ok(vec![performance_layer, health_layer].boxed())
}

fn create_security_layer(log_dir: &Path) -> Result<BoxedLayer, Box<dyn std::error::Error>> {
    let security_dir = log_dir.join("security");
    let security_appender =
        RollingFileAppender::new(Rotation::DAILY, &security_dir, "security_audit.log");

    // V15 P1 20.8-A：安全层使用 JSON 格式
    Ok(tracing_subscriber::fmt::layer()
        .with_writer(security_appender)
        .with_ansi(false)
        .with_target(true)
        .json()
        .boxed())
}

fn log_initialization_info(config: &LogConfig) {
    tracing::info!("增强日志系统初始化完成");
    tracing::info!("日志目录: {}", config.log_dir);
    tracing::info!("审计日志: {}/audit/", config.log_dir);
    tracing::info!("安全日志: {}/security/", config.log_dir);
    tracing::info!("性能日志: {}/performance/", config.log_dir);
}
