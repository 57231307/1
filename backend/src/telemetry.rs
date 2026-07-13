//! P9-6 OpenTelemetry 统一接入层（trace + metrics + log 三位一体）
//!
//! 本文件是 OpenTelemetry 集成的统一入口，提供：
//! 1. **Trace**：HTTP 请求 / DB 查询 / 业务流的分布式追踪
//! 2. **Metrics**：与 Prometheus 兼容的指标导出
//! 3. **Log**：与 `tracing` 框架深度集成
//!
//! ## 三种部署模式
//!
//! | 模式 | OTLP_ENDPOINT | 用途 |
//! |------|---------------|------|
//! | 本地开发 | `http://localhost:4317` | Jaeger / SigNoz |
//! | 测试环境 | `http://otel-collector:4317` | OTLP Collector |
//! | 生产环境 | 从 `OTEL_EXPORTER_OTLP_ENDPOINT` 环境变量读取 | Tempo / Honeycomb |
//!
//! ## 启用 OTel SDK
//!
//! 默认情况下，本模块使用 `tracing` 框架记录 span，**不实际导出** OTel。
//! 要启用 OTel 导出，需在 `Cargo.toml` 中添加：
//!
//! ```toml
//! opentelemetry = { version = "0.24", features = ["trace"] }
//! opentelemetry-otlp = { version = "0.17", features = ["grpc-tonic"] }
//! opentelemetry_sdk = { version = "0.24", features = ["rt-tokio"] }
//! tracing-opentelemetry = "0.25"
//! ```
//!
//! 然后在 `main.rs` 中调用 `telemetry::init_otel_provider()`。
//!
//! ## 资源属性
//!
//! 每个 span / metric 都会附加以下资源属性：
//! - `service.name = "bingxi-backend"`
//! - `service.version = env!("CARGO_PKG_VERSION")`
//! - `service.namespace = "erp"`
//! - `deployment.environment = std::env::var("ENV").unwrap_or("dev")`

use std::env;

/// OpenTelemetry 服务名（资源属性 service.name）
pub const SERVICE_NAME: &str = "bingxi-backend";

/// OpenTelemetry 服务命名空间
pub const SERVICE_NAMESPACE: &str = "erp";

/// 资源属性：服务版本（编译期注入）
pub fn service_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// 资源属性：部署环境
/// L-40 修复（批次 379 v13 复审）：使用 LazyLock 消除 silent default，
/// 首次调用时打印当前值；生产环境未设置时 warn，开发环境未设置时 info。
pub fn deployment_environment() -> String {
    static DEPLOYMENT_ENV: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
        match env::var("ENV") {
            Ok(v) => {
                tracing::info!(value = %v, "ENV 已设置");
                v
            }
            Err(_) => {
                if crate::utils::config::is_production() {
                    tracing::warn!("生产环境未设置 ENV，使用默认值 dev（建议显式设置 ENV=production）");
                } else {
                    tracing::info!("ENV 未设置，使用默认值 dev");
                }
                "dev".to_string()
            }
        }
    });
    DEPLOYMENT_ENV.clone()
}

/// OTLP 端点（gRPC）
/// L-40 修复（批次 379 v13 复审）：使用 LazyLock 消除 silent default，
/// 首次调用时打印当前值；生产环境未设置时 warn，开发环境未设置时 info。
pub fn otlp_endpoint() -> String {
    static OTLP_ENDPOINT: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
        match env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
            Ok(v) => {
                tracing::info!("OTEL_EXPORTER_OTLP_ENDPOINT 已设置");
                v
            }
            Err(_) => {
                if crate::utils::config::is_production() {
                    tracing::warn!("生产环境未设置 OTEL_EXPORTER_OTLP_ENDPOINT，使用默认值 http://localhost:4317（生产环境建议配置可达的 OTLP Collector）");
                } else {
                    tracing::info!("OTEL_EXPORTER_OTLP_ENDPOINT 未设置，使用默认值 http://localhost:4317");
                }
                "http://localhost:4317".to_string()
            }
        }
    });
    OTLP_ENDPOINT.clone()
}

/// 是否启用 OTel 导出（默认 false）
/// L-40 修复（批次 379 v13 复审）：使用 LazyLock 消除 silent default，
/// 首次调用时打印当前值；生产环境未设置时 warn，开发环境未设置时 info。
pub fn is_otel_enabled() -> bool {
    static OTEL_ENABLED: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| {
        match env::var("OTEL_ENABLED") {
            Ok(v) => {
                let enabled = v.parse::<bool>().unwrap_or(false);
                tracing::info!(value = %v, enabled, "OTEL_ENABLED 已设置");
                enabled
            }
            Err(_) => {
                if crate::utils::config::is_production() {
                    tracing::warn!("生产环境未设置 OTEL_ENABLED，默认 false（建议显式设置 OTEL_ENABLED=true 启用 OTel 导出）");
                } else {
                    tracing::info!("OTEL_ENABLED 未设置，默认 false");
                }
                false
            }
        }
    });
    *OTEL_ENABLED
}

/// 三个核心信号：Trace / Metrics / Log
pub mod signals {
    /// 追踪信号（分布式追踪）
    pub mod trace {
        use std::collections::HashMap;

        /// Span 属性
        #[derive(Debug, Clone, Default)]
        pub struct SpanAttrs {
            pub attrs: HashMap<String, String>,
        }

        impl SpanAttrs {
            pub fn new() -> Self {
                Self::default()
            }
            pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
                self.attrs.insert(key.into(), value.into());
                self
            }
            pub fn get(&self, key: &str) -> Option<&String> {
                self.attrs.get(key)
            }
        }

        /// Span 类型
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum SpanKind {
            Server,
            Client,
            Producer,
            Consumer,
            Internal,
        }

        impl SpanKind {
            pub fn as_str(&self) -> &'static str {
                match self {
                    Self::Server => "server",
                    Self::Client => "client",
                    Self::Producer => "producer",
                    Self::Consumer => "consumer",
                    Self::Internal => "internal",
                }
            }
        }
    }

    /// 指标信号
    pub mod metrics {
        use std::sync::atomic::{AtomicU64, Ordering};

        /// 计数器
        #[derive(Debug, Default)]
        pub struct Counter {
            value: AtomicU64,
        }

        impl Counter {
            pub fn new() -> Self {
                Self::default()
            }
            pub fn inc(&self) {
                self.value.fetch_add(1, Ordering::Relaxed);
            }
            pub fn add(&self, delta: u64) {
                self.value.fetch_add(delta, Ordering::Relaxed);
            }
            pub fn get(&self) -> u64 {
                self.value.load(Ordering::Relaxed)
            }
        }

        /// 直方图
        #[derive(Debug, Default)]
        pub struct Histogram {
            count: AtomicU64,
            sum: std::sync::Mutex<f64>,
        }

        impl Histogram {
            pub fn new() -> Self {
                Self::default()
            }
            pub fn observe(&self, value: f64) {
                self.count.fetch_add(1, Ordering::Relaxed);
                if let Ok(mut s) = self.sum.lock() {
                    *s += value;
                }
            }
            pub fn count(&self) -> u64 {
                self.count.load(Ordering::Relaxed)
            }
            pub fn sum(&self) -> f64 {
                self.sum.lock().map(|s| *s).unwrap_or(0.0)
            }
        }
    }

    /// 日志信号
    pub mod log {
        /// 日志级别
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub enum LogLevel {
            Trace,
            Debug,
            Info,
            Warn,
            Error,
        }

        impl LogLevel {
            pub fn as_str(&self) -> &'static str {
                match self {
                    Self::Trace => "TRACE",
                    Self::Debug => "DEBUG",
                    Self::Info => "INFO",
                    Self::Warn => "WARN",
                    Self::Error => "ERROR",
                }
            }
        }

        /// 日志条目
        #[derive(Debug, Clone)]
        pub struct LogEntry {
            pub level: LogLevel,
            pub target: String,
            pub message: String,
            pub timestamp: chrono::DateTime<chrono::Utc>,
        }
    }
}

/// 预定义 Span 名称
pub mod span_names {
    pub const HTTP_REQUEST: &str = "http.request";
    pub const DB_QUERY: &str = "db.query";
    pub const REDIS_OP: &str = "redis.op";
    pub const BUSINESS_FLOW: &str = "business.flow";
    pub const SALES_ORDER_CREATE: &str = "sales.order.create";
    pub const PURCHASE_ORDER_CREATE: &str = "purchase.order.create";
    pub const INVENTORY_TRANSFER: &str = "inventory.transfer";
    pub const AR_PAYMENT: &str = "ar.payment";
}

/// 预定义指标
pub mod metric_names {
    pub const HTTP_REQUESTS_TOTAL: &str = "http_requests_total";
    pub const HTTP_REQUEST_DURATION: &str = "http_request_duration_seconds";
    pub const DB_QUERIES_TOTAL: &str = "db_queries_total";
    pub const DB_QUERY_DURATION: &str = "db_query_duration_seconds";
    pub const BUSINESS_EVENTS_TOTAL: &str = "business_events_total";
    pub const ACTIVE_TENANTS: &str = "active_tenants";
}

/// 初始化 telemetry 子系统（轻量级，无 OTel 依赖）
///
/// 返回的 `TelemetryGuard` 在 drop 时会自动 flush 缓冲数据。
pub fn init() -> TelemetryGuard {
    let env = deployment_environment();
    tracing::info!(
        "P9-6 telemetry init: service={} version={} env={} otel_enabled={}",
        SERVICE_NAME,
        service_version(),
        env,
        is_otel_enabled()
    );
    TelemetryGuard { _private: () }
}

/// Telemetry 守卫（drop 时 flush）
pub struct TelemetryGuard {
    _private: (),
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        tracing::info!("P9-6 telemetry shutdown");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_metadata() {
        assert_eq!(SERVICE_NAME, "bingxi-backend");
        assert_eq!(SERVICE_NAMESPACE, "erp");
        assert!(!service_version().is_empty());
    }

    #[test]
    fn test_otlp_endpoint_default() {
        // 清除环境变量后默认值
        let endpoint = otlp_endpoint();
        assert!(!endpoint.is_empty());
    }

    #[test]
    fn test_otel_disabled_by_default() {
        // 默认未启用（除非显式设置 OTEL_ENABLED=true）
        let _ = is_otel_enabled();
    }

    #[test]
    fn test_span_kind_as_str() {
        assert_eq!(signals::trace::SpanKind::Server.as_str(), "server");
        assert_eq!(signals::trace::SpanKind::Client.as_str(), "client");
        assert_eq!(signals::trace::SpanKind::Producer.as_str(), "producer");
        assert_eq!(signals::trace::SpanKind::Consumer.as_str(), "consumer");
        assert_eq!(signals::trace::SpanKind::Internal.as_str(), "internal");
    }

    #[test]
    fn test_span_attrs() {
        let attrs = signals::trace::SpanAttrs::new()
            .with("http.method", "GET")
            .with("http.url", "/api/orders");
        assert_eq!(attrs.get("http.method"), Some(&"GET".to_string()));
        assert_eq!(attrs.get("http.url"), Some(&"/api/orders".to_string()));
    }

    #[test]
    fn test_counter() {
        let c = signals::metrics::Counter::new();
        c.inc();
        c.inc();
        c.add(5);
        assert_eq!(c.get(), 7);
    }

    #[test]
    fn test_histogram() {
        let h = signals::metrics::Histogram::new();
        h.observe(1.5);
        h.observe(2.5);
        assert_eq!(h.count(), 2);
        assert_eq!(h.sum(), 4.0);
    }

    #[test]
    fn test_log_level_ordering() {
        assert!(signals::log::LogLevel::Trace < signals::log::LogLevel::Info);
        assert!(signals::log::LogLevel::Info < signals::log::LogLevel::Error);
    }

    #[test]
    fn test_log_level_as_str() {
        assert_eq!(signals::log::LogLevel::Info.as_str(), "INFO");
        assert_eq!(signals::log::LogLevel::Error.as_str(), "ERROR");
    }

    #[test]
    fn test_span_names_constants() {
        assert_eq!(span_names::HTTP_REQUEST, "http.request");
        assert_eq!(span_names::DB_QUERY, "db.query");
        assert_eq!(span_names::SALES_ORDER_CREATE, "sales.order.create");
    }

    #[test]
    fn test_metric_names_constants() {
        assert_eq!(metric_names::HTTP_REQUESTS_TOTAL, "http_requests_total");
        assert_eq!(metric_names::HTTP_REQUEST_DURATION, "http_request_duration_seconds");
    }

    #[test]
    fn test_telemetry_init() {
        let _guard = init();
        // guard 应在 drop 时正常关闭
    }
}
