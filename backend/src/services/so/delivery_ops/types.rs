//! 销售发货服务的内部类型定义（delivery_ops/types）
//!
//! 批次 488 D10-3 拆分：从原 `so/delivery.rs` L99-124 迁移。
//! 3 个 struct 仅 delivery_ops 子模块内部使用，可见性 `pub(super)`（delivery_ops 可见）。

use rust_decimal::Decimal;

use crate::models::{sales_order, sales_order_item, warehouse};

// =====================================================
// ship_order 拆分辅助 struct
// =====================================================

/// 发货上下文：事务内加载的订单/明细/产品/仓库数据
pub(super) struct ShipOrderContext {
    pub(super) order: sales_order::Model,
    pub(super) order_items: Vec<sales_order_item::Model>,
    pub(super) product_map: std::collections::HashMap<i32, crate::models::product::Model>,
    pub(super) warehouse: warehouse::Model,
    pub(super) shipped_items_snapshot: Vec<(i32, rust_decimal::Decimal)>,
}

/// 发货明细处理结果：金额累计 + 待发布事件
pub(super) struct ShipmentItemsResult {
    pub(super) delivery_total_amount: Decimal,
    pub(super) delivery_total_tax: Decimal,
    pub(super) pending_inventory_events: Vec<crate::services::event_bus::BusinessEvent>,
}

/// 提交后上下文：用于事件发布
pub(super) struct ShipPostCommitContext {
    pub(super) ship_customer_id: i32,
    pub(super) ship_order_id: i32,
    pub(super) ship_items_for_event: Vec<crate::services::event_bus::ShippedItem>,
}
