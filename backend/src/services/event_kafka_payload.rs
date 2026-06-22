//! 业务事件 Kafka 线格式序列化（与 BusinessEvent 字段一一对应）
//!
//! 拆分自 event_kafka.rs：原 pub mod payload_serde { ... } 内部块。
//! 包含 EventPayload 枚举 + From<&BusinessEvent> + TryFrom<EventPayload> 三段实现。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::services::event_bus::{BusinessEvent, ShippedItem};

/// 为 `BusinessEvent` 增加 `Serialize` / `Deserialize` 派生（仅在 kafka 模块内使用）
///
/// 原 `BusinessEvent` 派生来自 `event_bus.rs`，没有 `Serialize`。这里通过新类型
/// `EventPayload` 包装，再借助 `serde_json` 透明转换，避免在 8 个必需文件之外
/// 改动 `event_bus.rs` 的公共定义。
pub mod payload_serde {
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};

    use crate::services::event_bus::{BusinessEvent, ShippedItem};

    /// 与 `BusinessEvent` 字段一一对应的可序列化结构
    #[derive(Serialize, Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    pub enum EventPayload {
        PurchaseReceiptCompleted {
            receipt_id: i32,
            order_id: i32,
            supplier_id: i32,
        },
        SalesOrderShipped {
            order_id: i32,
            customer_id: i32,
            items: Vec<ShippedItem>,
        },
        PaymentCompleted {
            payment_id: i32,
            invoice_id: i32,
            amount: Decimal,
        },
        InventoryAdjusted {
            product_id: i32,
            warehouse_id: i32,
            quantity_change: Decimal,
        },
        CollectionCompleted {
            collection_id: i32,
            invoice_id: Option<i32>,
            amount: Decimal,
        },
        PurchaseOrderApproved {
            order_id: i32,
            supplier_id: i32,
        },
        InventoryCountCompleted {
            count_id: i32,
            variance_count: i32,
        },
        BpmProcessFinished {
            business_type: String,
            business_id: i32,
            approved: bool,
        },
        LowStockAlert {
            product_id: i32,
            warehouse_id: i32,
            current_quantity: Decimal,
            reorder_point: Decimal,
            reorder_quantity: Decimal,
        },
        FinancialIndicatorUpdate {
            period: String,
            trigger_source: String,
        },
        MaterialShortageAlert {
            material_id: i32,
            material_name: String,
            material_code: String,
            required_quantity: Decimal,
            available_quantity: Decimal,
            shortage_quantity: Decimal,
            shortage_level: String,
            affected_orders_count: i32,
        },
        InventoryTransactionCreated {
            transaction_id: i32,
            transaction_type: String,
            product_id: i32,
            warehouse_id: i32,
            quantity_meters: Decimal,
            quantity_kg: Decimal,
            source_bill_type: Option<String>,
            source_bill_no: Option<String>,
            source_bill_id: Option<i32>,
            batch_no: String,
            color_no: String,
            created_by: Option<i32>,
        },
    }

    impl From<&BusinessEvent> for EventPayload {
        fn from(event: &BusinessEvent) -> Self {
            match event {
                BusinessEvent::PurchaseReceiptCompleted {
                    receipt_id,
                    order_id,
                    supplier_id,
                } => Self::PurchaseReceiptCompleted {
                    receipt_id: *receipt_id,
                    order_id: *order_id,
                    supplier_id: *supplier_id,
                },
                BusinessEvent::SalesOrderShipped {
                    order_id,
                    customer_id,
                    items,
                } => Self::SalesOrderShipped {
                    order_id: *order_id,
                    customer_id: *customer_id,
                    items: items.clone(),
                },
                BusinessEvent::PaymentCompleted {
                    payment_id,
                    invoice_id,
                    amount,
                } => Self::PaymentCompleted {
                    payment_id: *payment_id,
                    invoice_id: *invoice_id,
                    amount: *amount,
                },
                BusinessEvent::InventoryAdjusted {
                    product_id,
                    warehouse_id,
                    quantity_change,
                } => Self::InventoryAdjusted {
                    product_id: *product_id,
                    warehouse_id: *warehouse_id,
                    quantity_change: *quantity_change,
                },
                BusinessEvent::CollectionCompleted {
                    collection_id,
                    invoice_id,
                    amount,
                } => Self::CollectionCompleted {
                    collection_id: *collection_id,
                    invoice_id: *invoice_id,
                    amount: *amount,
                },
                BusinessEvent::PurchaseOrderApproved {
                    order_id,
                    supplier_id,
                } => Self::PurchaseOrderApproved {
                    order_id: *order_id,
                    supplier_id: *supplier_id,
                },
                BusinessEvent::InventoryCountCompleted {
                    count_id,
                    variance_count,
                } => Self::InventoryCountCompleted {
                    count_id: *count_id,
                    variance_count: *variance_count,
                },
                BusinessEvent::BpmProcessFinished {
                    business_type,
                    business_id,
                    approved,
                } => Self::BpmProcessFinished {
                    business_type: business_type.clone(),
                    business_id: *business_id,
                    approved: *approved,
                },
                BusinessEvent::LowStockAlert {
                    product_id,
                    warehouse_id,
                    current_quantity,
                    reorder_point,
                    reorder_quantity,
                } => Self::LowStockAlert {
                    product_id: *product_id,
                    warehouse_id: *warehouse_id,
                    current_quantity: *current_quantity,
                    reorder_point: *reorder_point,
                    reorder_quantity: *reorder_quantity,
                },
                BusinessEvent::FinancialIndicatorUpdate {
                    period,
                    trigger_source,
                } => Self::FinancialIndicatorUpdate {
                    period: period.clone(),
                    trigger_source: trigger_source.clone(),
                },
                BusinessEvent::MaterialShortageAlert {
                    material_id,
                    material_name,
                    material_code,
                    required_quantity,
                    available_quantity,
                    shortage_quantity,
                    shortage_level,
                    affected_orders_count,
                } => Self::MaterialShortageAlert {
                    material_id: *material_id,
                    material_name: material_name.clone(),
                    material_code: material_code.clone(),
                    required_quantity: *required_quantity,
                    available_quantity: *available_quantity,
                    shortage_quantity: *shortage_quantity,
                    shortage_level: shortage_level.clone(),
                    affected_orders_count: *affected_orders_count,
                },
                BusinessEvent::InventoryTransactionCreated {
                    transaction_id,
                    transaction_type,
                    product_id,
                    warehouse_id,
                    quantity_meters,
                    quantity_kg,
                    source_bill_type,
                    source_bill_no,
                    source_bill_id,
                    batch_no,
                    color_no,
                    created_by,
                } => Self::InventoryTransactionCreated {
                    transaction_id: *transaction_id,
                    transaction_type: transaction_type.clone(),
                    product_id: *product_id,
                    warehouse_id: *warehouse_id,
                    quantity_meters: *quantity_meters,
                    quantity_kg: *quantity_kg,
                    source_bill_type: source_bill_type.clone(),
                    source_bill_no: source_bill_no.clone(),
                    source_bill_id: *source_bill_id,
                    batch_no: batch_no.clone(),
                    color_no: color_no.clone(),
                    created_by: *created_by,
                },
            }
        }
    }

    impl TryFrom<EventPayload> for BusinessEvent {
        type Error = String;
        fn try_from(p: EventPayload) -> Result<Self, Self::Error> {
            Ok(match p {
                EventPayload::PurchaseReceiptCompleted {
                    receipt_id,
                    order_id,
                    supplier_id,
                } => Self::PurchaseReceiptCompleted {
                    receipt_id,
                    order_id,
                    supplier_id,
                },
                EventPayload::SalesOrderShipped {
                    order_id,
                    customer_id,
                    items,
                } => Self::SalesOrderShipped {
                    order_id,
                    customer_id,
                    items,
                },
                EventPayload::PaymentCompleted {
                    payment_id,
                    invoice_id,
                    amount,
                } => Self::PaymentCompleted {
                    payment_id,
                    invoice_id,
                    amount,
                },
                EventPayload::InventoryAdjusted {
                    product_id,
                    warehouse_id,
                    quantity_change,
                } => Self::InventoryAdjusted {
                    product_id,
                    warehouse_id,
                    quantity_change,
                },
                EventPayload::CollectionCompleted {
                    collection_id,
                    invoice_id,
                    amount,
                } => Self::CollectionCompleted {
                    collection_id,
                    invoice_id,
                    amount,
                },
                EventPayload::PurchaseOrderApproved {
                    order_id,
                    supplier_id,
                } => Self::PurchaseOrderApproved {
                    order_id,
                    supplier_id,
                },
                EventPayload::InventoryCountCompleted {
                    count_id,
                    variance_count,
                } => Self::InventoryCountCompleted {
                    count_id,
                    variance_count,
                },
                EventPayload::BpmProcessFinished {
                    business_type,
                    business_id,
                    approved,
                } => Self::BpmProcessFinished {
                    business_type,
                    business_id,
                    approved,
                },
                EventPayload::LowStockAlert {
                    product_id,
                    warehouse_id,
                    current_quantity,
                    reorder_point,
                    reorder_quantity,
                } => Self::LowStockAlert {
                    product_id,
                    warehouse_id,
                    current_quantity,
                    reorder_point,
                    reorder_quantity,
                },
                EventPayload::FinancialIndicatorUpdate {
                    period,
                    trigger_source,
                } => Self::FinancialIndicatorUpdate {
                    period,
                    trigger_source,
                },
                EventPayload::MaterialShortageAlert {
                    material_id,
                    material_name,
                    material_code,
                    required_quantity,
                    available_quantity,
                    shortage_quantity,
                    shortage_level,
                    affected_orders_count,
                } => Self::MaterialShortageAlert {
                    material_id,
                    material_name,
                    material_code,
                    required_quantity,
                    available_quantity,
                    shortage_quantity,
                    shortage_level,
                    affected_orders_count,
                },
                EventPayload::InventoryTransactionCreated {
                    transaction_id,
                    transaction_type,
                    product_id,
                    warehouse_id,
                    quantity_meters,
                    quantity_kg,
                    source_bill_type,
                    source_bill_no,
                    source_bill_id,
                    batch_no,
                    color_no,
                    created_by,
                } => Self::InventoryTransactionCreated {
                    transaction_id,
                    transaction_type,
                    product_id,
                    warehouse_id,
                    quantity_meters,
                    quantity_kg,
                    source_bill_type,
                    source_bill_no,
                    source_bill_id,
                    batch_no,
                    color_no,
                    created_by,
                },
            })
        }
    }
}

// 重导出 EventPayload 给外部直接访问
pub use payload_serde::EventPayload;
