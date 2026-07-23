//! 应收账款-核销管理业务实现子模块（ar_ops/verification_ops）
//!
//! D10 拆分：原 `ar_ops/verification.rs`（1062 行）按 facade 模式拆分，
//! 业务方法实现迁移至本目录子模块。`ar_ops/verification.rs` 保留为门面（仅文档）。
//!
//! 各子模块扩展 `ArService` 的核销管理方法：
//! - `query`  查询类公开 API（list_verifications / get_verification /
//!   get_unverified_invoices / get_unverified_payments）
//! - `auto`   自动核销（auto_verify + 9 个内部辅助：数据加载 / 客户分组匹配 /
//!   核销单创建 / 明细批量插入 / 发票状态更新）
//! - `manual` 手动核销 + 取消核销（manual_verify / cancel_verification +
//!   7 个内部辅助：金额校验 / 锁定发票收款 / 余额检查 / 核销单与明细创建 /
//!   发票状态更新）
//!
//! 结构体定义与构造函数 `ArService::new` 位于 `crate::services::ar_service`（facade），
//! 子模块通过 `impl ArService { ... }` 扩展方法。Rust 允许同一 crate 多文件多 impl 块，
//! 各 impl 块的方法自动合并到 `ArService` 类型上，无需 re-export。

pub mod auto;
pub mod manual;
pub mod query;
