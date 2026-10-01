//! V15 P1 20.6-B：API 网关熔断中间件
//!
//! 实现滑动窗口（5s）失败率检测，> 50% 触发熔断（open 状态直接返回 503），
//! 30s 后进入 half-open 探测，成功则 closed，失败则继续 open。
//!
//! 设计：每个 route_key 维护独立的 CircuitState，全局 HashMap + Mutex 管理。
//!
//! 出参与日志契约（本波次收口）：
//! - 短路响应必须是统一 `AppError` 信封（HTTP 503 + `code=SERVICE_UNAVAILABLE` +
//!   `trace_id` + `timestamp`），不再返回裸文本；`trace_id` 与 `X-Trace-Id` 响应头同源
//!   （本层挂载在 `trace_context_middleware` 内层，`TRACE_ID` task-local 已绑定）。
//! - 根因日志**按状态跃变记录一次**（opened / re-opened / half-open / recovered），外加
//!   每个 open 周期内首次短路记一次 WARN；同一周期内的后续请求只留 DEBUG 行，
//!   避免 open 期间逐请求刷屏（见 [`CircuitEvent`] 与 [`AppError::into_response_rate_limited`]）。

use std::collections::HashMap;
use std::sync::{Arc, Mutex, TryLockError};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use once_cell::sync::Lazy;

use crate::utils::error::AppError;

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
pub enum CircuitState {
    /// 关闭（正常放行）
    Closed,
    /// 打开（直接返回 503）
    Open,
    /// 半开（放行 1 个探测请求）
    HalfOpen,
}

impl CircuitState {
    fn as_str(self) -> &'static str {
        match self {
            CircuitState::Closed => "closed",
            CircuitState::Open => "open",
            CircuitState::HalfOpen => "half_open",
        }
    }
}

/// 窗口统计快照（日志与 detail 共用同一份取值，不另算一套）
#[derive(Debug, Clone, Copy)]
pub struct WindowStats {
    pub total: u32,
    pub failures: u32,
}

impl WindowStats {
    /// 窗口失败率（0.0 ~ 1.0）
    pub fn failure_rate(self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.failures as f64 / self.total as f64
        }
    }

    /// 百分比文本（日志用，避免浮点原值难读）
    pub fn percent_text(self) -> String {
        format!("{:.1}%", self.failure_rate() * 100.0)
    }
}

/// 熔断状态跃变 / 首次短路事件（供中间件做「跃变时记一次」的限流式日志）。
///
/// `CircuitEntry` 自己不知道 route_key，因此不在此处打日志，而是把事件交回中间件，
/// 由中间件带上 route 维度记录 —— 保证运维「见 503 也能见根因」。
#[derive(Debug, Clone, Copy)]
pub enum CircuitEvent {
    /// Closed → Open：窗口失败率越阈
    Opened { window: WindowStats },
    /// HalfOpen → Open：探测请求再次失败，重新熔断（新一轮冷却）
    ReOpened { window: WindowStats },
    /// Open → HalfOpen：冷却结束，放行探测
    HalfOpenProbeStarted { open_for_ms: u128 },
    /// HalfOpen → Closed：探测成功，服务恢复
    Recovered { route_had_failures: bool },
    /// 本 open 周期内的首次短路（后续短路不再逐条记 WARN，只留 DEBUG）
    FirstRejectionInCycle {
        open_for_ms: u128,
        cooldown_remaining_ms: u128,
        window: WindowStats,
    },
}

/// 熔断器条目（每个 route_key 一个）
pub struct CircuitEntry {
    pub state: CircuitState,
    /// 真滑动窗口：记录窗口内每个请求的 (时间戳, 是否失败)，统计时移除 5s 前的过期记录。
    /// A.3 修复：原为翻滚窗口（满 5s 直接清零），边界统计突变（如 4.9s 累积 9 失败，
    /// 5.0s 窗口重置为 0 不触发熔断）。改为 VecDeque 滑动窗口消除边界效应。
    window: std::collections::VecDeque<(Instant, bool)>,
    /// 进入 open 状态的时间（用于 30s 冷却判断）
    pub opened_at: Option<Instant>,
    /// half-open 状态已放行的探测请求数
    half_open_probes: u32,
    /// 当前 open 周期是否已记过「首次短路」WARN：进入 open 时重置
    rejection_logged_in_cycle: bool,
}

impl CircuitEntry {
    pub fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            window: std::collections::VecDeque::new(),
            opened_at: None,
            half_open_probes: 0,
            rejection_logged_in_cycle: false,
        }
    }

    /// 滑动窗口：移除 5s 前的过期记录，返回窗口内 (总数, 失败数)
    fn evict_and_count(&mut self) -> (u32, u32) {
        let cutoff = Instant::now() - Duration::from_secs(WINDOW_SECS);
        // 从队首移除过期记录（按时间顺序入队，队首最旧）
        while let Some(&(ts, _)) = self.window.front() {
            if ts < cutoff {
                self.window.pop_front();
            } else {
                break;
            }
        }
        let total = self.window.len() as u32;
        let failures = self.window.iter().filter(|(_, f)| *f).count() as u32;
        (total, failures)
    }

    /// 当前窗口快照（不做淘汰，日志/detail 用）
    fn window_stats(&self) -> WindowStats {
        WindowStats {
            total: self.window.len() as u32,
            failures: self.window.iter().filter(|(_, f)| *f).count() as u32,
        }
    }

    fn open_for_ms(&self) -> u128 {
        self.opened_at.map(|t| t.elapsed().as_millis()).unwrap_or(0)
    }

    fn cooldown_remaining_ms(&self) -> u128 {
        let cooldown = Duration::from_secs(OPEN_COOLDOWN_SECS);
        match self.opened_at {
            Some(opened_at) => cooldown.saturating_sub(opened_at.elapsed()).as_millis(),
            None => cooldown.as_millis(),
        }
    }

    /// 检查并自动转换状态（open → half-open），跃变写入 `events`
    fn maybe_transition_to_half_open(&mut self, events: &mut Vec<CircuitEvent>) {
        if self.state == CircuitState::Open {
            if let Some(opened_at) = self.opened_at {
                if opened_at.elapsed() >= Duration::from_secs(OPEN_COOLDOWN_SECS) {
                    let open_for_ms = opened_at.elapsed().as_millis();
                    self.state = CircuitState::HalfOpen;
                    self.half_open_probes = 0;
                    events.push(CircuitEvent::HalfOpenProbeStarted { open_for_ms });
                }
            }
        }
    }

    /// 记录一次「本周期首次短路」事件（同一周期内只记一次）
    fn note_rejection(&mut self, events: &mut Vec<CircuitEvent>) {
        if self.rejection_logged_in_cycle {
            return;
        }
        self.rejection_logged_in_cycle = true;
        events.push(CircuitEvent::FirstRejectionInCycle {
            open_for_ms: self.open_for_ms(),
            cooldown_remaining_ms: self.cooldown_remaining_ms(),
            window: self.window_stats(),
        });
    }

    /// 判断请求是否被熔断拒绝
    /// Closed/HalfOpen(未达探测上限) 放行；Open 或 HalfOpen 已达探测上限 拒绝
    ///
    /// 保持原签名（既有单测直接调用）；需要状态跃变日志的调用方请用
    /// [`CircuitEntry::should_reject_tracked`]。
    pub fn should_reject(&mut self) -> bool {
        self.should_reject_tracked(&mut Vec::new())
    }

    /// [`CircuitEntry::should_reject`] 的事件收集版
    pub fn should_reject_tracked(&mut self, events: &mut Vec<CircuitEvent>) -> bool {
        self.maybe_transition_to_half_open(events);
        match self.state {
            CircuitState::Open => {
                self.note_rejection(events);
                true
            }
            CircuitState::HalfOpen => {
                if self.half_open_probes < HALF_OPEN_PROBE_LIMIT {
                    self.half_open_probes += 1;
                    false
                } else {
                    self.note_rejection(events);
                    true
                }
            }
            CircuitState::Closed => false,
        }
    }

    /// 记录请求结果（成功 status < 500，失败 status >= 500）
    ///
    /// 保持原签名（既有单测直接调用）；需要状态跃变日志的调用方请用
    /// [`CircuitEntry::record_result_tracked`]。
    pub fn record_result(&mut self, is_failure: bool) {
        self.record_result_tracked(is_failure, &mut Vec::new());
    }

    /// [`CircuitEntry::record_result`] 的事件收集版
    pub fn record_result_tracked(&mut self, is_failure: bool, events: &mut Vec<CircuitEvent>) {
        // 滑动窗口：记录当前请求并移除过期记录
        self.window.push_back((Instant::now(), is_failure));
        let (total, failures) = self.evict_and_count();
        let window = WindowStats { total, failures };

        match self.state {
            CircuitState::HalfOpen => {
                if is_failure {
                    // 探测失败：重新进入 open（新一轮冷却 + 新的一次根因日志）
                    self.state = CircuitState::Open;
                    self.opened_at = Some(Instant::now());
                    self.rejection_logged_in_cycle = false;
                    events.push(CircuitEvent::ReOpened { window });
                } else {
                    // 探测成功：恢复 closed
                    self.state = CircuitState::Closed;
                    self.opened_at = None;
                    self.window.clear();
                    self.rejection_logged_in_cycle = false;
                    events.push(CircuitEvent::Recovered {
                        route_had_failures: window.failures > 0,
                    });
                }
            }
            CircuitState::Closed => {
                // 仅在窗口内有足够样本（>= 5 个请求）时评估失败率
                if total >= 5 && window.failure_rate() > FAILURE_RATE_THRESHOLD {
                    self.state = CircuitState::Open;
                    self.opened_at = Some(Instant::now());
                    self.rejection_logged_in_cycle = false;
                    events.push(CircuitEvent::Opened { window });
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

/// 状态跃变日志：短路/open 相关用 WARN，恢复/半开用 INFO —— 且只在跃变时各记一次。
///
/// `AppError::into_response_rate_limited` 另有一条 DEBUG 级别的本请求出参记录（含 trace_id），
/// 这里补的是「熔断器视角」的根因：路由、窗口失败率、已熔断时长、冷却剩余。
fn log_circuit_event(route_key: &str, event: &CircuitEvent) {
    match *event {
        CircuitEvent::Opened { window } => tracing::warn!(
            route = %route_key,
            state = %CircuitState::Open.as_str(),
            window_total = window.total,
            window_failures = window.failures,
            failure_rate = %window.percent_text(),
            threshold = %format!("{:.0}%", FAILURE_RATE_THRESHOLD * 100.0),
            window_secs = WINDOW_SECS,
            cooldown_secs = OPEN_COOLDOWN_SECS,
            "CircuitBreaker.opened：窗口失败率越阈，熔断开启；此后同周期的 503 只留 DEBUG 行，按本条根因排查"
        ),
        CircuitEvent::ReOpened { window } => tracing::warn!(
            route = %route_key,
            window_total = window.total,
            window_failures = window.failures,
            failure_rate = %window.percent_text(),
            cooldown_secs = OPEN_COOLDOWN_SECS,
            "CircuitBreaker.reopened：half-open 探测请求仍失败，重新熔断并开始新一轮冷却"
        ),
        CircuitEvent::HalfOpenProbeStarted { open_for_ms } => tracing::info!(
            route = %route_key,
            open_for_ms = open_for_ms,
            probe_limit = HALF_OPEN_PROBE_LIMIT,
            "CircuitBreaker.half_open：冷却结束，放行探测请求"
        ),
        CircuitEvent::Recovered { route_had_failures } => tracing::info!(
            route = %route_key,
            had_failures_in_window = route_had_failures,
            "CircuitBreaker.recovered：探测成功，熔断恢复为 closed，窗口计数已清零"
        ),
        CircuitEvent::FirstRejectionInCycle {
            open_for_ms,
            cooldown_remaining_ms,
            window,
        } => tracing::warn!(
            route = %route_key,
            open_for_ms = open_for_ms,
            cooldown_remaining_ms = cooldown_remaining_ms,
            window_total = window.total,
            window_failures = window.failures,
            failure_rate = %window.percent_text(),
            "CircuitBreaker.reject：本熔断周期首次短路（返回 503 AppError 信封）；同周期后续短路不再逐条记 WARN"
        ),
    }
}

/// 熔断开销明细（只进 tracing 日志，不进 HTTP 出参 —— 出参是固定脱敏文案）
fn circuit_detail(route_key: &str, entry: &CircuitEntry) -> String {
    let window = entry.window_stats();
    format!(
        "route={} state={} window_total={} window_failures={} failure_rate={} opened_for_ms={} cooldown_remaining_ms={}",
        route_key,
        entry.state.as_str(),
        window.total,
        window.failures,
        window.percent_text(),
        entry.open_for_ms(),
        entry.cooldown_remaining_ms()
    )
}

/// 表锁获取失败的原因分类（不把两种完全不同的故障混成一条日志）
fn describe_lock_error<T>(e: &TryLockError<T>) -> &'static str {
    match e {
        TryLockError::WouldBlock => "锁被其他请求持有（非阻塞获取失败）",
        TryLockError::Poisoned(_) => "持锁线程 panic 导致锁中毒",
    }
}

/// V15 P1 20.6-B：API 网关熔断中间件
/// 滑动窗口 5s，失败率 > 50% 触发 open；30s 后 half-open 探测；成功则 closed。
pub async fn circuit_breaker_middleware(req: Request<Body>, next: Next) -> Response {
    let route_key = extract_route_key(&req);

    // 1. 检查熔断状态，决定是否放行
    let mut events: Vec<CircuitEvent> = Vec::new();
    let should_reject = match CIRCUIT_BREAKERS.try_lock() {
        Ok(mut table) => {
            let entry = table
                .entry(route_key.clone())
                .or_insert_with(CircuitEntry::new);
            entry.should_reject_tracked(&mut events)
        }
        Err(e) => {
            let reason = describe_lock_error(&e);
            tracing::warn!(
                route = %route_key,
                error = %e,
                cause = %reason,
                "熔断器表锁不可用，fail-open 放行（本次请求不受熔断保护）"
            );
            false
        }
    };
    for event in &events {
        log_circuit_event(&route_key, event);
    }

    if should_reject {
        // 2a. 短路：统一 AppError 信封（503 + code=SERVICE_UNAVAILABLE + trace_id，
        //     trace_id 与 X-Trace-Id 响应头同源）。根因日志由上面的跃变事件按周期记录，
        //     因此这里用 rate-limited 出参（同形状，逐请求仅 DEBUG 行），避免 open 期间刷屏。
        let detail = match CIRCUIT_BREAKERS.try_lock() {
            Ok(table) => match table.get(&route_key) {
                Some(entry) => circuit_detail(&route_key, entry),
                None => format!("route={} state=absent", route_key),
            },
            Err(e) => {
                tracing::warn!(
                    route = %route_key,
                    error = %e,
                    cause = %describe_lock_error(&e),
                    "熔断明细读取失败，503 detail 降级为仅含 route（响应信封仍完整）"
                );
                format!("route={}", route_key)
            }
        };
        return AppError::service_unavailable(detail).into_response_rate_limited();
    }

    // 2b. 转发请求，记录结果
    let resp = next.run(req).await;
    let is_failure = resp.status().as_u16() >= 500;

    {
        match CIRCUIT_BREAKERS.try_lock() {
            Ok(mut table) => {
                if let Some(entry) = table.get_mut(&route_key) {
                    let mut events = Vec::new();
                    entry.record_result_tracked(is_failure, &mut events);
                    for event in &events {
                        log_circuit_event(&route_key, event);
                    }
                }
            }
            Err(e) => tracing::warn!(
                route = %route_key,
                error = %e,
                cause = %describe_lock_error(&e),
                "熔断器表锁不可用，本次结果未计入窗口统计（失败率判定将缺少该样本）"
            ),
        }
    }

    // 表锁不可用的情况已在上面两处 warn 中显式记录（不吞、不静默），
    // 本函数对响应本身不做额外处理：熔断是旁路观测，不改写业务响应。
    resp
}

/// V15 P1 20.6-B：获取所有路由的熔断器状态（供管理后台 / Prometheus 指标使用）
#[allow(dead_code)]
pub fn get_circuit_breaker_states() -> Vec<(String, &'static str, u32, u32)> {
    let Ok(table) = CIRCUIT_BREAKERS.try_lock() else {
        return Vec::new();
    };
    table
        .iter()
        .map(|(k, e)| {
            // A.3 修复：从 window VecDeque 统计 total/failures（原字段已移除）
            let total = e.window.len() as u32;
            let failures = e.window.iter().filter(|(_, f)| *f).count() as u32;
            (k.clone(), e.state.as_str(), total, failures)
        })
        .collect()
}
