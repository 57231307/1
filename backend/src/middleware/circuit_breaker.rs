//! V15 P1 20.6-B：API 网关熔断中间件
//!
//! 实现滑动窗口（5s）失败率检测，> 50% 触发熔断（open 状态直接返回 503），
//! 30s 后进入 half-open 探测，成功则 closed，失败则继续 open。
//!
//! 设计：每个 route_key 维护独立的 CircuitState，全局 HashMap + Mutex 管理。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use once_cell::sync::Lazy;

/// 滑动窗口大小（5 秒）：仅统计最近 5s 内的请求成败
const WINDOW_SECS: u64 = 5;

/// 失败率阈值（50%）：窗口内失败数 / 总数 > 0.5 触发熔断
const FAILURE_RATE_THRESHOLD: f64 = 0.5;

/// 熔断打开后冷却时间（30 秒）：30s 后进入 half-open 探测
const OPEN_COOLDOWN_SECS: u64 = 30;

/// half-open 状态放行的探测请求数（1 个成功则 closed，1 个失败则继续 open）
const HALF_OPEN_PROBE_LIMIT: u32 = 1;

/// 熔断器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitState {
    /// 关闭（正常放行）
    Closed,
    /// 打开（直接返回 503）
    Open,
    /// 半开（放行 1 个探测请求）
    HalfOpen,
}

/// 熔断器条目（每个 route_key 一个）
struct CircuitEntry {
    state: CircuitState,
    /// 滑动窗口内的总请求数
    total: u32,
    /// 滑动窗口内的失败请求数
    failures: u32,
    /// 窗口起始时间（超过 5s 重置窗口）
    window_start: Instant,
    /// 进入 open 状态的时间（用于 30s 冷却判断）
    opened_at: Option<Instant>,
    /// half-open 状态已放行的探测请求数
    half_open_probes: u32,
}

impl CircuitEntry {
    fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            total: 0,
            failures: 0,
            window_start: Instant::now(),
            opened_at: None,
            half_open_probes: 0,
        }
    }

    /// 滚动窗口：若窗口超过 5s 则重置统计
    fn maybe_reset_window(&mut self) {
        if self.window_start.elapsed() >= Duration::from_secs(WINDOW_SECS) {
            self.total = 0;
            self.failures = 0;
            self.window_start = Instant::now();
        }
    }

    /// 检查并自动转换状态（open → half-open）
    fn maybe_transition_to_half_open(&mut self) {
        if self.state == CircuitState::Open {
            if let Some(opened_at) = self.opened_at {
                if opened_at.elapsed() >= Duration::from_secs(OPEN_COOLDOWN_SECS) {
                    self.state = CircuitState::HalfOpen;
                    self.half_open_probes = 0;
                }
            }
        }
    }

    /// 判断请求是否被熔断拒绝
    /// Closed/HalfOpen(未达探测上限) 放行；Open 或 HalfOpen 已达探测上限 拒绝
    fn should_reject(&mut self) -> bool {
        self.maybe_transition_to_half_open();
        match self.state {
            CircuitState::Open => true,
            CircuitState::HalfOpen => {
                if self.half_open_probes < HALF_OPEN_PROBE_LIMIT {
                    self.half_open_probes += 1;
                    false
                } else {
                    true
                }
            }
            CircuitState::Closed => false,
        }
    }

    /// 记录请求结果（成功 status < 500，失败 status >= 500）
    fn record_result(&mut self, is_failure: bool) {
        self.maybe_reset_window();
        self.total += 1;
        if is_failure {
            self.failures += 1;
        }

        match self.state {
            CircuitState::HalfOpen => {
                if is_failure {
                    // 探测失败：回到 open
                    self.state = CircuitState::Open;
                    self.opened_at = Some(Instant::now());
                } else {
                    // 探测成功：恢复 closed
                    self.state = CircuitState::Closed;
                    self.opened_at = None;
                    self.total = 0;
                    self.failures = 0;
                }
            }
            CircuitState::Closed => {
                // 仅在窗口内有足够样本（>= 5 个请求）时评估失败率
                if self.total >= 5 {
                    let rate = self.failures as f64 / self.total as f64;
                    if rate > FAILURE_RATE_THRESHOLD {
                        self.state = CircuitState::Open;
                        self.opened_at = Some(Instant::now());
                    }
                }
            }
            CircuitState::Open => {
                // open 状态下不应有请求到达（should_reject 已拦截），忽略
            }
        }
    }
}

/// 全局熔断器表（按 route_key 索引）
static CIRCUIT_BREAKERS: Lazy<Arc<Mutex<HashMap<String, CircuitEntry>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// 提取 route_key：method + path（不含量化参数，避免 key 爆炸）
fn extract_route_key(req: &Request<Body>) -> String {
    let method = req.method().as_str();
    let path = req.uri().path();
    format!("{}:{}", method, path)
}

/// V15 P1 20.6-B：API 网关熔断中间件
/// 滑动窗口 5s，失败率 > 50% 触发 open；30s 后 half-open 探测；成功则 closed。
pub async fn circuit_breaker_middleware(req: Request<Body>, next: Next) -> Response {
    let route_key = extract_route_key(&req);

    // 1. 检查熔断状态，决定是否放行
    let should_reject = {
        let Ok(mut table) = CIRCUIT_BREAKERS.try_lock() else {
            // 锁不可用（PoisonError 或争用）：fail-open 放行，不影响业务
            tracing::warn!("熔断器表锁不可用，fail-open 放行；key={}", route_key);
            false
        };
        let entry = table.entry(route_key.clone()).or_insert_with(CircuitEntry::new);
        entry.should_reject()
    };

    if should_reject {
        tracing::warn!(
            route = %route_key,
            "CircuitBreaker: 熔断中（open/half-open 已达探测上限），返回 503"
        );
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "服务熔断中，请稍后重试".to_string(),
        )
            .into_response();
    }

    // 2. 转发请求，记录结果
    let resp = next.run(req).await;
    let is_failure = resp.status().as_u16() >= 500;

    {
        let Ok(mut table) = CIRCUIT_BREAKERS.try_lock() else {
            return resp;
        };
        if let Some(entry) = table.get_mut(&route_key) {
            entry.record_result(is_failure);
        }
    }

    resp
}

/// V15 P1 20.6-B：获取所有路由的熔断器状态（供管理后台 / Prometheus 指标使用）
pub fn get_circuit_breaker_states() -> Vec<(String, &'static str, u32, u32)> {
    let Ok(table) = CIRCUIT_BREAKERS.try_lock() else {
        return Vec::new();
    };
    table
        .iter()
        .map(|(k, e)| {
            let state_str = match e.state {
                CircuitState::Closed => "closed",
                CircuitState::Open => "open",
                CircuitState::HalfOpen => "half_open",
            };
            (k.clone(), state_str, e.total, e.failures)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_entry_closed_state() {
        let mut entry = CircuitEntry::new();
        // 5 个请求中 2 个失败（40% < 50% 阈值）应保持 closed
        for _ in 0..3 {
            entry.record_result(false);
        }
        for _ in 0..2 {
            entry.record_result(true);
        }
        assert_eq!(entry.state, CircuitState::Closed);
    }

    #[test]
    fn test_circuit_entry_open_on_threshold() {
        let mut entry = CircuitEntry::new();
        // 5 个请求中 3 个失败（60% > 50% 阈值）应触发 open
        for _ in 0..2 {
            entry.record_result(false);
        }
        for _ in 0..3 {
            entry.record_result(true);
        }
        assert_eq!(entry.state, CircuitState::Open);
        // open 状态应拒绝新请求
        assert!(entry.should_reject());
    }

    #[test]
    fn test_circuit_entry_half_open_recovery() {
        let mut entry = CircuitEntry::new();
        // 模拟触发 open（手动设置 opened_at 为 31s 前）
        entry.state = CircuitState::Open;
        entry.opened_at = Some(Instant::now() - Duration::from_secs(31));
        // should_reject 应自动转换为 half-open 并放行 1 个探测
        assert!(!entry.should_reject());
        assert_eq!(entry.state, CircuitState::HalfOpen);
        // 探测成功 → closed
        entry.record_result(false);
        assert_eq!(entry.state, CircuitState::Closed);
    }
}
