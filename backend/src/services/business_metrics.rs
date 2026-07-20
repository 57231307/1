//! P4-3 监控告警 - 业务指标扩展
//!
//! 在 `metrics_service.rs` 已有指标基础上，新增：
//!
//! ## 业务指标（20+）
//!
//! - `erp_orders_total{status}` - 订单总数（按状态）
//! - `erp_users_active` - 活跃用户数
//! - `erp_ar_balance_total` - 应收账款余额
//! - `erp_ap_balance_total` - 应付账款余额
//! - `erp_inventory_value_total` - 库存价值
//! - `erp_sessions_active` - 活跃 session 数
//! - `erp_cache_hits_total` - 缓存命中数
//! - `erp_cache_misses_total` - 缓存未命中数
//! - `erp_cache_hit_ratio` - 缓存命中率
//! - `erp_login_attempts_total{result}` - 登录尝试（成功/失败）
//! - `erp_login_lockouts_total` - 登录锁定次数
//! - `erp_slow_queries_total{label}` - 慢查询计数
//! - `erp_db_pool_size` - DB 连接池当前大小
//! - `erp_db_pool_overflow_total` - 连接池溢出次数
//! - `erp_websocket_connections` - WebSocket 连接数
//! - `erp_rate_limit_blocked_total{scope}` - 限流拦截次数
//! - `erp_security_alerts_total{type}` - 安全告警计数
//! - `erp_file_uploads_total` - 文件上传总数
//! - `erp_report_executions_total{report}` - 报表执行次数
//! - `erp_ai_predictions_total{model}` - AI 预测次数
//!
//! ## HTTP 增强指标
//!
//! - `http_request_size_bytes` - 请求体大小
//! - `http_response_size_bytes` - 响应体大小
//!
//! ## 使用方式
//!
//! ```rust,ignore
//! use crate::services::business_metrics::BusinessMetrics;
//!
//! // 批次 106 P1-2 修复：通过 state.metrics.business_metrics 访问
//! let m = state.metrics.business_metrics.clone();
//! m.record_order_created("pending");
//! m.set_ar_balance(12345.67);
//! ```

use prometheus::{
    Histogram, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, Opts, Registry,
};

/// 业务指标集合
///
/// 批次 106 P1-2 修复（2026-07-04）：BusinessMetrics 已接入 MetricsService，
/// 通过同一 Registry 注册，/metrics 端点自动暴露 20+ erp_* 指标。
/// 业务侧调用路径：`state.metrics.business_metrics.record_*(...)`。
#[derive(Debug, Clone)]
pub struct BusinessMetrics {
    // ===== 业务核心指标 =====
    /// 订单总数（按状态）
    pub orders_total: IntCounterVec,
    /// 活跃用户数
    pub users_active: IntGauge,
    /// 应收账款余额
    pub ar_balance: IntGauge,
    /// 应付账款余额
    pub ap_balance: IntGauge,
    /// 库存总价值
    pub inventory_value: IntGauge,

    // ===== 会话与缓存 =====
    /// 活跃 session 数
    pub sessions_active: IntGauge,
    /// 缓存命中数
    pub cache_hits: IntCounter,
    /// 缓存未命中数
    pub cache_misses: IntCounter,
    /// 登录尝试（按结果）
    pub login_attempts: IntCounterVec,
    /// 登录锁定次数
    pub login_lockouts: IntCounter,

    // ===== 性能 =====
    /// 慢查询计数（按标签）
    pub slow_queries: IntCounterVec,
    /// 慢查询耗时
    pub slow_query_duration: HistogramVec,
    /// DB 连接池当前大小
    pub db_pool_size: IntGauge,
    /// DB 连接池溢出次数
    pub db_pool_overflow: IntCounter,

    // ===== 安全与告警 =====
    /// WebSocket 连接数
    pub ws_connections: IntGauge,
    /// 限流拦截次数（按 scope）
    pub rate_limit_blocked: IntCounterVec,
    /// 安全告警（按类型）
    pub security_alerts: IntCounterVec,
    /// SQL 注入审计命中
    pub sql_injection_blocked: IntCounter,

    // ===== 业务功能 =====
    /// 文件上传总数
    pub file_uploads: IntCounter,
    /// 报表执行次数（按报表名）
    pub report_executions: IntCounterVec,
    /// AI 预测次数（按模型）
    pub ai_predictions: IntCounterVec,

    // ===== HTTP 增强 =====
    /// HTTP 请求体大小
    pub http_request_size_bytes: Histogram,
    /// HTTP 响应体大小
    pub http_response_size_bytes: Histogram,
}

impl BusinessMetrics {
    /// 创建并注册所有业务指标
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        let (orders_total, users_active, ar_balance, ap_balance, inventory_value) =
            register_business_core_metrics(registry)?;
        let (sessions_active, cache_hits, cache_misses, login_attempts, login_lockouts) =
            register_session_cache_metrics(registry)?;
        let (slow_queries, slow_query_duration, db_pool_size, db_pool_overflow) =
            register_performance_metrics(registry)?;
        let (ws_connections, rate_limit_blocked, security_alerts, sql_injection_blocked) =
            register_security_metrics(registry)?;
        let (file_uploads, report_executions, ai_predictions) =
            register_business_feature_metrics(registry)?;
        let (http_request_size_bytes, http_response_size_bytes) =
            register_http_metrics(registry)?;

        Ok(Self {
            orders_total,
            users_active,
            ar_balance,
            ap_balance,
            inventory_value,
            sessions_active,
            cache_hits,
            cache_misses,
            login_attempts,
            login_lockouts,
            slow_queries,
            slow_query_duration,
            db_pool_size,
            db_pool_overflow,
            ws_connections,
            rate_limit_blocked,
            security_alerts,
            sql_injection_blocked,
            file_uploads,
            report_executions,
            ai_predictions,
            http_request_size_bytes,
            http_response_size_bytes,
        })
    }

    // ===== 业务核心便捷方法 =====
    /// 记录订单创建
    pub fn record_order_created(&self, status: &str) {
        self.orders_total.with_label_values(&[status]).inc();
    }

    /// 设置活跃用户数
    pub fn set_users_active(&self, count: i64) {
        self.users_active.set(count);
    }

    /// 设置应收余额
    pub fn set_ar_balance(&self, balance_fen: i64) {
        self.ar_balance.set(balance_fen);
    }

    /// 设置应付余额
    pub fn set_ap_balance(&self, balance_fen: i64) {
        self.ap_balance.set(balance_fen);
    }

    /// 设置库存价值
    pub fn set_inventory_value(&self, value_fen: i64) {
        self.inventory_value.set(value_fen);
    }

    // ===== 缓存便捷方法 =====
    /// 记录缓存命中
    pub fn record_cache_hit(&self) {
        self.cache_hits.inc();
    }

    /// 记录缓存未命中
    pub fn record_cache_miss(&self) {
        self.cache_misses.inc();
    }

    /// 缓存命中率（0.0 - 1.0）
    pub fn cache_hit_ratio(&self) -> f64 {
        let hits = self.cache_hits.get() as f64;
        let misses = self.cache_misses.get() as f64;
        let total = hits + misses;
        if total == 0.0 { 0.0 } else { hits / total }
    }

    // ===== 登录便捷方法 =====
    /// 记录登录尝试
    pub fn record_login(&self, success: bool) {
        let result = if success { "success" } else { "failure" };
        self.login_attempts.with_label_values(&[result]).inc();
    }

    /// 记录账户锁定
    pub fn record_lockout(&self) {
        self.login_lockouts.inc();
    }

    // ===== 慢查询便捷方法 =====
    /// 记录慢查询
    pub fn record_slow_query(&self, label: &str, duration_secs: f64) {
        self.slow_queries.with_label_values(&[label]).inc();
        self.slow_query_duration.with_label_values(&[label]).observe(duration_secs);
    }

    // ===== 安全便捷方法 =====
    /// 记录限流拦截
    pub fn record_rate_limit_blocked(&self, scope: &str) {
        self.rate_limit_blocked.with_label_values(&[scope]).inc();
    }

    /// 记录安全告警
    pub fn record_security_alert(&self, alert_type: &str) {
        self.security_alerts.with_label_values(&[alert_type]).inc();
    }

    /// 记录 SQL 注入拦截
    pub fn record_sql_injection_blocked(&self) {
        self.sql_injection_blocked.inc();
    }
}

/// 注册业务核心指标：订单/用户/应收/应付/库存
fn register_business_core_metrics(
    registry: &Registry,
) -> Result<(IntCounterVec, IntGauge, IntGauge, IntGauge, IntGauge), prometheus::Error> {
    let orders_total = IntCounterVec::new(
        Opts::new("erp_orders_total", "Total number of orders by status"),
        &["status"],
    )?;
    registry.register(Box::new(orders_total.clone()))?;

    let users_active = IntGauge::new("erp_users_active", "Number of active users")?;
    registry.register(Box::new(users_active.clone()))?;

    let ar_balance = IntGauge::new("erp_ar_balance_total", "Accounts receivable balance (fen)")?;
    registry.register(Box::new(ar_balance.clone()))?;

    let ap_balance = IntGauge::new("erp_ap_balance_total", "Accounts payable balance (fen)")?;
    registry.register(Box::new(ap_balance.clone()))?;

    let inventory_value = IntGauge::new("erp_inventory_value_total", "Inventory value (fen)")?;
    registry.register(Box::new(inventory_value.clone()))?;

    Ok((orders_total, users_active, ar_balance, ap_balance, inventory_value))
}

/// 注册会话与缓存指标
fn register_session_cache_metrics(
    registry: &Registry,
) -> Result<(IntGauge, IntCounter, IntCounter, IntCounterVec, IntCounter), prometheus::Error> {
    let sessions_active = IntGauge::new(
        "erp_sessions_active",
        "Number of active user sessions",
    )?;
    registry.register(Box::new(sessions_active.clone()))?;

    let cache_hits = IntCounter::new("erp_cache_hits_total", "Cache hit count")?;
    registry.register(Box::new(cache_hits.clone()))?;

    let cache_misses = IntCounter::new("erp_cache_misses_total", "Cache miss count")?;
    registry.register(Box::new(cache_misses.clone()))?;

    let login_attempts = IntCounterVec::new(
        Opts::new("erp_login_attempts_total", "Login attempts by result"),
        &["result"],
    )?;
    registry.register(Box::new(login_attempts.clone()))?;

    let login_lockouts =
        IntCounter::new("erp_login_lockouts_total", "Number of account lockouts")?;
    registry.register(Box::new(login_lockouts.clone()))?;

    Ok((sessions_active, cache_hits, cache_misses, login_attempts, login_lockouts))
}

/// 注册性能指标：慢查询/DB 连接池
fn register_performance_metrics(
    registry: &Registry,
) -> Result<(IntCounterVec, HistogramVec, IntGauge, IntCounter), prometheus::Error> {
    let slow_queries = IntCounterVec::new(
        Opts::new("erp_slow_queries_total", "Slow queries count by label"),
        &["label"],
    )?;
    registry.register(Box::new(slow_queries.clone()))?;

    let slow_query_duration = HistogramVec::new(
        HistogramOpts::new(
            "erp_slow_query_duration_seconds",
            "Slow query duration in seconds",
        ),
        &["label"],
    )?;
    registry.register(Box::new(slow_query_duration.clone()))?;

    let db_pool_size = IntGauge::new(
        "erp_db_pool_size",
        "Current database connection pool size",
    )?;
    registry.register(Box::new(db_pool_size.clone()))?;

    let db_pool_overflow = IntCounter::new(
        "erp_db_pool_overflow_total",
        "Database pool overflow events",
    )?;
    registry.register(Box::new(db_pool_overflow.clone()))?;

    Ok((slow_queries, slow_query_duration, db_pool_size, db_pool_overflow))
}

/// 注册安全指标：WebSocket/限流/告警/SQL 注入
fn register_security_metrics(
    registry: &Registry,
) -> Result<(IntGauge, IntCounterVec, IntCounterVec, IntCounter), prometheus::Error> {
    let ws_connections = IntGauge::new(
        "erp_websocket_connections",
        "Active WebSocket connections",
    )?;
    registry.register(Box::new(ws_connections.clone()))?;

    let rate_limit_blocked = IntCounterVec::new(
        Opts::new(
            "erp_rate_limit_blocked_total",
            "Requests blocked by rate limit by scope",
        ),
        &["scope"],
    )?;
    registry.register(Box::new(rate_limit_blocked.clone()))?;

    let security_alerts = IntCounterVec::new(
        Opts::new("erp_security_alerts_total", "Security alerts by type"),
        &["type"],
    )?;
    registry.register(Box::new(security_alerts.clone()))?;

    let sql_injection_blocked = IntCounter::new(
        "erp_sql_injection_blocked_total",
        "SQL injection patterns blocked",
    )?;
    registry.register(Box::new(sql_injection_blocked.clone()))?;

    Ok((ws_connections, rate_limit_blocked, security_alerts, sql_injection_blocked))
}

/// 注册业务功能指标：文件上传/报表/AI 预测
fn register_business_feature_metrics(
    registry: &Registry,
) -> Result<(IntCounter, IntCounterVec, IntCounterVec), prometheus::Error> {
    let file_uploads = IntCounter::new("erp_file_uploads_total", "File uploads count")?;
    registry.register(Box::new(file_uploads.clone()))?;

    let report_executions = IntCounterVec::new(
        Opts::new(
            "erp_report_executions_total",
            "Report executions by report name",
        ),
        &["report"],
    )?;
    registry.register(Box::new(report_executions.clone()))?;

    let ai_predictions = IntCounterVec::new(
        Opts::new("erp_ai_predictions_total", "AI predictions by model"),
        &["model"],
    )?;
    registry.register(Box::new(ai_predictions.clone()))?;

    Ok((file_uploads, report_executions, ai_predictions))
}

/// 注册 HTTP 增强指标：请求/响应体大小
fn register_http_metrics(
    registry: &Registry,
) -> Result<(Histogram, Histogram), prometheus::Error> {
    let http_request_size_bytes = Histogram::with_opts(HistogramOpts::new(
        "http_request_size_bytes",
        "HTTP request body size in bytes",
    ))?;
    registry.register(Box::new(http_request_size_bytes.clone()))?;

    let http_response_size_bytes = Histogram::with_opts(HistogramOpts::new(
        "http_response_size_bytes",
        "HTTP response body size in bytes",
    ))?;
    registry.register(Box::new(http_response_size_bytes.clone()))?;

    Ok((http_request_size_bytes, http_response_size_bytes))
}

/// 指标注册表构建器（仅测试用）
///
/// 批次 106 P1-2 修复：原 pub fn 改为 #[cfg(test)]，避免生产代码死代码。
/// 生产环境通过 `MetricsService::new()` 内部构造 BusinessMetrics 并注册到同一 Registry。
#[cfg(test)]
pub fn build_registry_and_metrics() -> Result<(std::sync::Arc<Registry>, BusinessMetrics), prometheus::Error> {
    let registry = std::sync::Arc::new(Registry::new());
    let metrics = BusinessMetrics::new(&registry)?;
    Ok((registry, metrics))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // P9-1: 测试夹具 helper，统一 build_registry_and_metrics 的 expect
    fn build_metrics() -> (Arc<prometheus::Registry>, BusinessMetrics) {
        build_registry_and_metrics().expect("P9-1: 测试夹具 metrics 注册失败")
    }

    #[test]
    fn 测试_business_metrics_注册() {
        // 中文测试名：测试 business metrics 全部注册成功
        let (registry, _m) = build_metrics();
        let families = registry.gather();
        // 至少 20+ 个指标家族
        assert!(families.len() >= 20, "指标家族数应 >= 20，实际: {}", families.len());
    }

    #[test]
    fn 测试_缓存命中率() {
        // 中文测试名：测试缓存命中率计算
        let (_r, m) = build_metrics();
        m.record_cache_hit();
        m.record_cache_hit();
        m.record_cache_hit();
        m.record_cache_miss();
        let ratio = m.cache_hit_ratio();
        assert!((ratio - 0.75).abs() < 1e-9);
    }

    #[test]
    fn 测试_登录记录() {
        // 中文测试名：测试登录成功/失败记录
        let (_r, m) = build_metrics();
        m.record_login(true);
        m.record_login(true);
        m.record_login(false);
        // 验证不 panic
    }
}
