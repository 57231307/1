//! 应收对账核销操作子模块（ar/vfy_ops）
//!
//! 批次 490 D10-4b 拆分自原 `ar/vfy.rs`（1368 行）的 `impl ArReconciliationService`
//! 业务方法块，按业务子领域细分。每个子模块扩展 `ArReconciliationService` 的部分方法：
//! - `match`          自动对账匹配（`auto_match` + 明细创建辅助）
//! - `aging`          账龄分桶报告（`get_aging_report` + 分桶辅助）
//! - `reconciliation` 自动生成对账单（`generate_reconciliation` + 拉取/构造辅助）
//! - `confirm`        客户确认/争议（`customer_confirm` / `customer_dispute`）
//!
//! 结构体定义与构造函数 `ArReconciliationService::new` 位于 `super`（`ar/mod.rs`），
//! 子模块通过 `impl super::ArReconciliationService { ... }` 扩展方法。
//! 测试模块保留在门面 `ar/vfy.rs` 中。

pub mod r#match;
pub mod aging;
pub mod reconciliation;
pub mod confirm;
