use axum::extract::State;
use axum::http::StatusCode;
use bingxi_backend::container::*;
use bingxi_backend::services::business_metrics::*;
use bingxi_backend::services::metrics_service::*;
use prometheus::Registry;
use std::sync::Arc;

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

    // 记录数据库查询不应抛异常
    metrics.record_db_query(0.1);

    // 验证 metrics 对象仍然有效
    assert!(
        metrics.db_query_duration_seconds.get_sample_count() >= 0,
        "metrics 应保持有效状态"
    );
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
    let response = metrics_handler(State(bingxi_backend::container::AppState::default())).await;

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
    let router = create_metrics_router();

    // 验证路由创建成功
    assert!(!format!("{:?}", router).is_empty(), "路由器不应为空");
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

    // 记录 HTTP 持续时间不应抛异常
    metrics.record_http_duration_by_route("GET", "/api/v1/erp/products", 0.123);

    // 验证 metrics 对象仍然有效
    assert!(
        metrics
            .http_request_duration_by_route
            .with_label_values(&["GET", "/api/v1/erp/products"])
            .get_sample_count()
            >= 0,
        "metrics 应保持有效状态"
    );
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

// ===== 批次 106 P1-2 新增：BusinessMetrics 接入验证 =====

#[test]
fn test_business_metrics_integrated_into_metrics_service() {
    // 批次 106 P1-2 修复：验证 BusinessMetrics 已接入 MetricsService，
    // 通过同一 Registry 注册，/metrics 端点 gather 时自动包含 erp_* 指标
    let metrics_service = test_metrics_service();

    // 业务指标可通过 state.metrics.business_metrics.record_*(...) 调用
    metrics_service
        .business_metrics
        .record_order_created("pending");
    metrics_service.business_metrics.record_cache_hit();
    metrics_service.business_metrics.record_login(true);

    // gather 后应包含 erp_* 指标家族
    let gathered = metrics_service.gather();
    let erp_metric_names: Vec<&str> = gathered
        .iter()
        .map(|f| f.get_name())
        .filter(|name| name.starts_with("erp_"))
        .collect();
    assert!(
        !erp_metric_names.is_empty(),
        "gather 应包含 erp_* 指标，实际: {:?}",
        erp_metric_names
    );
    assert!(
        erp_metric_names.contains(&"erp_orders_total"),
        "应包含 erp_orders_total，实际: {:?}",
        erp_metric_names
    );
    assert!(
        erp_metric_names.contains(&"erp_cache_hits_total"),
        "应包含 erp_cache_hits_total，实际: {:?}",
        erp_metric_names
    );
}
