//! 定制订单状态机适配器
//!
//! A.5.4：将定制订单纯函数状态机适配到统一的 StateMachine trait。
//! 复用 `crate::utils::process_state_machine` 中的 `can_transition` / `next_status` 纯函数，
//! 不修改现有 service，仅做 trait 桥接。
//!
//! 状态集合（String）：draft / lab_dip / quotation / yarn_purchasing / dyeing /
//! finishing / delivery / after_sales / completed / cancelled
//! 事件集合（String）：
//! - "advance"：自动推进到下一阶段（调用 next_status）
//! - "cancel"：取消订单（非终态 → cancelled）
//! - "set:<target>"：直接设置目标状态（用 can_transition 校验合法性）

use crate::utils::process_state_machine::{
    CustomOrderStatus, can_transition as pure_can_transition, next_status as pure_next_status,
};
use crate::utils::state_machine_trait::{StateMachine, TransitionResult};

/// 定制订单状态机适配器
///
/// 持有当前状态（String），通过委托纯函数实现 trait。
/// 构造时校验初始状态合法，运行期保证状态机语义一致。
#[derive(Debug, Clone)]
pub struct CustomOrderStateMachineAdapter {
    /// 当前状态字符串（对应 CustomOrderStatus::as_str）
    current: String,
}

impl CustomOrderStateMachineAdapter {
    /// 构造适配器，初始状态必须合法
    pub fn new(initial: &str) -> Result<Self, String> {
        let _ = initial
            .parse::<CustomOrderStatus>()
            .map_err(|e| format!("初始状态非法: {initial}, {e}"))?;
        Ok(Self {
            current: initial.to_string(),
        })
    }

    /// 解析 "set:<target>" 事件为目标状态
    fn parse_set_event(event: &str) -> Option<&str> {
        event.strip_prefix("set:")
    }
}

impl StateMachine for CustomOrderStateMachineAdapter {
    type State = String;
    type Event = String;
    type Error = String;

    fn current_state(&self) -> &Self::State {
        &self.current
    }

    fn can_transition(&self, event: &Self::Event) -> bool {
        match event.as_str() {
            // 自动推进：当前非终态即可
            "advance" => pure_next_status(&self.current).is_ok(),
            // 取消：当前非终态即可
            "cancel" => {
                matches!(
                    self.current.parse::<CustomOrderStatus>(),
                    Ok(s) if !s.is_terminal()
                )
            }
            // 直接设置目标：委托纯函数校验
            other => {
                if let Some(target) = Self::parse_set_event(other) {
                    pure_can_transition(&self.current, target)
                } else {
                    false
                }
            }
        }
    }

    fn transition(&mut self, event: &Self::Event) -> Result<TransitionResult<Self::State>, Self::Error> {
        if !self.can_transition(event) {
            return Err(format!("非法状态转换: {} -> {}", self.current, event));
        }

        let old = self.current.clone();
        let next = match event.as_str() {
            "advance" => pure_next_status(&self.current)
                .map(|s| s.as_str().to_string())
                .map_err(|e| format!("推进失败: {e}"))?,
            "cancel" => CustomOrderStatus::Cancelled.as_str().to_string(),
            other => {
                let target = Self::parse_set_event(other)
                    .ok_or_else(|| format!("无法解析事件: {other}"))?;
                target.to_string()
            }
        };

        self.current = next.clone();
        Ok(TransitionResult { from: old, to: next })
    }

    fn available_events(&self) -> Vec<Self::Event> {
        let mut events = Vec::new();
        // 终态无可推进事件
        let cur = match self.current.parse::<CustomOrderStatus>() {
            Ok(s) => s,
            Err(_) => return events,
        };
        if cur.is_terminal() {
            return events;
        }
        // 非终态可推进 / 取消
        if pure_next_status(&self.current).is_ok() {
            events.push("advance".to_string());
        }
        events.push("cancel".to_string());
        // 可直接跳转的目标状态（除自身和 cancelled，cancelled 已由 cancel 覆盖）
        for target in [
            CustomOrderStatus::Draft,
            CustomOrderStatus::LabDip,
            CustomOrderStatus::Quotation,
            CustomOrderStatus::YarnPurchasing,
            CustomOrderStatus::Dyeing,
            CustomOrderStatus::Finishing,
            CustomOrderStatus::Delivery,
            CustomOrderStatus::AfterSales,
            CustomOrderStatus::Completed,
            CustomOrderStatus::Cancelled,
        ] {
            let t = target.as_str();
            if t != self.current
                && target != CustomOrderStatus::Cancelled
                && pure_can_transition(&self.current, t)
            {
                events.push(format!("set:{t}"));
            }
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_full_pipeline() {
        let mut sm = CustomOrderStateMachineAdapter::new("draft").unwrap();
        assert_eq!(sm.current_state(), "draft");

        for expected in ["lab_dip", "quotation", "yarn_purchasing", "dyeing", "finishing", "delivery", "after_sales", "completed"] {
            let r = sm.transition(&"advance".to_string()).unwrap();
            assert_eq!(r.to, expected);
        }
        // 终态不可再推进
        assert!(!sm.can_transition(&"advance".to_string()));
    }

    #[test]
    fn cancel_from_non_terminal() {
        let mut sm = CustomOrderStateMachineAdapter::new("yarn_purchasing").unwrap();
        assert!(sm.can_transition(&"cancel".to_string()));
        let r = sm.transition(&"cancel".to_string()).unwrap();
        assert_eq!(r.to, "cancelled");
        assert!(!sm.can_transition(&"cancel".to_string()));
    }

    #[test]
    fn set_target_validates_legality() {
        let mut sm = CustomOrderStateMachineAdapter::new("draft").unwrap();
        // draft → quotation 非法（必须经过 lab_dip）
        assert!(!sm.can_transition(&"set:quotation".to_string()));
        // draft → cancelled 合法
        assert!(sm.can_transition(&"set:cancelled".to_string()));
        let r = sm.transition(&"set:cancelled".to_string()).unwrap();
        assert_eq!(r.to, "cancelled");
    }

    #[test]
    fn illegal_transition_returns_error() {
        let mut sm = CustomOrderStateMachineAdapter::new("completed").unwrap();
        let err = sm.transition(&"advance".to_string()).unwrap_err();
        assert!(err.contains("非法状态转换"));
    }

    #[test]
    fn invalid_initial_state_rejected() {
        assert!(CustomOrderStateMachineAdapter::new("unknown").is_err());
    }

    #[test]
    fn available_events_non_terminal() {
        let sm = CustomOrderStateMachineAdapter::new("draft").unwrap();
        let events = sm.available_events();
        assert!(events.contains(&"advance".to_string()));
        assert!(events.contains(&"cancel".to_string()));
    }

    #[test]
    fn available_events_terminal_empty() {
        let sm = CustomOrderStateMachineAdapter::new("completed").unwrap();
        assert!(sm.available_events().is_empty());
    }
}
