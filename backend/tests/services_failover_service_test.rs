use bingxi_backend::services::failover_service::*;


#[test]
fn test_metrics_creation() {
    let m = FailoverMetrics::new();
    assert!(m.is_ok());
}

// 死代码清理（2026-06-26）：test_format_failover_error 测试的 format_failover_error 已删除。