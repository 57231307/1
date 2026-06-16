//! 熔断器实现
//!
//! 当主调用连续失败达到阈值时打开熔断器，阻止后续主调用直接失败，
//! 经过熔断时长后进入半开状态探测主调用是否恢复。
//!
//! # 状态机
//!
//! ```text
//! Closed --连续失败 N 次--> Open --经过 duration S--> HalfOpen --探测成功--> Closed
//!                                                      └--探测失败--> Open
//! ```

use std::sync::atomic::{AtomicI64, AtomicU32, AtomicU8, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 熔断器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// 关闭（正常）
    Closed = 0,
    /// 打开（熔断）
    Open = 1,
    /// 半开（探测）
    HalfOpen = 2,
}

impl CircuitState {
    /// 从 u8 转换为状态
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Closed,
            1 => Self::Open,
            2 => Self::HalfOpen,
            _ => Self::Closed,
        }
    }

    /// 转换为 i64（用于 Prometheus 指标）
    pub fn as_i64(self) -> i64 {
        self as i64
    }
}

/// 熔断器
pub struct CircuitBreaker {
    /// 熔断阈值（连续失败次数）
    threshold: u32,
    /// 熔断时长
    duration: Duration,
    /// 当前状态
    state: AtomicU8,
    /// 连续失败次数
    consecutive_failures: AtomicU32,
    /// 熔断开始时间（毫秒）
    opened_at_ms: AtomicI64,
}

impl CircuitBreaker {
    /// 创建熔断器
    ///
    /// # 参数
    ///
    /// - `threshold`：连续失败次数阈值，达到后打开熔断器
    /// - `duration`：熔断时长，超过后进入半开状态
    pub fn new(threshold: u32, duration: Duration) -> Self {
        Self {
            threshold,
            duration,
            state: AtomicU8::new(CircuitState::Closed as u8),
            consecutive_failures: AtomicU32::new(0),
            opened_at_ms: AtomicI64::new(0),
        }
    }

    /// 默认熔断器（阈值 5，时长 30s）
    pub fn default_breaker() -> Self {
        Self::new(5, Duration::from_secs(30))
    }

    /// 获取当前状态
    pub fn state(&self) -> CircuitState {
        CircuitState::from_u8(self.state.load(Ordering::Acquire))
    }

    /// 检查熔断器是否打开
    ///
    /// 如果熔断器已打开且未超过熔断时长，返回 true。
    /// 如果超过熔断时长，自动转入半开状态并返回 false。
    pub fn is_open(&self) -> bool {
        let current = self.state();
        if current == CircuitState::Open {
            let opened_at = self.opened_at_ms.load(Ordering::Acquire);
            let now = Self::now_ms();
            let elapsed = now.saturating_sub(opened_at);
            if elapsed >= self.duration.as_millis() as i64 {
                // 进入半开状态
                self.state.store(CircuitState::HalfOpen as u8, Ordering::Release);
                return false;
            }
            return true;
        }
        false
    }

    /// 记录成功
    ///
    /// 重置连续失败次数并将状态转为 Closed。
    pub fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::Release);
        self.state.store(CircuitState::Closed as u8, Ordering::Release);
    }

    /// 记录失败
    ///
    /// 增加连续失败次数，达到阈值后打开熔断器。
    pub fn record_failure(&self) {
        let failures = self.consecutive_failures.fetch_add(1, Ordering::AcqRel) + 1;
        if failures >= self.threshold {
            self.state.store(CircuitState::Open as u8, Ordering::Release);
            self.opened_at_ms.store(Self::now_ms(), Ordering::Release);
        }
    }

    /// 强制重置熔断器
    pub fn reset(&self) {
        self.consecutive_failures.store(0, Ordering::Release);
        self.state.store(CircuitState::Closed as u8, Ordering::Release);
        self.opened_at_ms.store(0, Ordering::Release);
    }

    /// 强制打开熔断器（用于手动测试）
    pub fn force_open(&self) {
        self.state.store(CircuitState::Open as u8, Ordering::Release);
        self.opened_at_ms.store(Self::now_ms(), Ordering::Release);
    }

    /// 获取连续失败次数
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures.load(Ordering::Acquire)
    }

    /// 获取当前毫秒时间戳
    fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_circuit_initial_state() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(1));
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(!cb.is_open());
    }

    #[test]
    fn test_circuit_open_after_threshold() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(1));
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(cb.is_open());
    }

    #[test]
    fn test_circuit_success_resets() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(1));
        cb.record_failure();
        cb.record_failure();
        cb.record_success();
        assert_eq!(cb.consecutive_failures(), 0);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_half_open_after_duration() {
        let cb = CircuitBreaker::new(2, Duration::from_millis(100));
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        // 等待超过熔断时长
        thread::sleep(Duration::from_millis(150));
        // is_open 应自动转为半开
        assert!(!cb.is_open());
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_circuit_force_open() {
        let cb = CircuitBreaker::new(5, Duration::from_secs(1));
        cb.force_open();
        assert!(cb.is_open());
    }
}
