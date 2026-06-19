//! 库存预留服务（inv/hold）
//!
//! 占位模块。原 `inventory_transfer_service.rs` 不包含独立的"库存预留"方法。
//! 库存预留业务已由顶层 `services/inventory_reservation_service.rs` 独立提供（含
//! 销售订单锁库、采购订单锁库、超期自动释放、释放转可用量等）。
//! 本模块保留扩展空间，可用于后续：
//! - 调拨预留（中途状态锁库）
//! - 调拨占用统计
//! - 跨仓库预留优先级
//!
//! 实际预留请使用：
//! - `crate::services::inventory_reservation_service::InventoryReservationService`

