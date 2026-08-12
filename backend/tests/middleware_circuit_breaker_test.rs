    use super::*;
#[cfg(test)]
mod tests {

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