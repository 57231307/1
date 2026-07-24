//! 应收对账单主流程操作子模块（ar/recon_ops）
//!
//! 批次 D10 拆分自原 `ar/recon.rs`（1070 行）的 `impl ArReconciliationService`
//! 业务方法块，按业务子领域细分。每个子模块扩展 `ArReconciliationService` 的部分方法：
//! - `crud`       对账单 CRUD（create / get_by_id / list / update / get_with_details）
//! - `lifecycle`  对账单状态机（delete / send / close / update_status）
//!
//! 结构体定义与构造函数 `ArReconciliationService::new` 位于 `super`（`ar/mod.rs`），
//! 子模块通过 `impl super::super::ArReconciliationService { ... }` 扩展方法。
//! 测试模块保留在门面 `ar/recon.rs` 中。

pub mod crud;
pub mod lifecycle;
