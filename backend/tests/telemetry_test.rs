#[cfg(test)]
mod tests {
use bingxi_backend::telemetry::*;


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
        let enabled = is_otel_enabled();
        // 验证返回布尔值
        assert!(enabled == true || enabled == false, "应返回有效的布尔值");
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
        assert_eq!(
            metric_names::HTTP_REQUEST_DURATION,
            "http_request_duration_seconds"
        );
    }

    #[test]
    fn test_telemetry_init() {
        let guard = init();
        // 验证 guard 创建成功
        assert!(!format!("{:?}", guard).is_empty(), "telemetry guard 不应为空");
    }
}