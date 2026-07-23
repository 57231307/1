//! 凭证服务的业务实现子模块（voucher_ops）
//!
//! 批次 488 D10-2a 拆分：从原 `voucher_service.rs` L115-1310 迁移 impl VoucherService 方法。
//!
//! 模块层级关系：
//! - `voucher_ops` 与 `voucher_service` 同为 `crate::services` 下的兄弟模块
//! - `VoucherService` struct 定义在 `crate::services::voucher_service`（facade）
//! - 各子模块通过 `use crate::services::voucher_service::VoucherService;` 直接导入，编写各自 `impl VoucherService` 块
//! - `db` 字段在 facade 中声明为 `pub(crate)`，本模块 impl 块可直接访问 `self.db`
//! - impl 块分散在子模块中，Rust 允许同一 crate 多文件多 impl 块

pub mod assist;
pub mod balance;
pub mod crud;
pub mod workflow;
