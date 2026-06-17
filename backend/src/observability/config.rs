//! P9-6 OpenTelemetry 一体化导出部署（Jaeger + OTLP Collector）
//!
//! 包含：
//! 1. **Jaeger**：开源分布式追踪系统（all-in-one 模式）
//! 2. **OTLP Collector**：OpenTelemetry 协议收集器
//! 3. **Prometheus**：指标采集（与 OTel 兼容）
//! 4. **Grafana**：可视化面板
//!
//! ## 启动
//!
//! ```bash
//! cd deploy/observability
//! docker-compose up -d
//! ```
//!
//! ## 访问
//!
//! - Jaeger UI: http://localhost:16686
//! - Prometheus: http://localhost:9090
//! - Grafana: http://localhost:3000
//!
//! ## 后端环境变量
//!
//! ```bash
//! export OTEL_ENABLED=true
//! export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
//! ```

use serde::{Deserialize, Serialize};

/// 可观测性配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// 服务名
    pub service_name: String,
    /// 服务版本
    pub service_version: String,
    /// 部署环境（dev / staging / prod）
    pub environment: String,
    /// OTLP gRPC 端点
    pub otlp_endpoint: String,
    /// 是否启用追踪
    pub trace_enabled: bool,
    /// 是否启用指标
    pub metrics_enabled: bool,
    /// 追踪采样率（0.0 - 1.0）
    pub sample_ratio: f64,
    /// 导出间隔（秒）
    pub export_interval_secs: u64,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            service_name: "bingxi-backend".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            environment: "dev".to_string(),
            otlp_endpoint: "http://localhost:4317".to_string(),
            trace_enabled: true,
            metrics_enabled: true,
            sample_ratio: 1.0,
            export_interval_secs: 30,
        }
    }
}

impl ObservabilityConfig {
    /// 从环境变量加载
    pub fn from_env() -> Self {
        Self {
            service_name: std::env::var("OTEL_SERVICE_NAME")
                .unwrap_or_else(|_| "bingxi-backend".to_string()),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            environment: std::env::var("ENV").unwrap_or_else(|_| "dev".to_string()),
            otlp_endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:4317".to_string()),
            trace_enabled: std::env::var("OTEL_TRACE_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            metrics_enabled: std::env::var("OTEL_METRICS_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            sample_ratio: std::env::var("OTEL_SAMPLE_RATIO")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1.0),
            export_interval_secs: std::env::var("OTEL_EXPORT_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
        }
    }
}

/// 资源属性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAttrs {
    pub service_name: String,
    pub service_namespace: String,
    pub service_version: String,
    pub deployment_environment: String,
    pub tenant_id: Option<String>,
}

impl ResourceAttrs {
    pub fn from_config(cfg: &ObservabilityConfig) -> Self {
        Self {
            service_name: cfg.service_name.clone(),
            service_namespace: "erp".to_string(),
            service_version: cfg.service_version.clone(),
            deployment_environment: cfg.environment.clone(),
            tenant_id: std::env::var("TENANT_ID").ok(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = ObservabilityConfig::default();
        assert_eq!(cfg.service_name, "bingxi-backend");
        assert!(cfg.trace_enabled);
        assert!(cfg.metrics_enabled);
        assert_eq!(cfg.sample_ratio, 1.0);
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
}
