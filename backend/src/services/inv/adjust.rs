//! 库存调整服务（inv/adjust）
//!
//! 占位模块。原 `inventory_transfer_service.rs` 不包含独立的"库存调整"方法。
//! 库存调整业务已由顶层 `services/inventory_adjustment_service.rs` 独立提供。
//! 本模块保留扩展空间，可用于后续在调拨（move）流程中集成：
//! - 调拨差异自动调整
//! - 调拨出/入库的尾差处理
//! - 多仓库联动调整建议
//!
//! 实际库存调整单据请使用：
//! - `crate::services::inventory_adjustment_service::InventoryAdjustmentService`

#![allow(dead_code)]
