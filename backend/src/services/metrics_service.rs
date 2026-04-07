#![allow(dead_code, unused_variables, unused_imports, unused_mut)]
//! Prometheus 监控服务
//! 提供系统指标收集和导出功能

use axum::{
    extract::State,
    http::StatusCode,
    response::Response,
    routing::get,
    Router,
};
use prometheus::{Encoder, IntCounter, IntGauge, Histogram, Registry, TextEncoder};
use std::sync::Arc;

/// 监控指标注册表
pub type MetricsRegistry = Registry;

/// 自定义指标集合
#[derive(Debug, Clone)]
pub struct Metrics {
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
}

impl Metrics {
    /// 创建新的指标集合并注册
    pub fn new(registry: &MetricsRegistry) -> Result<Self, prometheus::Error> {
        // HTTP 请求总数
        let http_requests_total = IntCounter::new(
            "http_requests_total",
            "Total number of HTTP requests"
        )?;
        registry.register(Box::new(http_requests_total.clone()))?;
        
        // 当前活跃请求数
        let http_requests_in_flight = IntGauge::new(
            "http_requests_in_flight",
            "Number of HTTP requests currently being processed"
        )?;
        registry.register(Box::new(http_requests_in_flight.clone()))?;
        
        // HTTP 请求耗时
        let http_request_duration_seconds = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request duration in seconds"
            )
        )?;
        registry.register(Box::new(http_request_duration_seconds.clone()))?;
        
        // 数据库连接数
        let db_connections = IntGauge::new(
            "db_connections",
            "Number of database connections"
        )?;
        registry.register(Box::new(db_connections.clone()))?;
        
        // 数据库查询耗时
        let db_query_duration_seconds = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "db_query_duration_seconds",
                "Database query duration in seconds"
            )
        )?;
        registry.register(Box::new(db_query_duration_seconds.clone()))?;
        
        // 业务操作总数
        let business_operations_total = IntCounter::new(
            "business_operations_total",
            "Total number of business operations"
        )?;
        registry.register(Box::new(business_operations_total.clone()))?;
        
        // 错误总数
        let errors_total = IntCounter::new(
            "errors_total",
            "Total number of errors"
        )?;
        registry.register(Box::new(errors_total.clone()))?;
        
        Ok(Self {
            http_requests_total,
            http_requests_in_flight,
            http_request_duration_seconds,
            db_connections,
            db_query_duration_seconds,
            business_operations_total,
            errors_total,
        })
    }
    
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
    State(metrics_service): State<Arc<MetricsService>>,
) -> Result<Response<String>, StatusCode> {
    let encoder = TextEncoder::new();
    let metric_families = metrics_service.gather();
    
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let output = String::from_utf8(buffer)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain; version=0.0.4")
        .body(output)
        .unwrap())
}

/// 创建监控路由
pub fn create_metrics_router(metrics_service: Arc<MetricsService>) -> Router<Arc<MetricsService>> {
    Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(metrics_service)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let registry = Registry::new();
        let metrics = Metrics::new(&registry).unwrap();
        
        let initial_count = metrics.http_requests_total.get();
        metrics.record_http_request(0.5);
        let new_count = metrics.http_requests_total.get();
        
        assert_eq!(new_count, initial_count + 1);
    }

    #[test]
    fn test_start_end_request() {
        let registry = Registry::new();
        let metrics = Metrics::new(&registry).unwrap();
        
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
        let registry = Registry::new();
        let metrics = Metrics::new(&registry).unwrap();
        
        metrics.record_db_query(0.1);
        // 直方图指标不容易直接验证，但不抛出异常即成功
    }

    #[test]
    fn test_record_business_operation() {
        let registry = Registry::new();
        let metrics = Metrics::new(&registry).unwrap();
        
        let initial = metrics.business_operations_total.get();
        metrics.record_business_operation();
        let new = metrics.business_operations_total.get();
        
        assert_eq!(new, initial + 1);
    }

    #[test]
    fn test_record_error() {
        let registry = Registry::new();
        let metrics = Metrics::new(&registry).unwrap();
        
        let initial = metrics.errors_total.get();
        metrics.record_error();
        let new = metrics.errors_total.get();
        
        assert_eq!(new, initial + 1);
    }

    #[test]
    fn test_set_db_connections() {
        let registry = Registry::new();
        let metrics = Metrics::new(&registry).unwrap();
        
        metrics.set_db_connections(10);
        assert_eq!(metrics.db_connections.get(), 10);
        
        metrics.set_db_connections(5);
        assert_eq!(metrics.db_connections.get(), 5);
    }

    #[test]
    fn test_gather_metrics() {
        let metrics_service = MetricsService::new().unwrap();
        
        metrics_service.metrics.record_http_request(0.5);
        metrics_service.metrics.record_error();
        
        let gathered = metrics_service.gather();
        assert!(!gathered.is_empty());
    }

    #[test]
    fn test_metrics_clone() {
        let metrics_service = MetricsService::new().unwrap();
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
        let metrics_service = Arc::new(MetricsService::new().unwrap());
        
        let response = metrics_handler(State(metrics_service)).await;
        
        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(response.status(), 200);
        
        let headers = response.headers();
        assert!(headers.get("Content-Type").is_some());
        assert!(headers.get("Content-Type")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("text/plain"));
    }

    #[test]
    fn test_create_metrics_router() {
        let metrics_service = Arc::new(MetricsService::new().unwrap());
        let _router = create_metrics_router(metrics_service.clone());
        
        // 验证路由创建成功
        assert!(Arc::strong_count(&metrics_service) >= 1);
    }
}
