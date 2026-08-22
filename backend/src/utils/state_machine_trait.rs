//! 可插拔状态机框架 trait 定义
//!
//! A.5.1：项目存在四种状态机范式（缸号 DB 规则表驱动、8D 枚举+payload、
//! 定制订单 utils 纯函数、BPM JSON 图遍历），新域接入需选型。
//! 本 trait 提供统一接口，各域适配后可统一管理和测试。
//!
//! 使用方式：
//! ```
//! use crate::utils::state_machine_trait::{StateMachine, TransitionResult};
//!
//! struct MyStateMachine { current: String }
//! impl StateMachine for MyStateMachine {
//!     type State = String;
//!     type Event = String;
//!     type Error = String;
//!     fn current_state(&self) -> &Self::State { &self.current }
//!     fn can_transition(&self, event: &Self::Event) -> bool {
//!         matches!((self.current.as_str(), event.as_str()),
//!             ("draft", "submit") | ("pending", "approve") | ("pending", "reject"))
//!     }
//!     fn transition(&mut self, event: &Self::Event) -> Result<TransitionResult, Self::Error> {
//!         if !self.can_transition(event) {
//!             return Err(format!("非法状态转换: {} -> {}", self.current, event));
//!         }
//!         let old = self.current.clone();
//!         self.current = match (old.as_str(), event.as_str()) {
//!             ("draft", "submit") => "pending".to_string(),
//!             ("pending", "approve") => "approved".to_string(),
//!             ("pending", "reject") => "rejected".to_string(),
//!             _ => return Err("无法到达".to_string()),
//!         };
//!         Ok(TransitionResult { from: old, to: self.current.clone() })
//!     }
//! }
//! ```

/// 状态机统一接口（A.5.1）
///
/// 各域状态机适配此 trait 后，可统一管理状态流转、校验和测试。
/// 适配方负责实现：当前状态查询、合法性检查、状态流转。
// 后续接入 SchedulerRegistry/StateMachine 时会使用
#[allow(dead_code)]
pub trait StateMachine {
    /// 状态类型（如 String 枚举值、自定义 enum）
    type State;
    /// 事件类型（触发状态转换的操作，如 "submit"/"approve"/"reject"）
    type Event;
    /// 错误类型（非法转换时的错误）
    type Error;

    /// 获取当前状态
    fn current_state(&self) -> &Self::State;

    /// 检查给定事件是否可触发合法转换
    fn can_transition(&self, event: &Self::Event) -> bool;

    /// 执行状态转换，返回转换结果（from→to）
    ///
    /// 实现方需保证：非法转换返回 Err，合法转换更新内部状态并返回 Ok。
    fn transition(&mut self, event: &Self::Event) -> Result<TransitionResult<Self::State>, Self::Error>;

    /// 获取所有合法的下一步事件列表（可选，用于 UI 展示可操作按钮）
    fn available_events(&self) -> Vec<Self::Event> {
        Vec::new()
    }
}

/// 状态转换结果
// 后续接入 SchedulerRegistry/StateMachine 时会使用
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TransitionResult<S> {
    /// 转换前状态
    pub from: S,
    /// 转换后状态
    pub to: S,
}
