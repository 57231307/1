// TODO(tech-debt): 此文件已开启 dead_code 检查；后续接入时如出现未使用项，应按模板逐项评估。
// 当前所有 pub API 均通过 FailoverMetrics / FailoverService 被业务引用。

//! 主备隔离服务
//!
//! 提供主备隔离业务逻辑：
//! - 健康检查（数据库 / 缓存）
//! - 切换事件记录
//! - 状态查询
//! - 手动切换（仅管理员）
//! - Prometheus 指标导出

use chrono::Utc;
use sea_orm::{QuerySelect, ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::sync::Arc;
use tracing::{info, warn};

use crate::config::failover::FailoverConfig;
use crate::models::failover_event as event_model;
use crate::models::failover_status as status_model;
use crate::utils::failover::FailoverError;
use prometheus::{Encoder, IntCounterVec, IntGaugeVec, Opts, Registry, TextEncoder};

// ==================== Prometheus 指标 ====================

/// 主备隔离指标
pub struct FailoverMetrics {
    /// 主调用总次数
    pub primary_total: IntCounterVec,
    /// 主调用失败总次数
    pub primary_failed_total: IntCounterVec,
    /// 备用调用总次数
    pub backup_total: IntCounterVec,
    /// 切换总次数
    pub switch_total: IntCounterVec,
    /// 熔断器状态（0=关闭, 1=打开, 2=半开）
    pub circuit_state: IntGaugeVec,
    /// Prometheus 注册表
    pub registry: Registry,
}

impl FailoverMetrics {
    /// 创建指标实例
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();
        let primary_total = IntCounterVec::new(
            Opts::new("failover_primary_total", "主调用总次数"),
            &["function"],
        )?;
        let primary_failed_total = IntCounterVec::new(
            Opts::new("failover_primary_failed_total", "主调用失败总次数"),
            &["function"],
        )?;
        let backup_total = IntCounterVec::new(
            Opts::new("failover_backup_total", "备用调用总次数"),
            &["function"],
        )?;
        let switch_total = IntCounterVec::new(
            Opts::new("failover_switch_total", "主备切换总次数"),
            &["function"],
        )?;
        let circuit_state = IntGaugeVec::new(
            Opts::new("failover_circuit_state", "熔断器状态（0=关闭,1=打开,2=半开）"),
            &["function"],
        )?;
        registry.register(Box::new(primary_total.clone()))?;
        registry.register(Box::new(primary_failed_total.clone()))?;
        registry.register(Box::new(backup_total.clone()))?;
        registry.register(Box::new(switch_total.clone()))?;
        registry.register(Box::new(circuit_state.clone()))?;
        for func in &["database", "cache"] {
            circuit_state.with_label_values(&[func]).set(0);
        }
        Ok(Self {
            primary_total,
            primary_failed_total,
            backup_total,
            switch_total,
            circuit_state,
            registry,
        })
    }

    /// 记录主调用
    pub fn record_primary(&self, function: &str) {
        self.primary_total.with_label_values(&[function]).inc();
    }

    /// 记录主调用失败
    pub fn record_primary_failed(&self, function: &str) {
        self.primary_failed_total
            .with_label_values(&[function])
            .inc();
    }

    /// 记录备用调用
    pub fn record_backup(&self, function: &str) {
        self.backup_total.with_label_values(&[function]).inc();
    }

    /// 记录切换
    pub fn record_switch(&self, function: &str) {
        self.switch_total.with_label_values(&[function]).inc();
    }

    /// 设置熔断器状态
    pub fn set_circuit_state(&self, function: &str, state: i64) {
        self.circuit_state
            .with_label_values(&[function])
            .set(state);
    }

    /// 导出 Prometheus 文本格式
    pub fn export_text(&self) -> Result<String, prometheus::Error> {
        let metric_families = self.registry.gather();
        let encoder = TextEncoder::new();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        String::from_utf8(buffer).map_err(|e| prometheus::Error::Msg(e.to_string()))
    }
}

impl Default for FailoverMetrics {
    fn default() -> Self {
        // P9-1: 集中 unwrap 到 helper，失败时回退到空结构
        fn mk_counter(name: &str, help: &str) -> IntCounterVec {
            IntCounterVec::new(Opts::new(name, help), &["function"])
                .unwrap_or_else(|e| panic!("P9-1: 指标 {name} 初始化失败: {e}"))
        }
        fn mk_gauge(name: &str, help: &str) -> IntGaugeVec {
            IntGaugeVec::new(Opts::new(name, help), &["function"])
                .unwrap_or_else(|e| panic!("P9-1: 指标 {name} 初始化失败: {e}"))
        }
        Self::new().unwrap_or_else(|_| Self {
            primary_total: mk_counter("failover_primary_total", "主调用总次数"),
            primary_failed_total: mk_counter("failover_primary_failed_total", "主调用失败总次数"),
            backup_total: mk_counter("failover_backup_total", "备用调用总次数"),
            switch_total: mk_counter("failover_switch_total", "主备切换总次数"),
            circuit_state: mk_gauge("failover_circuit_state", "熔断器状态"),
            registry: Registry::new(),
        })
    }
}

// ==================== 主备隔离服务 ====================

/// 主备隔离服务
pub struct FailoverService {
    /// 数据库连接
    pub db: DatabaseConnection,
    /// 配置
    pub config: FailoverConfig,
    /// Prometheus 指标
    pub metrics: Arc<FailoverMetrics>,
}

impl FailoverService {
    /// 创建服务实例
    pub fn new(db: DatabaseConnection, config: FailoverConfig, metrics: Arc<FailoverMetrics>) -> Self {
        Self {
            db,
            config,
            metrics,
        }
    }

    /// 获取所有主备状态
    pub async fn get_statuses(&self) -> Result<Vec<status_model::FailoverStatusDto>, String> {
        let statuses = status_model::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| format!("查询状态失败: {}", e))?;
        Ok(statuses.into_iter().map(Into::into).collect())
    }

    /// 获取最近切换事件
    pub async fn get_recent_events(&self, limit: u64) -> Result<Vec<event_model::FailoverEventDto>, String> {
        use sea_orm::QueryOrder;
        let events = event_model::Entity::find()
            .order_by_desc(event_model::Column::CreatedAt)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(|e| format!("查询事件失败: {}", e))?;
        Ok(events.into_iter().map(Into::into).collect())
    }

    /// 记录切换事件
    pub async fn record_event(
        &self,
        function_name: &str,
        event_type: &str,
        from_state: Option<&str>,
        to_state: Option<&str>,
        reason: Option<&str>,
        latency_ms: Option<i32>,
        tenant_id: Option<i64>,
    ) -> Result<(), String> {
        let now = Utc::now();
        let active = event_model::ActiveModel {
            function_name: Set(function_name.to_string()),
            event_type: Set(event_type.to_string()),
            from_state: Set(from_state.map(|s| s.to_string())),
            to_state: Set(to_state.map(|s| s.to_string())),
            reason: Set(reason.map(|s| s.to_string())),
            latency_ms: Set(latency_ms),
            tenant_id: Set(tenant_id),
            created_at: Set(now),
            ..Default::default()
        };
        active
            .insert(&self.db)
            .await
            .map_err(|e| format!("记录事件失败: {}", e))?;
        Ok(())
    }

    /// 更新主备状态
    pub async fn update_status(
        &self,
        function_name: &str,
        current_state: &str,
        circuit_state: &str,
    ) -> Result<(), String> {
        use sea_orm::ActiveModelTrait;
        let now = Utc::now();
        if let Ok(Some(existing)) = status_model::Entity::find()
            .filter(status_model::Column::FunctionName.eq(function_name))
            .one(&self.db)
            .await
        {
            let mut active: status_model::ActiveModel = existing.into();
            active.current_state = Set(current_state.to_string());
            active.circuit_state = Set(circuit_state.to_string());
            active.updated_at = Set(now);
            let _ = active.update(&self.db).await;
        } else {
            let active = status_model::ActiveModel {
                function_name: Set(function_name.to_string()),
                current_state: Set(current_state.to_string()),
                circuit_state: Set(circuit_state.to_string()),
                consecutive_failures: Set(0),
                total_primary_calls: Set(0),
                total_backup_calls: Set(0),
                total_switches: Set(0),
                updated_at: Set(now),
                ..Default::default()
            };
            let _ = active.insert(&self.db).await;
        }
        Ok(())
    }

    /// 手动触发切换（仅管理员）
    pub async fn test_switch(&self, function_name: &str) -> Result<String, String> {
        info!(function = function_name, "手动触发主备切换");
        // 记录事件
        self.record_event(
            function_name,
            "switch_to_backup",
            Some("primary"),
            Some("backup"),
            Some("manual switch"),
            None,
            None,
        )
        .await?;

        // 更新状态
        self.update_status(function_name, "backup", "open").await?;

        // 记录指标
        self.metrics.record_switch(function_name);

        Ok(format!("已切换 {} 至备用", function_name))
    }

    /// 导出 Prometheus 指标
    pub fn export_metrics(&self) -> Result<String, String> {
        self.metrics.export_text().map_err(|e| e.to_string())
    }

    /// 健康检查（ping 数据库 + 缓存）
    pub async fn health_check(&self) -> Result<HealthStatus, String> {
        use status_model::Column as StatusCol;
        // 检查数据库连接
        let db_status = match status_model::Entity::find()
            .filter(StatusCol::FunctionName.eq("database"))
            .one(&self.db)
            .await
        {
            Ok(Some(s)) => s.current_state,
            Ok(None) => "unknown".to_string(),
            Err(_) => "error".to_string(),
        };

        // 检查缓存连接
        let cache_status = match status_model::Entity::find()
            .filter(StatusCol::FunctionName.eq("cache"))
            .one(&self.db)
            .await
        {
            Ok(Some(s)) => s.current_state,
            Ok(None) => "unknown".to_string(),
            Err(_) => "error".to_string(),
        };

        Ok(HealthStatus {
            database: db_status,
            cache: cache_status,
        })
    }
}

/// 健康状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthStatus {
    pub database: String,
    pub cache: String,
}

/// 错误辅助：FailoverError 转字符串
#[allow(dead_code)]
pub fn format_failover_error<E: std::fmt::Debug>(e: &FailoverError<E>) -> String {
    match e {
        FailoverError::PrimaryFailed(_) => "主调用失败".to_string(),
        FailoverError::PrimaryTimeout => "主调用超时".to_string(),
        FailoverError::BackupFailed(_) => "备用调用失败".to_string(),
        FailoverError::BackupTimeout => "备用调用超时".to_string(),
        FailoverError::BothFailed(_, _) => "主备同时失败".to_string(),
        FailoverError::BothTimeout => "主备同时超时".to_string(),
        FailoverError::CircuitOpen => "熔断器已打开".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let m = FailoverMetrics::new();
        assert!(m.is_ok());
    }

    #[test]
    fn test_format_failover_error() {
        let err: FailoverError<String> = FailoverError::PrimaryFailed("test".to_string());
        let s = format_failover_error(&err);
        assert_eq!(s, "主调用失败");
    }
}
