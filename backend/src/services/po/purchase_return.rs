//! 采购退货服务（po/purchase_return）
//!
//! 占位模块。`return` 是 Rust 关键字，文件/模块名使用 `purchase_return`。
//! 实际采购退货业务已由 `services/purchase_return_service.rs` 提供。
//! 本模块仅作为按业务子领域拆分的目录占位，保留扩展空间。
//!
//! 如需在此模块内扩展退货业务，请：
//! 1. `impl PurchaseOrderService` 复用 `services::po::order::PurchaseOrderService`
//! 2. 在 `super::order` 中按业务对象添加退货相关方法
//! 3. 保持 `crate::impl_generate_no!` 单据号生成器宏的复用模式

#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。

// 当前模块暂未包含具体实现，作为业务子领域占位。
// 后续扩展时建议在此文件中添加：
// - 采购退货单创建
// - 退货审批工作流
// - 退货出库联动
