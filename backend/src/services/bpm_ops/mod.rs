//! BPM 工作流服务业务实现子模块（bpm_ops）
//!
//! 批次 D10 拆分：从原 `bpm_service.rs`（1060 行）拆出 `BpmService` impl 业务方法块。
//! - `instance`：流程实例生命周期（start_process / cancel_instance / 业务查询 / 审批链 / 业务关联）
//! - `task`：任务审批流（approve_task + 推进/拒绝/完成 + 查询/转办/催办）
//! - `monitor`：流程监控统计（实例/任务统计 + 待处理任务列表 + 实例列表）
//!
//! `BpmService` struct 定义与 `new` 构造函数、纯函数（`evaluate_bpm_condition` /
//! `resolve_first_task_node`）保留在 facade `bpm_service` 中，impl 业务方法块分散到本子模块，
//! Rust 允许同一 crate 多文件多 impl 块。
//! `db` 字段声明为 `pub(crate)` 以便本子模块访问；跨 facade 调用的纯函数亦为 `pub(crate)`。

pub mod instance;
pub mod monitor;
pub mod task;
