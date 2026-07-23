//! 缸号全生命周期状态机 Service 的业务实现子模块（dye_batch_state_machine_ops）
//!
//! 批次 490 D10-4a 拆分：从原 `dye_batch_state_machine_service.rs` 迁移 4 个 Service 的 impl 块。
//! 每个子模块包含对应 Service 的业务方法 impl 块（struct + new 保留在 facade）。
//!
//! 模块层级关系：
//! - `dye_batch_state_machine_ops` 与 `dye_batch_state_machine_service` 同为 `crate::services` 下的兄弟模块
//! - `dye_batch_state_machine_service.rs` 作为 facade，保留 4 个 Service struct + new 构造函数
//!   + 11 个纯验证函数 + 10 个 DTOs + 单元测试
//! - 子模块通过 `use crate::services::dye_batch_state_machine_service::{...}` 复用 facade 的
//!   struct / DTOs / 纯函数，并为 struct 追加 impl 块（Rust 允许跨模块同一 struct 多个 impl 块）
//! - struct 的 `db` 字段为 `pub(crate)`，供本目录子模块访问

pub mod lifecycle_log;
pub mod operation;
pub mod rework;
pub mod state_rule;
