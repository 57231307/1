//! 产量工资服务的业务实现子模块（wage_ops）
//!
//! 批次 490 D10-4a 拆分：从原 `wage_service.rs` 迁移 3 个 Service 的 impl 块。
//! struct 定义 + new 构造函数 + DTOs + 9 个纯函数 + 测试保留在 facade `wage_service.rs`。
//!
//! 模块层级关系：
//! - `wage_ops` 与 `wage_service` 同为 `crate::services` 下的兄弟模块
//! - `wage_service.rs` 作为 facade，保留 3 个 Service struct + new 方法 + DTOs + 纯函数 + 测试
//! - 子模块 impl facade 定义的 struct（依赖 db 字段为 pub(crate)）
//! - 子模块通过 `use crate::services::wage_service::{...}` 复用 facade 的 DTOs 和纯函数

pub mod calculation;
pub mod rate;
pub mod record;
