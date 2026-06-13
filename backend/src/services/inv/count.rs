//! 库存盘点服务（inv/count）
//!
//! 占位模块。原 `inventory_transfer_service.rs` 不包含独立的"库存盘点"方法。
//! 库存盘点业务已由顶层 `services/inventory_count_service.rs` 独立提供（含盘盈盘亏处理、
//! 盘点差异生成库存调整单等完整流程）。
//! 本模块保留扩展空间，可用于后续：
//! - 调拨前自动盘点（冻结库存）
//! - 调拨后盘点对账
//! - 抽盘 + 复盘工作流
//!
//! 实际盘点请使用：
//! - `crate::services::inventory_count_service::InventoryCountService`

#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
