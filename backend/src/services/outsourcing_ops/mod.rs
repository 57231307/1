//! 委外加工 ops 子模块入口（outsourcing_ops）
//!
//! 批次 489 D10-2b 拆分：从原 `outsourcing_service.rs` 拆出 4 个 Service impl 块。
//! - `order`：OutsourcingOrderService impl（订单 CRUD + 状态机 + 收回损耗计算）
//! - `order_item`：OutsourcingOrderItemService impl（发料明细 CRUD）
//! - `receipt`：OutsourcingReceiptService impl（收回入库单 CRUD + confirm）
//! - `voucher`：OutsourcingVoucherService impl（凭证 CRUD + post）
//! - `types`：10 个 DTO struct
//!
//! 4 个 Service struct 定义与 `new` 构造函数保留在 facade `outsourcing_service` 中，
//! impl 块分散到本子模块，Rust 允许同一 crate 多文件多 impl 块。

pub mod order;
pub mod order_item;
pub mod receipt;
pub mod types;
pub mod voucher;

// re-export DTOs，facade 通过 `pub use` 二次 re-export 保持外部引用路径不变
pub use types::{
    CreateOutsourcingOrderItemRequest, CreateOutsourcingOrderRequest, CreateOutsourcingReceiptRequest,
    CreateOutsourcingVoucherRequest, OutsourcingOrderQuery, OutsourcingReceiptQuery,
    OutsourcingVoucherQuery, UpdateOutsourcingOrderItemRequest, UpdateOutsourcingOrderRequest,
    UpdateOutsourcingReceiptRequest,
};
