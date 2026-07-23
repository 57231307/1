//! 事件总线业务实现子模块（event_bus_ops）
//!
//! 本模块与 `event_bus` 同为 `crate::services` 下的兄弟模块。
//! 公共类型（`BusinessEvent` / `ShippedItem` / `EventBus` / `EventBusState`）、
//! 全局状态（`EVENT_BUS` / `EVENT_BUS_STATE` / `MAIN_LISTENER_HANDLE`）、
//! 以及 publish/subscribe 公共 API 仍定义在 `crate::services::event_bus`（facade）。
//!
//! 本 ops 子模块通过 `use crate::services::event_bus::{...}` 导入 facade 类型与状态，
//! 编写各自的事件监听 / Kafka 初始化自由函数。
//! `EventBusState` 字段与 `lock_event_bus_state` 在 facade 中声明为 `pub(crate)`，
//! 本模块可直接访问。

pub mod kafka;
pub mod listener;

// 对外 re-export：facade 通过 `pub use crate::services::event_bus_ops::*` 再次导出，
// 保持 `crate::services::event_bus::{start_event_listener, init_event_bus_with_kafka_config,
// shutdown_event_bus}` 旧路径完全向后兼容。
pub use kafka::init_event_bus_with_kafka_config;
pub use listener::{shutdown_event_bus, start_event_listener};
