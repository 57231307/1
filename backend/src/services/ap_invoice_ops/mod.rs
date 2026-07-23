//! 应付单服务的业务实现子模块（ap_invoice_ops）
//!
//! 批次 490 D10-4b 拆分：从原 `ap_invoice_service.rs` 迁移 ApInvoiceService 的 impl 块。
//! struct 定义 + new 构造函数 + ApInvoiceListQuery + 校验纯函数 + 单号生成宏 + 单元测试
//! 保留在 facade `ap_invoice_service.rs`。
//!
//! 模块层级关系：
//! - `ap_invoice_ops` 与 `ap_invoice_service` 同为 `crate::services` 下的兄弟模块
//! - `ap_invoice_service.rs` 作为 facade，保留 ApInvoiceService struct + new 方法 + 校验纯函数 + 测试
//! - 子模块 impl facade 定义的 ApInvoiceService（依赖 db 字段为 pub(crate)）
//! - 子模块通过 `use crate::services::ap_invoice_service::{...}` 复用 facade 的 DTOs 和 Service

pub mod crud;
pub mod receipt;
pub mod report;
pub mod types;
