//! 缸号状态机 StateMachine trait 适配器（dye_batch_state_machine_ops/state_machine_adapter）
//!
//! A.5.2：将缸号状态机适配到统一的 StateMachine trait。
//! 适配器包装现有纯函数版内置流转规则表（dye_batch_state_machine_validation），
//! 不修改现有 service 逻辑，仅做 trait 接口包装。
//!
//! 设计说明：
//! - type State = String（缸号生命周期状态值，如 "pending_schedule"/"dyeing" 等）
//! - type Event = String（流转操作代码，如 "schedule"/"start_dyeing"/"wash" 等）
//! - type Error = String（非法转换错误描述）
//! - 当前为简单版：复用 validation 模块的内置规则纯函数，后续可接入 DB 规则表
//! - 已知限制：on_hold 状态的 RESUME 事件可流转到多个目标状态（dyeing/washing/...），
//!   简单版 transition 只取规则表第一个匹配项，精确路由待接入 DB 后按上下文判定

use crate::services::dye_batch_state_machine_service::{
    get_allowed_transitions, is_terminal_status,
};
use crate::utils::state_machine_trait::{StateMachine, TransitionResult};

/// 缸号状态机适配器，实现统一 StateMachine trait
pub struct DyeBatchStateMachineAdapter {
    /// 当前缸号生命周期状态
    current: String,
}

impl DyeBatchStateMachineAdapter {
    /// 构造适配器，传入初始状态（如 "pending_schedule"）
    pub fn new(initial_state: String) -> Self {
        Self {
            current: initial_state,
        }
    }

    /// 获取当前状态的可变引用（供外部直接重置，如 DB 同步后校正）
    pub fn set_state(&mut self, state: String) {
        self.current = state;
    }
}

impl StateMachine for DyeBatchStateMachineAdapter {
    type State = String;
    type Event = String;
    type Error = String;

    /// 返回当前缸号状态
    fn current_state(&self) -> &Self::State {
        &self.current
    }

    /// 检查给定事件在当前状态下是否可触发合法转换
    /// 复用 validation 模块的内置流转规则表
    fn can_transition(&self, event: &Self::Event) -> bool {
        // 终态不可流转
        if is_terminal_status(&self.current) {
            return false;
        }
        // 查规则表：当前状态是否存在该事件对应的流转
        let allowed = get_allowed_transitions(&self.current);
        allowed.iter().any(|(_, code)| *code == event.as_str())
    }

    /// 执行状态转换，返回 TransitionResult（from → to）
    /// 非法转换返回 Err；合法转换更新内部状态并返回 Ok
    fn transition(&mut self, event: &Self::Event) -> Result<TransitionResult<Self::State>, Self::Error> {
        if !self.can_transition(event) {
            return Err(format!(
                "非法状态转换: {} -> (事件: {})",
                self.current, event
            ));
        }
        // 从规则表找到目标状态（简单版取第一个匹配项）
        let allowed = get_allowed_transitions(&self.current);
        let to = allowed
            .iter()
            .find(|(_, code)| *code == event.as_str())
            .map(|(to, _)| to.to_string())
            .ok_or_else(|| format!("未找到事件 {} 对应的目标状态", event))?;

        let old = self.current.clone();
        self.current = to;
        Ok(TransitionResult {
            from: old,
            to: self.current.clone(),
        })
    }

    /// 获取当前状态下所有合法的下一步事件列表（用于 UI 展示可操作按钮）
    fn available_events(&self) -> Vec<Self::Event> {
        if is_terminal_status(&self.current) {
            return Vec::new();
        }
        get_allowed_transitions(&self.current)
            .iter()
            .map(|(_, code)| code.to_string())
            .collect()
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 正向流转：pending_schedule → scheduled → preparing → dyeing
    #[test]
    fn test_normal_flow() {
        let mut sm = DyeBatchStateMachineAdapter::new("pending_schedule".to_string());
        assert_eq!(sm.current_state(), "pending_schedule");

        assert!(sm.can_transition(&"schedule".to_string()));
        let r = sm.transition(&"schedule".to_string()).unwrap();
        assert_eq!(r.from, "pending_schedule");
        assert_eq!(r.to, "scheduled");

        sm.transition(&"prepare".to_string()).unwrap();
        assert_eq!(sm.current_state(), "preparing");

        sm.transition(&"start_dyeing".to_string()).unwrap();
        assert_eq!(sm.current_state(), "dyeing");
    }

    /// 非法转换应返回 Err
    #[test]
    fn test_illegal_transition() {
        let mut sm = DyeBatchStateMachineAdapter::new("pending_schedule".to_string());
        // pending_schedule 不允许直接 start_dyeing
        assert!(!sm.can_transition(&"start_dyeing".to_string()));
        let err = sm.transition(&"start_dyeing".to_string());
        assert!(err.is_err());
    }

    /// 终态不可流转
    #[test]
    fn test_terminal_status_blocked() {
        let mut sm = DyeBatchStateMachineAdapter::new("shipped".to_string());
        assert!(!sm.can_transition(&"schedule".to_string()));
        assert!(sm.available_events().is_empty());
    }

    /// available_events 返回当前状态允许的事件
    #[test]
    fn test_available_events() {
        let sm = DyeBatchStateMachineAdapter::new("dyeing".to_string());
        let events = sm.available_events();
        // dyeing → washing / cancelled / terminated / on_hold / failed
        assert!(events.contains(&"wash".to_string()));
        assert!(events.contains(&"cancel".to_string()));
        assert!(events.contains(&"terminate".to_string()));
        assert!(events.contains(&"hold".to_string()));
        assert!(events.contains(&"fail".to_string()));
    }

    /// set_state 可直接重置状态
    #[test]
    fn test_set_state() {
        let mut sm = DyeBatchStateMachineAdapter::new("pending_schedule".to_string());
        sm.set_state("dyeing".to_string());
        assert_eq!(sm.current_state(), "dyeing");
    }
}
