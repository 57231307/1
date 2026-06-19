//! Prometheus 监控服务
//!
//! 提供系统指标收集和导出功能。
//!
//! ## 指标分层
//!
//! - **基础指标**（无标签）：历史兼容，老的 PromQL 表达式继续生效
//!   - `http_requests_total` / `http_requests_in_flight` / `http_request_duration_seconds`
//!   - `db_connections` / `db_query_duration_seconds`
//!   - `business_operations_total` / `errors_total`
//!
//! - **带标签指标**（per-route / per-status）：
//!   - `http_requests_by_route{method,route,status}`  每条路由每状态码的请求计数
//!   - `http_request_duration_by_route{method,route}` 每条路由的请求耗时直方图
//!   - `http_requests_by_status_class{class}`        2xx/3xx/4xx/5xx 请求数
//!   - `business_operations_by_type{operation}`      业务操作按类型计数
//!
//! ## 中间件
//!
//! `middleware::metrics::metrics_middleware` 会自动按 method/route/status 记录带标签指标。
//! 调用方通常只需要 `state.metrics.metrics.record_business_operation("create_user")` 即可。

use axum::{extract::State, http::StatusCode, response::Response, routing::get, Router};
use prometheus::{
    Encoder, Histogram, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, Opts,
    Registry, TextEncoder,
};
use std::sync::Arc;

/// 监控指标注册表
pub type MetricsRegistry = Registry;

/// 状态码分类（用于 `http_requests_by_status_class`）
#[derive(Debug, Clone, Copy)]
pub enum StatusClass {
    /// 1xx 信息响应
    Informational,
    /// 2xx 成功
    Success,
    /// 3xx 重定向
    Redirection,
    /// 4xx 客户端错误
    ClientError,
    /// 5xx 服务端错误
    ServerError,
}

impl StatusClass {
    /// 解析 HTTP 状态码到分类
    pub fn from_status(status: StatusCode) -> Self {
        let code = status.as_u16();
        match code {
            100..=199 => Self::Informational,
            200..=299 => Self::Success,
            300..=399 => Self::Redirection,
            400..=499 => Self::ClientError,
            500..=599 => Self::ServerError,
            _ => Self::ServerError,
        }
    }

    /// 分类的 PromQL 标签值
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Informational => "1xx",
            Self::Success => "2xx",
            Self::Redirection => "3xx",
            Self::ClientError => "4xx",
            Self::ServerError => "5xx",
        }
    }
}

/// 自定义指标集合
#[derive(Debug, Clone)]
pub struct Metrics {
    // ===== 基础指标（无标签，保持向后兼容） =====
    /// HTTP 请求总数
    pub http_requests_total: IntCounter,
    /// 当前活跃请求数
    pub http_requests_in_flight: IntGauge,
    /// HTTP 请求耗时（秒）
    pub http_request_duration_seconds: Histogram,
    /// 数据库连接数
    pub db_connections: IntGauge,
    /// 数据库查询耗时（秒）
    pub db_query_duration_seconds: Histogram,
    /// 业务操作总数
    pub business_operations_total: IntCounter,
    /// 错误总数
    pub errors_total: IntCounter,

    // ===== 带标签指标（P3.2 新增） =====
    /// 按 method / route / status 分类的请求计数
    pub http_requests_by_route: IntCounterVec,
    /// 按 method / route 分类的请求耗时直方图
    pub http_request_duration_by_route: HistogramVec,
    /// 按状态码分类（2xx/3xx/4xx/5xx）的请求计数
    pub http_requests_by_status_class: IntCounterVec,
    /// 按业务操作类型分类的请求计数
    pub business_operations_by_type: IntCounterVec,
}

impl Metrics {
    /// 创建新的指标集合并注册
    pub fn new(registry: &MetricsRegistry) -> Result<Self, prometheus::Error> {
        // ===== 基础指标注册（保持原行为） =====
        let http_requests_total =
            IntCounter::new("http_requests_total", "Total number of HTTP requests")?;
        registry.register(Box::new(http_requests_total.clone()))?;

        let http_requests_in_flight = IntGauge::new(
            "http_requests_in_flight",
            "Number of HTTP requests currently being processed",
        )?;
        registry.register(Box::new(http_requests_in_flight.clone()))?;

        let http_request_duration_seconds = Histogram::with_opts(HistogramOpts::new(
            "http_request_duration_seconds",
            "HTTP request duration in seconds",
        ))?;
        registry.register(Box::new(http_request_duration_seconds.clone()))?;

        let db_connections = IntGauge::new("db_connections", "Number of database connections")?;
        registry.register(Box::new(db_connections.clone()))?;

        let db_query_duration_seconds = Histogram::with_opts(HistogramOpts::new(
            "db_query_duration_seconds",
            "Database query duration in seconds",
        ))?;
        registry.register(Box::new(db_query_duration_seconds.clone()))?;

        let business_operations_total = IntCounter::new(
            "business_operations_total",
            "Total number of business operations",
        )?;
        registry.register(Box::new(business_operations_total.clone()))?;

        let errors_total = IntCounter::new("errors_total", "Total number of errors")?;
        registry.register(Box::new(errors_total.clone()))?;

        // ===== 带标签指标注册（P3.2 新增） =====

        // per-route 请求计数
        let http_requests_by_route = IntCounterVec::new(
            Opts::new(
                "http_requests_by_route",
                "Total number of HTTP requests labelled by method, route and status code",
            ),
            &["method", "route", "status"],
        )?;
        registry.register(Box::new(http_requests_by_route.clone()))?;

        // per-route 请求耗时直方图
        let http_request_duration_by_route = HistogramVec::new(
            HistogramOpts::new(
                "http_request_duration_by_route",
                "HTTP request duration in seconds labelled by method and route",
            ),
            &["method", "route"],
        )?;
        registry.register(Box::new(http_request_duration_by_route.clone()))?;

        // 状态码分类计数
        let http_requests_by_status_class = IntCounterVec::new(
            Opts::new(
                "http_requests_by_status_class",
                "Total number of HTTP requests grouped by status code class (1xx/2xx/3xx/4xx/5xx)",
            ),
            &["class"],
        )?;
        registry.register(Box::new(http_requests_by_status_class.clone()))?;

        // 业务操作按类型计数
        let business_operations_by_type = IntCounterVec::new(
            Opts::new(
                "business_operations_by_type",
                "Total number of business operations grouped by operation name",
            ),
            &["operation"],
        )?;
        registry.register(Box::new(business_operations_by_type.clone()))?;

        Ok(Self {
            http_requests_total,
            http_requests_in_flight,
            http_request_duration_seconds,
            db_connections,
            db_query_duration_seconds,
            business_operations_total,
            errors_total,
            http_requests_by_route,
            http_request_duration_by_route,
            http_requests_by_status_class,
            business_operations_by_type,
        })
    }

    // ===== 基础操作（保持向后兼容） =====

    /// 记录 HTTP 请求
    pub fn record_http_request(&self, duration_secs: f64) {
        self.http_requests_total.inc();
        self.http_request_duration_seconds.observe(duration_secs);
    }

    /// 开始处理请求
    pub fn start_request(&self) {
        self.http_requests_in_flight.inc();
    }

    /// 结束处理请求
    pub fn end_request(&self) {
        self.http_requests_in_flight.dec();
    }

    /// 记录数据库查询
    pub fn record_db_query(&self, duration_secs: f64) {
        self.db_query_duration_seconds.observe(duration_secs);
    }

    /// 记录业务操作
    pub fn record_business_operation(&self) {
        self.business_operations_total.inc();
    }

    /// 记录错误
    pub fn record_error(&self) {
        self.errors_total.inc();
    }

    /// 设置数据库连接数
    pub fn set_db_connections(&self, count: i64) {
        self.db_connections.set(count);
    }

    /// 记录慢查询（超过阈值时记录日志）
    pub fn record_slow_query(&self, duration_secs: f64, query_name: &str) {
        const SLOW_QUERY_THRESHOLD: f64 = 1.0; // 1 秒

        self.db_query_duration_seconds.observe(duration_secs);

        if duration_secs > SLOW_QUERY_THRESHOLD {
            tracing::warn!(
                "慢查询检测: {} 耗时 {:.3}s (阈值: {:.1}s)",
                query_name,
                duration_secs,
                SLOW_QUERY_THRESHOLD
            );
        }
    }

    /// 记录慢请求（超过阈值时记录日志）
    pub fn record_slow_request(&self, duration_secs: f64, path: &str, method: &str) {
        const SLOW_REQUEST_THRESHOLD: f64 = 2.0; // 2 秒

        self.http_request_duration_seconds.observe(duration_secs);

        if duration_secs > SLOW_REQUEST_THRESHOLD {
            tracing::warn!(
                "慢请求检测: {} {} 耗时 {:.3}s (阈值: {:.1}s)",
                method,
                path,
                duration_secs,
                SLOW_REQUEST_THRESHOLD
            );
        }
    }

    // ===== 带标签操作（P3.2 新增） =====

    /// 按 method / route / status 记录请求
    ///
    /// 典型调用方：`middleware::metrics::metrics_middleware` 在响应后自动调用
    pub fn record_http_by_route(&self, method: &str, route: &str, status: StatusCode) {
        let status_str = status.as_u16().to_string();
        let class = StatusClass::from_status(status);

        self.http_requests_by_route
            .with_label_values(&[method, route, &status_str])
            .inc();
        self.http_requests_by_status_class
            .with_label_values(&[class.as_str()])
            .inc();
    }

    /// 按 method / route 记录请求耗时
    pub fn record_http_duration_by_route(&self, method: &str, route: &str, duration_secs: f64) {
        self.http_request_duration_by_route
            .with_label_values(&[method, route])
            .observe(duration_secs);
    }

    /// 按业务操作类型记录（如 `"create_user"` / `"approve_order"`）
    ///
    /// 调用方：`self.record_business_operation_by_type("create_user")`
    pub fn record_business_operation_by_type(&self, operation: &str) {
        self.business_operations_total.inc();
        self.business_operations_by_type
            .with_label_values(&[operation])
            .inc();
    }
}

/// 监控服务
#[derive(Debug, Clone)]
pub struct MetricsService {
    pub registry: Arc<MetricsRegistry>,
    pub metrics: Arc<Metrics>,
}

impl MetricsService {
    /// 创建新的监控服务
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();
        let metrics = Metrics::new(&registry)?;

        Ok(Self {
            registry: Arc::new(registry),
            metrics: Arc::new(metrics),
        })
    }

    /// 获取指标数据
    pub fn gather(&self) -> Vec<prometheus::proto::MetricFamily> {
        self.registry.gather()
    }
}

/// 导出 Prometheus 格式的指标
pub async fn metrics_handler(
    State(state): State<crate::utils::app_state::AppState>,
) -> Result<Response<String>, StatusCode> {
    let encoder = TextEncoder::new();
    let metric_families = state.metrics.gather();

    let mut buffer = Vec::new();
    encoder
        .encode(&metric_families, &mut buffer)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let output = String::from_utf8(buffer).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain; version=0.0.4")
        .body(output)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// 创建监控路由
pub fn create_metrics_router() -> Router<crate::utils::app_state::AppState> {
    Router::new().route("/metrics", get(metrics_handler))
}

#[cfg(test)]
mod tests {
    use super::*;

    // P9-1: 测试夹具 helper，封装 MetricsService 的常见初始化模式
    fn test_metrics_service() -> MetricsService {
        MetricsService::new().expect("P9-1: 测试夹具 MetricsService 初始化失败")
    }

    fn test_metrics() -> Metrics {
        let registry = Registry::new();
        Metrics::new(&registry).expect("P9-1: 测试夹具 Metrics 初始化失败")
    }

    #[test]
    fn test_metrics_service_creation() {
        let result = MetricsService::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_metrics_creation() {
        let registry = Registry::new();
        let result = Metrics::new(&registry);
        assert!(result.is_ok());
    }

    #[test]
    fn test_record_http_request() {
        let metrics = test_metrics();

        let initial_count = metrics.http_requests_total.get();
        metrics.record_http_request(0.5);
        let new_count = metrics.http_requests_total.get();

        assert_eq!(new_count, initial_count + 1);
    }

    #[test]
    fn test_start_end_request() {
        let metrics = test_metrics();

        let initial = metrics.http_requests_in_flight.get();
        metrics.start_request();
        let during = metrics.http_requests_in_flight.get();
        metrics.end_request();
        let after = metrics.http_requests_in_flight.get();

        assert_eq!(during, initial + 1);
        assert_eq!(after, initial);
    }

    #[test]
    fn test_record_db_query() {
        let metrics = test_metrics();

        metrics.record_db_query(0.1);
        // 直方图指标不容易直接验证，但不抛出异常即成功
    }

    #[test]
    fn test_record_business_operation() {
        let metrics = test_metrics();

        let initial = metrics.business_operations_total.get();
        metrics.record_business_operation();
        let new = metrics.business_operations_total.get();

        assert_eq!(new, initial + 1);
    }

    #[test]
    fn test_record_error() {
        let metrics = test_metrics();

        let initial = metrics.errors_total.get();
        metrics.record_error();
        let new = metrics.errors_total.get();

        assert_eq!(new, initial + 1);
    }

    #[test]
    fn test_set_db_connections() {
        let metrics = test_metrics();

        metrics.set_db_connections(10);
        assert_eq!(metrics.db_connections.get(), 10);

        metrics.set_db_connections(5);
        assert_eq!(metrics.db_connections.get(), 5);
    }

    #[test]
    fn test_gather_metrics() {
        let metrics_service = test_metrics_service();

        metrics_service.metrics.record_http_request(0.5);
        metrics_service.metrics.record_error();

        let gathered = metrics_service.gather();
        assert!(!gathered.is_empty());
    }

    #[test]
    fn test_metrics_clone() {
        let metrics_service = test_metrics_service();
        let cloned = metrics_service.clone();

        assert_eq!(
            Arc::as_ptr(&metrics_service.registry),
            Arc::as_ptr(&cloned.registry)
        );
        assert_eq!(
            Arc::as_ptr(&metrics_service.metrics),
            Arc::as_ptr(&cloned.metrics)
        );
    }

    #[tokio::test]
    async fn test_metrics_handler() {
        let response = metrics_handler(State(crate::utils::app_state::AppState::default())).await;

        // P9-1: 用 match 处理 handler 返回的 Result
        let response = match response {
            Ok(r) => r,
            Err(e) => panic!("P9-1: metrics handler 返回错误: {e}"),
        };
        assert_eq!(response.status(), 200);

        let headers = response.headers();
        let content_type = match headers.get("Content-Type") {
            Some(v) => v,
            None => panic!("P9-1: Content-Type header 缺失"),
        };
        let content_type = content_type
            .to_str()
            .expect("P9-1: content-type 应为合法 ASCII");
        assert!(content_type.contains("text/plain"));
    }

    #[test]
    fn test_create_metrics_router() {
        let _router = create_metrics_router();

        // 路由创建成功即视为通过（无需再断言常量）
    }

    // ===== P3.2 新增指标测试 =====

    #[test]
    fn test_status_class_from_status() {
        assert!(matches!(
            StatusClass::from_status(StatusCode::OK),
            StatusClass::Success
        ));
        assert!(matches!(
            StatusClass::from_status(StatusCode::NOT_FOUND),
            StatusClass::ClientError
        ));
        assert!(matches!(
            StatusClass::from_status(StatusCode::INTERNAL_SERVER_ERROR),
            StatusClass::ServerError
        ));
        assert!(matches!(
            StatusClass::from_status(StatusCode::MOVED_PERMANENTLY),
            StatusClass::Redirection
        ));
    }

    #[test]
    fn test_record_http_by_route() {
        let metrics = test_metrics();

        metrics.record_http_by_route("GET", "/api/v1/erp/users", StatusCode::OK);
        metrics.record_http_by_route("GET", "/api/v1/erp/users", StatusCode::OK);
        metrics.record_http_by_route(
            "POST",
            "/api/v1/erp/users",
            StatusCode::INTERNAL_SERVER_ERROR,
        );

        // 验证计数器增加
        let count_2xx = metrics
            .http_requests_by_status_class
            .with_label_values(&["2xx"])
            .get();
        let count_5xx = metrics
            .http_requests_by_status_class
            .with_label_values(&["5xx"])
            .get();

        assert_eq!(count_2xx, 2);
        assert_eq!(count_5xx, 1);
    }

    #[test]
    fn test_record_http_duration_by_route() {
        let metrics = test_metrics();

        metrics.record_http_duration_by_route("GET", "/api/v1/erp/products", 0.123);
        // 直方图记录不会抛异常即视为成功
    }

    #[test]
    fn test_record_business_operation_by_type() {
        let metrics = test_metrics();

        let total_before = metrics.business_operations_total.get();
        metrics.record_business_operation_by_type("create_user");
        metrics.record_business_operation_by_type("create_user");
        metrics.record_business_operation_by_type("approve_order");

        // 总数 +3
        assert_eq!(metrics.business_operations_total.get(), total_before + 3);

        // 类型计数
        let create_user_count = metrics
            .business_operations_by_type
            .with_label_values(&["create_user"])
            .get();
        let approve_order_count = metrics
            .business_operations_by_type
            .with_label_values(&["approve_order"])
            .get();

        assert_eq!(create_user_count, 2);
        assert_eq!(approve_order_count, 1);
    }
}
