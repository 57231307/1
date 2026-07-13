//! 慢查询审计中间件（P4-1 性能优化）
//!
//! 在 SQL 执行时记录耗时超过阈值的查询，输出到 `tracing` 日志与 Prometheus 指标。
//!
//! ## 工作原理
//!
//! 由于 SeaORM/SQLx 的执行 hook 不暴露在应用层，本中间件通过业务层
//! 调用的 `SlowQueryRecorder::record()` 接入：
//!
//! 1. service 层在关键 SQL 前后调用 `SlowQueryRecorder::start()` 获取计时器
//! 2. `finish()` 时若耗时 > 100ms（可配置），记录到 `tracing::warn!`
//! 3. 同时通过 `MetricsService::record_slow_query` 暴露 Prometheus 指标
//!
//! ## 使用方式
//!
//! ```rust,ignore
//! let rec = SlowQueryRecorder::start("select_orders", None);
//! let result = query_orders().await;
//! rec.finish();
//! ```

use crate::services::metrics_service::MetricsService;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 慢查询阈值（可由环境变量 `BINGXI_SLOW_QUERY_MS` 覆盖，默认 100ms）
///
/// L-38 修复（批次 370 v13 复审）：使用 LazyLock 确保首次调用时打印当前阈值，
/// 消除 silent default（原实现环境变量未设置时静默使用 100ms，无任何日志）。
static SLOW_QUERY_THRESHOLD_MS: std::sync::LazyLock<u64> = std::sync::LazyLock::new(|| {
    let ms = std::env::var("BINGXI_SLOW_QUERY_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(100);
    if std::env::var("BINGXI_SLOW_QUERY_MS").is_err() {
        tracing::info!(
            "BINGXI_SLOW_QUERY_MS 未设置，使用默认阈值 100ms"
        );
    } else {
        tracing::info!(
            threshold_ms = ms,
            "BINGXI_SLOW_QUERY_MS 已设置"
        );
    }
    ms
});

pub fn slow_query_threshold() -> Duration {
    Duration::from_millis(*SLOW_QUERY_THRESHOLD_MS)
}

/// 慢查询记录器（RAII 风格：创建时开始计时，drop 时判断是否上报）
pub struct SlowQueryRecorder {
    /// 查询标签（如 `select_orders` / `find_inventory_stocks`）
    pub label: &'static str,
    /// 起始时间
    pub start: Instant,
    /// 指标服务（可空 - 测试环境允许为 None）
    pub metrics: Option<Arc<MetricsService>>,
}

impl SlowQueryRecorder {
    /// 启动一个慢查询记录器
    pub fn start(label: &'static str, metrics: Option<Arc<MetricsService>>) -> Self {
        Self {
            label,
            start: Instant::now(),
            metrics,
        }
    }

    /// 完成计时；如超过阈值则记录到日志与指标
    pub fn finish(self) {
        let elapsed = self.start.elapsed();
        if elapsed >= slow_query_threshold() {
            tracing::warn!(
                target: "slow_query",
                label = self.label,
                elapsed_ms = elapsed.as_millis() as u64,
                threshold_ms = slow_query_threshold().as_millis() as u64,
                "检测到慢查询",
            );
            if let Some(m) = &self.metrics {
                m.record_slow_query_metric(self.label, elapsed);
            }
        }
    }
}

/// 慢查询指标 trait 扩展
///
/// 业务侧 metrics_service 不一定实现该方法，故用 trait + 默认空实现避免破坏现有签名。
/// 批次 97 P1-15 修复（v5 复审）：trait 方法重命名为 `record_slow_query_metric`，
/// 避免与 MetricsService 的 inherent 方法 `record_slow_query(duration_secs, query_name)` 同名冲突。
pub trait SlowQueryMetrics {
    /// 记录一次慢查询
    fn record_slow_query_metric(&self, label: &str, elapsed: Duration);
}

impl SlowQueryMetrics for MetricsService {
    fn record_slow_query_metric(&self, label: &str, elapsed: Duration) {
        // 真正接入 MetricsService 的 Prometheus 指标，替代原 no-op 占位实现。
        // record_slow_query 是 Metrics（不是 MetricsService）的 inherent 方法，
        // 通过 self.metrics（Arc<Metrics>）auto-deref 调用。
        let duration_secs = elapsed.as_secs_f64();
        self.metrics.record_slow_query(duration_secs, label);
    }
}
