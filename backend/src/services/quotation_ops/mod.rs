//! 销售报价单业务实现子模块（quotation_ops）
//!
//! D11 拆分：从原 `quotation_service.rs` 迁移 QuotationService 的 impl 块。
//! struct 定义 + new/from_state 构造函数 + ServiceError 枚举 + 单元测试
//! 保留在 facade `quotation_service.rs`。
//!
//! 模块层级关系：
//! - `quotation_ops` 与 `quotation_service` 同为 `crate::services` 下的兄弟模块
//! - `quotation_service.rs` 作为 facade，保留 QuotationService struct + 构造函数 + ServiceError + 测试
//! - 子模块 impl facade 定义的 QuotationService（依赖 db 字段为 pub(crate)）
//! - 子模块通过 `use crate::services::quotation_service::{...}` 复用 facade 的类型

pub mod calc;
pub mod crud;
pub mod lifecycle;
pub mod types;
pub mod update;
