//! 应收账款-核销管理门面（ar_ops/verification）
//!
//! D10 拆分：原 `ar_ops/verification.rs`（1062 行）按 facade 模式拆分，
//! 业务方法实现迁移至 `ar_ops/verification_ops/` 子模块（query / auto / manual）。
//! 本文件保留为门面，仅含模块文档说明。
//!
//! 原始迁移历史：批次 488 D10-1 从原 `ar_service.rs` L753-1778 迁移至本模块。
//!
//! 核销管理 23 个方法分布：
//! - 查询类（4，公开 API）：list_verifications / get_verification /
//!   get_unverified_invoices / get_unverified_payments
//!   → `verification_ops::query`
//! - 自动核销（1 公开 + 9 内部辅助）：auto_verify
//!   → `verification_ops::auto`
//! - 手动核销 + 取消核销（2 公开 + 7 内部辅助）：manual_verify /
//!   cancel_verification
//!   → `verification_ops::manual`
//!
//! 业务规则：
//! - 核销基于 ar_reconciliation + ar_reconciliation_item 表实现
//! - ar_reconciliations 作为核销单主表（reconciliation_status = CLOSED/CANCELLED）
//! - ar_reconciliation_items 记录每笔核销明细（INVOICE/RECEIPT 类型）
//! - 自动核销按客户分组贪心匹配；手动核销单张发票 + 单张收款单
//!
//! 模块层级关系：
//! - `verification_ops` 与 `verification` 同为 `ar_ops` 下的兄弟模块
//! - `ArService` struct + `new` 定义在 `crate::services::ar_service`（facade）
//! - `verification_ops` 子模块通过 `impl ArService { ... }` 扩展方法
//! - impl 块分散在子模块中，Rust 允许同一 crate 多文件多 impl 块
//! - 各 impl 块的方法自动合并到 `ArService` 类型上，外部调用路径不变
