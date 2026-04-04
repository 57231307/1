//! 系统告警服务
//! 监控系统关键指标并在达到阈值时发送告警

use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info, warn};

use crate::services::metrics_service::MetricsService;

/// 告警配置
#[derive(Debug, Clone)]
pub struct AlertConfig {
    /// HTTP 请求错误率阈值（百分比）
    pub http_error_rate_threshold: f64,
    /// 数据库查询耗时阈值（秒）
    pub db_query_duration_threshold: f64,
    /// 系统负载阈值
    pub system_load_threshold: f64,
    /// 内存使用阈值（百分比）
    pub memory_usage_threshold: f64,
    /// 告警检查间隔（秒）
    pub check_interval: u64,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            http_error_rate_threshold: 5.0, // 5%
            db_query_duration_threshold: 1.0, // 1秒
            system_load_threshold: 8.0, // 系统负载
            memory_usage_threshold: 80.0, // 80%
            check_interval: 60, // 60秒
        }
    }
}

/// 告警级别
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// 告警信息
#[derive(Debug, Clone)]
pub struct Alert {
    pub level: AlertLevel,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metric_name: String,
    pub current_value: f64,
    pub threshold: f64,
}

/// 告警服务
#[derive(Debug, Clone)]
pub struct AlertService {
    metrics: Arc<MetricsService>,
    config: AlertConfig,
    alerts: Arc<tokio::sync::Mutex<Vec<Alert>>>,
}

impl AlertService {
    /// 创建新的告警服务
    pub fn new(metrics: Arc<MetricsService>, config: Option<AlertConfig>) -> Self {
        Self {
            metrics,
            config: config.unwrap_or_default(),
            alerts: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    /// 启动告警监控
    pub async fn start_monitoring(&self) {
        let interval = interval(Duration::from_secs(self.config.check_interval));
        let self_clone = self.clone();

        tokio::spawn(async move {
            info!("告警监控服务启动，检查间隔：{}秒", self_clone.config.check_interval);

            loop {
                interval.tick().await;
                self_clone.check_metrics().await;
            }
        });
    }

    /// 检查指标并生成告警
    async fn check_metrics(&self) {
        // 检查HTTP错误率
        self.check_http_error_rate().await;

        // 检查数据库查询耗时
        self.check_db_query_duration().await;

        // 检查系统指标
        self.check_system_metrics().await;
    }

    /// 检查HTTP错误率
    async fn check_http_error_rate(&self) {
        // 这里需要从metrics中获取HTTP错误率
        // 由于我们的metrics服务目前没有直接提供错误率，我们可以假设一个值来演示
        let error_rate = 3.5; // 模拟值

        if error_rate > self.config.http_error_rate_threshold {
            let alert = Alert {
                level: AlertLevel::Warning,
                message: format!("HTTP错误率超过阈值: {:.2}%", error_rate),
                timestamp: chrono::Utc::now(),
                metric_name: "http_error_rate",
                current_value: error_rate,
                threshold: self.config.http_error_rate_threshold,
            };
            self.generate_alert(alert).await;
        }
    }

    /// 检查数据库查询耗时
    async fn check_db_query_duration(&self) {
        // 这里需要从metrics中获取数据库查询耗时
        // 由于我们的metrics服务目前没有直接提供平均查询耗时，我们可以假设一个值来演示
        let avg_duration = 0.8; // 模拟值（秒）

        if avg_duration > self.config.db_query_duration_threshold {
            let alert = Alert {
                level: AlertLevel::Warning,
                message: format!("数据库查询耗时超过阈值: {:.2}秒", avg_duration),
                timestamp: chrono::Utc::now(),
                metric_name: "db_query_duration",
                current_value: avg_duration,
                threshold: self.config.db_query_duration_threshold,
            };
            self.generate_alert(alert).await;
        }
    }

    /// 检查系统指标
    async fn check_system_metrics(&self) {
        // 检查系统负载
        if let Ok(load) = get_system_load() {
            if load > self.config.system_load_threshold {
                let alert = Alert {
                    level: AlertLevel::Error,
                    message: format!("系统负载超过阈值: {:.2}", load),
                    timestamp: chrono::Utc::now(),
                    metric_name: "system_load",
                    current_value: load,
                    threshold: self.config.system_load_threshold,
                };
                self.generate_alert(alert).await;
            }
        }

        // 检查内存使用
        if let Ok(memory_usage) = get_memory_usage() {
            if memory_usage > self.config.memory_usage_threshold {
                let alert = Alert {
                    level: AlertLevel::Error,
                    message: format!("内存使用超过阈值: {:.2}%", memory_usage),
                    timestamp: chrono::Utc::now(),
                    metric_name: "memory_usage",
                    current_value: memory_usage,
                    threshold: self.config.memory_usage_threshold,
                };
                self.generate_alert(alert).await;
            }
        }
    }

    /// 生成告警
    async fn generate_alert(&self, alert: Alert) {
        // 保存告警
        let mut alerts = self.alerts.lock().await;
        alerts.push(alert.clone());
        // 只保留最近100条告警
        if alerts.len() > 100 {
            alerts.drain(0..alerts.len() - 100);
        }

        // 记录告警
        match alert.level {
            AlertLevel::Info => info!("{}", alert.message),
            AlertLevel::Warning => warn!("{}", alert.message),
            AlertLevel::Error | AlertLevel::Critical => error!("{}", alert.message),
        }

        // 这里可以添加告警通知逻辑，比如发送邮件、短信等
        // 目前我们只是记录日志
    }

    /// 获取最近的告警
    pub async fn get_recent_alerts(&self, limit: usize) -> Vec<Alert> {
        let alerts = self.alerts.lock().await;
        let start = if alerts.len() > limit {
            alerts.len() - limit
        } else {
            0
        };
        alerts[start..].to_vec()
    }

    /// 获取告警配置
    pub fn get_config(&self) -> AlertConfig {
        self.config.clone()
    }

    /// 更新告警配置
    pub fn update_config(&mut self, config: AlertConfig) {
        self.config = config;
    }
}

/// 获取系统负载
fn get_system_load() -> Result<f64, std::io::Error> {
    // 在真实环境中，这里应该读取系统负载
    // 现在我们返回一个模拟值
    Ok(4.5)
}

/// 获取内存使用百分比
fn get_memory_usage() -> Result<f64, std::io::Error> {
    // 在真实环境中，这里应该读取内存使用情况
    // 现在我们返回一个模拟值
    Ok(65.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::metrics_service::MetricsService;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_alert_service_creation() {
        let metrics_service = Arc::new(MetricsService::new().unwrap());
        let alert_service = AlertService::new(metrics_service, None);
        assert_eq!(alert_service.config.http_error_rate_threshold, 5.0);
    }

    #[tokio::test]
    async fn test_alert_generation() {
        let metrics_service = Arc::new(MetricsService::new().unwrap());
        let alert_service = AlertService::new(metrics_service, None);

        let alert = Alert {
            level: AlertLevel::Warning,
            message: "Test alert".to_string(),
            timestamp: chrono::Utc::now(),
            metric_name: "test_metric".to_string(),
            current_value: 10.0,
            threshold: 5.0,
        };

        alert_service.generate_alert(alert).await;

        let alerts = alert_service.get_recent_alerts(10).await;
        assert!(!alerts.is_empty());
    }

    #[tokio::test]
    async fn test_get_recent_alerts() {
        let metrics_service = Arc::new(MetricsService::new().unwrap());
        let alert_service = AlertService::new(metrics_service, None);

        // 添加多个告警
        for i in 0..5 {
            let alert = Alert {
                level: AlertLevel::Info,
                message: format!("Test alert {}", i).to_string(),
                timestamp: chrono::Utc::now(),
                metric_name: "test_metric".to_string(),
                current_value: i as f64,
                threshold: 5.0,
            };
            alert_service.generate_alert(alert).await;
        }

        let alerts = alert_service.get_recent_alerts(3).await;
        assert_eq!(alerts.len(), 3);
    }

    #[tokio::test]
    async fn test_update_config() {
        let metrics_service = Arc::new(MetricsService::new().unwrap());
        let mut alert_service = AlertService::new(metrics_service, None);

        let new_config = AlertConfig {
            http_error_rate_threshold: 10.0,
            db_query_duration_threshold: 2.0,
            system_load_threshold: 10.0,
            memory_usage_threshold: 90.0,
            check_interval: 30,
        };

        alert_service.update_config(new_config);
        assert_eq!(alert_service.config.http_error_rate_threshold, 10.0);
        assert_eq!(alert_service.config.check_interval, 30);
    }
}
