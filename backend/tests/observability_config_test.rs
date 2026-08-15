use bingxi_backend::observability::config::*;

#[test]
fn test_default_config() {
    let cfg = ObservabilityConfig::default();
    assert_eq!(cfg.service_name, "bingxi-backend");
    assert!(cfg.trace_enabled);
    assert!(cfg.metrics_enabled);
    assert_eq!(cfg.sample_ratio, 1.0);
    // from_env 生产环境默认 10% 采样率
    let env_cfg = ObservabilityConfig::from_env();
    assert!(env_cfg.sample_ratio <= 1.0);
}

#[test]
fn test_from_env() {
    let cfg = ObservabilityConfig::from_env();
    // 至少 service_name 应有默认值
    assert!(!cfg.service_name.is_empty());
}

#[test]
fn test_resource_attrs() {
    let cfg = ObservabilityConfig::default();
    let attrs = ResourceAttrs::from_config(&cfg);
    assert_eq!(attrs.service_namespace, "erp");
    assert_eq!(attrs.service_name, "bingxi-backend");
}
