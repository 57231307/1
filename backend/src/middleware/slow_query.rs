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
//! let rec = SlowQueryRecorder::start("select_orders", None, None);
//! let result = query_orders().await;
//! rec.finish();
//! ```

use crate::models::notification::{NotificationPriority, NotificationType};
use crate::services::metrics_service::MetricsService;
use crate::services::notification_service::{CreateNotificationRequest, NotificationService};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 慢查询阈值（可由环境变量 `BINGXI_SLOW_QUERY_MS` 覆盖，默认 100ms）；L-38 修复（批次 370 v13 复审）：使用
/// LazyLock 确保首次调用时打印当前阈值， 消除 silent default（原实现环境变量未设置时静默使用 100ms，无任何日志）。
static SLOW_QUERY_THRESHOLD_MS: std::sync::LazyLock<u64> = std::sync::LazyLock::new(|| {
    let ms = std::env::var("BINGXI_SLOW_QUERY_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(100);
    if std::env::var("BINGXI_SLOW_QUERY_MS").is_err() {
        tracing::info!("BINGXI_SLOW_QUERY_MS 未设置，使用默认阈值 100ms");
    } else {
        tracing::info!(threshold_ms = ms, "BINGXI_SLOW_QUERY_MS 已设置");
    }
    ms
});

/// V15 P2 20.5-B：慢查询告警去重窗口（每小时聚合去重）
/// key: SQL hash, value: (last_alert_time, alert_count_this_hour)
static SLOW_QUERY_ALERT_STATE: std::sync::LazyLock<Mutex<HashMap<u64, (Instant, u32)>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// 慢查询告警去重窗口时长（1 小时）
const ALERT_DEDUP_WINDOW: Duration = Duration::from_secs(3600);

pub fn slow_query_threshold() -> Duration {
    Duration::from_millis(*SLOW_QUERY_THRESHOLD_MS)
}

/// 计算 SQL 文本的 hash（用于告警去重）
fn sql_hash(sql: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    sql.hash(&mut hasher);
    hasher.finish()
}

/// 检查是否应该发送告警（每小时聚合去重）
/// 返回 true 表示应该发送告警，false 表示已被去重
fn should_send_alert(sql_hash: u64) -> bool {
    let mut state = match SLOW_QUERY_ALERT_STATE.lock() {
        Ok(s) => s,
        Err(_) => return true, // 锁中毒时放行
    };
    let now = Instant::now();
    match state.get_mut(&sql_hash) {
        Some((last_alert, count)) => {
            if now.duration_since(*last_alert) >= ALERT_DEDUP_WINDOW {
                // 超过去重窗口，重置计数
                *last_alert = now;
                *count = 1;
                true
            } else {
                // 在去重窗口内，仅第一次发送
                *count += 1;
                false
            }
        }
        None => {
            state.insert(sql_hash, (now, 1));
            true
        }
    }
}

/// 获取去重窗口内的告警次数（用于告警消息中展示）
fn get_alert_count(sql_hash: u64) -> u32 {
    SLOW_QUERY_ALERT_STATE
        .lock()
        .ok()
        .and_then(|state| state.get(&sql_hash).map(|(_, count)| *count))
        .unwrap_or(1)
}

/// 慢查询记录器（RAII 风格：创建时开始计时，drop 时判断是否上报）
pub struct SlowQueryRecorder {
    /// 查询标签（如 `select_orders` / `find_inventory_stocks`）
    pub label: &'static str,
    /// 起始时间
    pub start: Instant,
    /// 指标服务（可空 - 测试环境允许为 None）
    pub metrics: Option<Arc<MetricsService>>,
    /// 通知服务（可空 - 用于慢查询告警）
    pub notification_service: Option<Arc<NotificationService>>,
    /// V15 P2 20.5-B：可选的 SQL 文本（用于告警消息）
    pub sql_text: Option<String>,
}

impl SlowQueryRecorder {
    /// 启动一个慢查询记录器
    pub fn start(
        label: &'static str,
        metrics: Option<Arc<MetricsService>>,
        notification_service: Option<Arc<NotificationService>>,
    ) -> Self {
        Self {
            label,
            start: Instant::now(),
            metrics,
            notification_service,
            sql_text: None,
        }
    }

    /// V15 P2 20.5-B：设置 SQL 文本（用于告警消息中展示）
    pub fn with_sql_text(mut self, sql: &str) -> Self {
        self.sql_text = Some(sql.to_string());
        self
    }

    /// 完成计时；如超过阈值则记录到日志与指标；超过 2 倍阈值时发送告警通知（每小时聚合去重）
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
            // 超过 2 倍阈值时发送告警通知（每小时聚合去重）
            if elapsed >= slow_query_threshold() * 2 {
                if let Some(ns) = &self.notification_service {
                    let label = self.label;
                    let elapsed_ms = elapsed.as_millis() as u64;
                    // V15 P2 20.5-B：使用 SQL hash 做去重，每小时聚合
                    let hash = sql_hash(self.sql_text.as_deref().unwrap_or(label));
                    if should_send_alert(hash) {
                        let alert_count = get_alert_count(hash);
                        let sql_preview = self
                            .sql_text
                            .as_deref()
                            .unwrap_or(label)
                            .chars()
                            .take(200)
                            .collect::<String>();
                        let dedup_key = format!("slow_query_alert:{}", hash);
                        let content = format!(
                            "检测到严重慢查询：{}，耗时 {}ms（阈值 {}ms）\nSQL: {}{}",
                            label,
                            elapsed_ms,
                            slow_query_threshold().as_millis(),
                            sql_preview,
                            if alert_count > 1 {
                                format!("\n（本小时第 {} 次出现）", alert_count)
                            } else {
                                String::new()
                            },
                        );
                        let req = CreateNotificationRequest {
                            user_id: 1, // 系统管理员
                            notification_type: NotificationType::System,
                            title: "慢查询告警".to_string(),
                            content,
                            priority: NotificationPriority::High,
                            business_type: Some("SLOW_QUERY".to_string()),
                            business_id: None,
                            action_url: Some("/system/slow-queries".to_string()),
                            sender_id: None,
                            sender_name: Some("系统".to_string()),
                            dedup_key: Some(dedup_key),
                        };
                        let ns = ns.clone();
                        tokio::spawn(async move {
                            let _ = ns.create_notification(req).await;
                        });
                    }
                }
            }
        }
    }
}

/// 慢查询指标 trait 扩展；业务侧 metrics_service 不一定实现该方法，故用 trait + 默认空实现避免破坏现有签名。 批次 97 P1-15 修复（v5 复审）：trait 方法重命名为
/// `record_slow_query_metric`， 避免与 MetricsService 的 inherent 方法 `record_slow_query(duration_secs, query_name)` 同名冲突。
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
