//! 应收对账 - 核销服务门面（ar/vfy）
//!
//! 批次 490 D10-4b：原 `ar/vfy.rs`（1368 行）按 facade 模式拆分，业务方法实现
//! 迁移至 `ar/vfy_ops/` 子模块（match / aging / reconciliation / confirm）。
//! 本文件保留为门面：重新导出公共 DTO 与 `ArReconciliationService`，并保留测试模块。
//!
//! 高级对账算法（实现见 `vfy_ops`）：
//! - `auto_match`         自动对账：精确金额 + 日期顺序 + 客户汇总三种策略
//! - `get_aging_report`   账龄分桶分析（5 档：当期 / 1-30 / 31-60 / 61-90 / 90+）
//! - `generate_reconciliation` 自动生成对账单（含明细行）
//! - `customer_confirm` / `customer_dispute` 带状态校验的客户操作
//!
//! 拆分自原 `ar_reconciliation_service.rs` 的 `// 增强功能` 段。
//! 结构体定义与构造函数 `ArReconciliationService::new` 位于 `super`（`ar/mod.rs`）。

// 重新导出 Service 结构体与测试中使用的 DTO，保持 `crate::services::ar::vfy::*` 路径稳定
// 其余 DTO（AgingBucket/AgingReport/AutoMatchResult/CustomerAgingSummary/GenerateReconciliationRequest）
// 已由 ar/mod.rs 定义并通过 services/mod.rs re-export，无需在此重复 re-export
