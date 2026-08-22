//! 8D 质量管理流程状态机适配器（A.5.3）
//!
//! 将 8D 的 11 态 + 10 条合法边适配到统一 StateMachine trait。
//! 本适配器为纯内存状态机，仅负责状态合法性校验与流转，
//! 不操作数据库；实际业务推进仍由 QualityEightDService 完成。
//!
//! 事件语义：
//!   - "start"   启动 8D（not_started → d0_plan）
//!   - "advance" 推进到下一 D 阶段（d0_plan→...→d8_recognize，8 条边）
//!   - "close"   关闭 8D（d8_recognize → closed）

use crate::services::quality_8d_service::EightDStatus;
use crate::utils::state_machine_trait::{StateMachine, TransitionResult};

/// 8D 状态机适配器（适配 StateMachine trait）
///
/// 持有当前状态字符串，按 10 条合法边校验与执行转换。
pub struct Quality8dStateMachineAdapter {
    /// 当前状态（8D 状态值字符串，如 "not_started"/"d0_plan" 等）
    current: String,
}

impl Quality8dStateMachineAdapter {
    /// 以指定初始状态构造适配器
    pub fn new(current: impl Into<String>) -> Self {
        Self {
            current: current.into(),
        }
    }

    /// 当前状态字符串解析为 8D 枚举；解析失败返回 None
    fn current_status(&self) -> Option<EightDStatus> {
        self.current.parse().ok()
    }

    /// 给定当前状态与事件，返回下一状态；非法组合返回 None
    ///
    /// 10 条合法边：
    ///   not_started -start->   d0_plan
    ///   d0_plan      -advance-> d1_team
    ///   d1_team      -advance-> d2_problem
    ///   d2_problem   -advance-> d3_interim
    ///   d3_interim   -advance-> d4_root_cause
    ///   d4_root_cause -advance-> d5_permanent
    ///   d5_permanent -advance-> d6_verify
    ///   d6_verify    -advance-> d7_prevent
    ///   d7_prevent   -advance-> d8_recognize
    ///   d8_recognize -close->   closed
    fn next_state(&self, status: EightDStatus, event: &str) -> Option<EightDStatus> {
        match (status, event) {
            (EightDStatus::NotStarted, "start") => Some(EightDStatus::D0Plan),
            (EightDStatus::D0Plan, "advance") => Some(EightDStatus::D1Team),
            (EightDStatus::D1Team, "advance") => Some(EightDStatus::D2Problem),
            (EightDStatus::D2Problem, "advance") => Some(EightDStatus::D3Interim),
            (EightDStatus::D3Interim, "advance") => Some(EightDStatus::D4RootCause),
            (EightDStatus::D4RootCause, "advance") => Some(EightDStatus::D5Permanent),
            (EightDStatus::D5Permanent, "advance") => Some(EightDStatus::D6Verify),
            (EightDStatus::D6Verify, "advance") => Some(EightDStatus::D7Prevent),
            (EightDStatus::D7Prevent, "advance") => Some(EightDStatus::D8Recognize),
            (EightDStatus::D8Recognize, "close") => Some(EightDStatus::Closed),
            _ => None,
        }
    }
}

impl StateMachine for Quality8dStateMachineAdapter {
    type State = String;
    type Event = String;
    type Error = String;

    fn current_state(&self) -> &Self::State {
        &self.current
    }

    fn can_transition(&self, event: &Self::Event) -> bool {
        let Some(status) = self.current_status() else {
            return false;
        };
        self.next_state(status, event).is_some()
    }

    fn transition(
        &mut self,
        event: &Self::Event,
    ) -> Result<TransitionResult<Self::State>, Self::Error> {
        let Some(status) = self.current_status() else {
            return Err(format!("非法当前状态: {}", self.current));
        };
        let Some(next) = self.next_state(status, event) else {
            return Err(format!("非法状态转换: {} -> {}", self.current, event));
        };
        let from = self.current.clone();
        self.current = next.as_str().to_string();
        Ok(TransitionResult {
            from,
            to: self.current.clone(),
        })
    }

    fn available_events(&self) -> Vec<Self::Event> {
        let Some(status) = self.current_status() else {
            return Vec::new();
        };
        match status {
            EightDStatus::NotStarted => vec!["start".to_string()],
            EightDStatus::D8Recognize => vec!["close".to_string()],
            EightDStatus::Closed => Vec::new(),
            // d0_plan ~ d7_prevent 均可 advance
            _ => vec!["advance".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_lifecycle() {
        let mut sm = Quality8dStateMachineAdapter::new("not_started");
        assert!(sm.can_transition(&"start".to_string()));
        let r = sm.transition(&"start".to_string()).unwrap();
        assert_eq!(r.from, "not_started");
        assert_eq!(r.to, "d0_plan");

        // advance 走完 d0_plan -> d8_recognize
        for expected in [
            "d1_team",
            "d2_problem",
            "d3_interim",
            "d4_root_cause",
            "d5_permanent",
            "d6_verify",
            "d7_prevent",
            "d8_recognize",
        ] {
            let r = sm.transition(&"advance".to_string()).unwrap();
            assert_eq!(r.to, expected);
        }

        // close -> closed
        let r = sm.transition(&"close".to_string()).unwrap();
        assert_eq!(r.to, "closed");
        // 终态无可操作事件
        assert!(sm.available_events().is_empty());
        assert!(!sm.can_transition(&"advance".to_string()));
    }

    #[test]
    fn illegal_transition_returns_err() {
        let mut sm = Quality8dStateMachineAdapter::new("not_started");
        // not_started 不允许 advance
        assert!(!sm.can_transition(&"advance".to_string()));
        assert!(sm.transition(&"advance".to_string()).is_err());
    }

    #[test]
    fn available_events_by_state() {
        assert_eq!(
            Quality8dStateMachineAdapter::new("not_started").available_events(),
            vec!["start".to_string()]
        );
        assert_eq!(
            Quality8dStateMachineAdapter::new("d0_plan").available_events(),
            vec!["advance".to_string()]
        );
        assert_eq!(
            Quality8dStateMachineAdapter::new("d8_recognize").available_events(),
            vec!["close".to_string()]
        );
        assert!(
            Quality8dStateMachineAdapter::new("closed")
                .available_events()
                .is_empty()
        );
    }
}
