// 集成测试：FailoverMetrics 监控指标

use bingxi_backend::services::failover_service::FailoverMetrics;

#[test]
fn test_metrics_creation() {
    let m = FailoverMetrics::new();
    assert!(m.is_ok());
}

#[test]
fn test_metrics_record_methods() {
    let metrics = FailoverMetrics::default();
    metrics.record_primary("database");
    metrics.record_primary_failed("database");
    metrics.record_backup("database");
    metrics.record_switch("database");
    metrics.set_circuit_state("database", 1);

    // 验证所有记录方法执行成功
    let text = metrics.export_text().unwrap();
    assert!(text.contains("failover_primary_total"), "应包含 primary 指标");
    assert!(text.contains("failover_circuit_state"), "应包含 circuit_state 指标");
}

#[test]
fn test_metrics_export_text() {
    let metrics = FailoverMetrics::default();
    metrics.record_primary("database");
    let result = metrics.export_text();
    assert!(result.is_ok());
    let text = result.unwrap();
    assert!(text.contains("failover_primary_total"));
    assert!(text.contains("failover_circuit_state"));
}

#[test]
fn test_metrics_multiple_functions() {
    let metrics = FailoverMetrics::default();
    metrics.record_primary("database");
    metrics.record_primary("cache");
    metrics.record_switch("database");
    metrics.record_switch("cache");
    let text = metrics.export_text().unwrap();
    // 应该包含两个 function 的指标
    assert!(text.contains("function=\"database\""));
    assert!(text.contains("function=\"cache\""));
}
