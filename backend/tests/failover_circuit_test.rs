// 集成测试：熔断器状态机

use bingxi_backend::utils::failover::circuit_breaker::{CircuitBreaker, CircuitState};
use std::thread;
use std::time::Duration;

#[test]
fn test_circuit_breaker_initial_state() {
    let cb = CircuitBreaker::new(3, Duration::from_secs(1));
    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(!cb.is_open());
}

#[test]
fn test_circuit_breaker_opens_after_threshold() {
    let cb = CircuitBreaker::new(3, Duration::from_secs(1));
    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Closed);
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);
    assert!(cb.is_open());
}

#[test]
fn test_circuit_breaker_resets_on_success() {
    let cb = CircuitBreaker::new(3, Duration::from_secs(1));
    cb.record_failure();
    cb.record_failure();
    cb.record_success();
    assert_eq!(cb.consecutive_failures(), 0);
    assert_eq!(cb.state(), CircuitState::Closed);
}

#[test]
fn test_circuit_breaker_half_open_after_duration() {
    let cb = CircuitBreaker::new(2, Duration::from_millis(100));
    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);
    thread::sleep(Duration::from_millis(150));
    // is_open 应自动转为半开
    assert!(!cb.is_open());
    assert_eq!(cb.state(), CircuitState::HalfOpen);
}

#[test]
fn test_circuit_breaker_force_open() {
    let cb = CircuitBreaker::new(5, Duration::from_secs(1));
    cb.force_open();
    assert!(cb.is_open());
}

#[test]
fn test_circuit_breaker_reset() {
    let cb = CircuitBreaker::new(3, Duration::from_secs(1));
    cb.record_failure();
    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);
    cb.reset();
    assert_eq!(cb.state(), CircuitState::Closed);
    assert_eq!(cb.consecutive_failures(), 0);
}

#[test]
fn test_circuit_state_as_i64() {
    assert_eq!(CircuitState::Closed.as_i64(), 0);
    assert_eq!(CircuitState::Open.as_i64(), 1);
    assert_eq!(CircuitState::HalfOpen.as_i64(), 2);
}
