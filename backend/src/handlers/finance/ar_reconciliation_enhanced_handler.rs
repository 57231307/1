//! 应收对账增强 Handler 兼容层
//!
//! 历史原因：原 ar_reconciliation_enhanced_handler.rs 中的 fn 已合并到
//! ar_reconciliation_handler.rs（应收对账基础版）。本文件保留原模块名以便
//! `crate::handlers::ar_reconciliation_enhanced_handler::xxx` 的旧引用仍能工作。
//! 新代码请直接使用 `crate::handlers::ar_reconciliation_handler::xxx`。

pub use crate::handlers::ar_reconciliation_handler::*;
