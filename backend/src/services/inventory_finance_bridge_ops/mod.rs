//! 库存财务桥接 ops 子模块入口（inventory_finance_bridge_ops）
//!
//! 拆分：从原 inventory_finance_bridge_service.rs 迁移 InventoryFinanceBridgeService 的 impl 块。
//! - listener：事件监听器启动/关闭 + 事件分发处理（start_listener / shutdown_listener /
//!   handle_inventory_event_safe / handle_inventory_transaction）
//! - voucher：库存交易凭证生成（采购入库/销售出库/库存调整/生产入库/生产领料/
//!   采购退货/销售退货 7 类凭证 + build/validate/fetch/get 辅助方法）
//!
//! InventoryFinanceBridgeService struct 定义与 new 构造函数 + 参数对象 DTOs（VoucherItemArgs /
//! VoucherCreateArgs / BridgeVoucherArgs）保留在 facade inventory_finance_bridge_service 中，
//! impl 块分散到本子模块，Rust 允许同一 crate 多文件多 impl 块。

pub mod listener;
pub mod voucher;
