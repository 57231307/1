//! 应收对账 - 付款服务（ar/pay）
//!
//! 占位模块。原 `ar_reconciliation_service.rs` 不包含独立的"付款"服务方法。
//! 实际付款/收款业务已由顶层 `services/ar_collection_service.rs` 独立提供（含
//! 收款单创建、核销、退款、银行流水对账等）。
//!
//! 本模块保留扩展空间，可用于后续在 `ArReconciliationService` 中加入：
//! - 付款计划与对账单的联动
//! - 分期收款的账龄计算
//! - 预收款/订金处理
//!
//! 实际收款请使用：
//! - `crate::services::ar_collection_service::ArCollectionService`

#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
