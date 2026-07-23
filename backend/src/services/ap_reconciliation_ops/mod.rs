//! 供应商对账服务的业务实现子模块（ap_reconciliation_ops）
//!
//! D10-5 拆分：从原 `ap_reconciliation_service.rs` 迁移 ApReconciliationService 的 impl 块。
//! struct 定义 + new 构造函数 + 单号生成宏 + 单元测试保留在 facade `ap_reconciliation_service.rs`。
//!
//! 模块层级关系：
//! - `ap_reconciliation_ops` 与 `ap_reconciliation_service` 同为 `crate::services` 下的兄弟模块
//! - `ap_reconciliation_service.rs` 作为 facade，保留 ApReconciliationService struct + new 方法 + 宏 + 测试
//! - 子模块 impl facade 定义的 ApReconciliationService（依赖 db 字段为 pub(crate)）
//! - 子模块通过 `use crate::services::ap_reconciliation_service::ApReconciliationService` 复用 Service
//! - DTOs 迁移至 `types`，facade 通过 `pub use` 重新导出外部与测试实际使用的 DTO

pub mod auto;
pub mod confirm;
pub mod crud;
pub mod report;
pub mod types;
