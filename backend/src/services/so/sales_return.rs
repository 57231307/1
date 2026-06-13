//! 销售退货服务（so/return_rs）
//!
//! 占位模块。`return` 是 Rust 关键字，文件/模块名使用 `return_rs`。
//! 实际销售退货业务已由 `services/sales_return_service.rs` 提供。
//! 本模块仅作为按业务子领域拆分的目录占位，保留扩展空间。
//!
//! 如需在此模块内扩展退货业务，请：
//! 1. `impl SalesService` 复用 `services::so::order::SalesService`
//! 2. 添加销售退货单、退货审批、退货入库等方法
//! 3. 保持 `crate::impl_generate_no!` 单据号生成器宏的复用模式

#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。
