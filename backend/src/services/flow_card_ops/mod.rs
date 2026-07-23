//! 流转卡与工序流转业务实现子模块（flow_card_ops）
//!
//! D10 第 5 批拆分：从原 flow_card_service.rs 迁移 4 个 Service 的 impl 块。
//! struct 定义 + new 构造函数 + 9 个 DTOs + 5 个纯函数 + 单元测试保留在 facade flow_card_service.rs。
//!
//! 模块层级关系：
//! - flow_card_ops 与 flow_card_service 同为 crate::services 下的兄弟模块
//! - flow_card_service.rs 作为 facade，保留 4 个 Service struct + new 方法 + DTOs + 纯函数 + 测试
//! - 子模块 impl facade 定义的 struct（依赖 db 字段为 pub(crate)）
//! - 子模块通过 use crate::services::flow_card_service::{...} 复用 facade 的 DTOs 和纯函数

pub mod card_crud;
pub mod card_state;
pub mod feedback;
pub mod route;
pub mod step;
