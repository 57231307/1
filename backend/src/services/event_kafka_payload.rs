//! 业务事件 Kafka 线格式序列化（与 BusinessEvent 字段一一对应）
//!
//! 拆分自 event_kafka.rs：原 pub mod payload_serde { ... } 内部块。
//! 包含 EventPayload 枚举 + From<&BusinessEvent> + TryFrom<EventPayload> 三段实现。

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
        // B-P1-4 修复（批次 361 v13 复审）：销售订单状态变更事件
        SalesOrderSubmitted {
            order_id: i32,
            customer_id: i32,
            user_id: i32,
        },
        SalesOrderApproved {
            order_id: i32,
            customer_id: i32,
            user_id: i32,
        },
        SalesOrderCompleted {
            order_id: i32,
            customer_id: i32,
            user_id: i32,
        },
        SalesOrderCancelled {
            order_id: i32,
            customer_id: i32,
            user_id: i32,
        },
        SalesOrderRejected {
            order_id: i32,
            customer_id: i32,
            user_id: i32,
        },
        PaymentCompleted {
            payment_id: i32,
            invoice_id: i32,
            amount: Decimal,
            user_id: i32,
        },
        CollectionCompleted {
            collection_id: i32,
            invoice_id: Option<i32>,
            amount: Decimal,
            /// P1 1-1 修复（批次 78 v1 复审）：收款操作人 ID
            user_id: i32,
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
            /// P2 5-18 修复：审批人 ID（从 BPM 事件 payload 携带）
            approver_id: i32,
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
        // B-P1-3 修复（批次 384 v13 复审）：客户/供应商主数据变更事件
        CustomerUpdated {
            customer_id: i32,
            customer_name: String,
            user_id: i32,
        },
        SupplierUpdated {
            supplier_id: i32,
            supplier_name: String,
            user_id: i32,
        },
        // v14 批次 420 修复 T-P1-3：染色完成/质检完成事件
        DyeBatchCompleted {
            batch_id: i32,
            batch_no: String,
            color_no: Option<String>,
            greige_fabric_id: Option<i32>,
            planned_quantity: Option<Decimal>,
            completed_by: Option<i32>,
        },
        QualityInspectionCompleted {
            inspection_id: i32,
            batch_id: Option<i32>,
            product_id: i32,
            result: String,
            inspector_id: Option<i32>,
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
                BusinessEvent::SalesOrderSubmitted {
                    order_id,
                    customer_id,
                    user_id,
                } => Self::SalesOrderSubmitted {
                    order_id: *order_id,
                    customer_id: *customer_id,
                    user_id: *user_id,
                },
                BusinessEvent::SalesOrderApproved {
                    order_id,
                    customer_id,
                    user_id,
                } => Self::SalesOrderApproved {
                    order_id: *order_id,
                    customer_id: *customer_id,
                    user_id: *user_id,
                },
                BusinessEvent::SalesOrderCompleted {
                    order_id,
                    customer_id,
                    user_id,
                } => Self::SalesOrderCompleted {
                    order_id: *order_id,
                    customer_id: *customer_id,
                    user_id: *user_id,
                },
                BusinessEvent::SalesOrderCancelled {
                    order_id,
                    customer_id,
                    user_id,
                } => Self::SalesOrderCancelled {
                    order_id: *order_id,
                    customer_id: *customer_id,
                    user_id: *user_id,
                },
                BusinessEvent::SalesOrderRejected {
                    order_id,
                    customer_id,
                    user_id,
                } => Self::SalesOrderRejected {
                    order_id: *order_id,
                    customer_id: *customer_id,
                    user_id: *user_id,
                },
                BusinessEvent::PaymentCompleted {
                    payment_id,
                    invoice_id,
                    amount,
                    user_id,
                } => Self::PaymentCompleted {
                    payment_id: *payment_id,
                    invoice_id: *invoice_id,
                    amount: *amount,
                    user_id: *user_id,
                },
                BusinessEvent::CollectionCompleted {
                    collection_id,
                    invoice_id,
                    amount,
                    user_id,
                } => Self::CollectionCompleted {
                    collection_id: *collection_id,
                    invoice_id: *invoice_id,
                    amount: *amount,
                    user_id: *user_id,
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
                    approver_id,
                } => Self::BpmProcessFinished {
                    business_type: business_type.clone(),
                    business_id: *business_id,
                    approved: *approved,
                    approver_id: *approver_id,
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
                BusinessEvent::CustomerUpdated {
                    customer_id,
                    customer_name,
                    user_id,
                } => Self::CustomerUpdated {
                    customer_id: *customer_id,
                    customer_name: customer_name.clone(),
                    user_id: *user_id,
                },
                BusinessEvent::SupplierUpdated {
                    supplier_id,
                    supplier_name,
                    user_id,
                } => Self::SupplierUpdated {
                    supplier_id: *supplier_id,
                    supplier_name: supplier_name.clone(),
                    user_id: *user_id,
                },
                // v14 批次 420 修复 T-P1-3：染色完成/质检完成事件转换
                BusinessEvent::DyeBatchCompleted {
                    batch_id,
                    batch_no,
                    color_no,
                    greige_fabric_id,
                    planned_quantity,
                    completed_by,
                } => Self::DyeBatchCompleted {
                    batch_id: *batch_id,
                    batch_no: batch_no.clone(),
                    color_no: color_no.clone(),
                    greige_fabric_id: *greige_fabric_id,
                    planned_quantity: *planned_quantity,
                    completed_by: *completed_by,
                },
                BusinessEvent::QualityInspectionCompleted {
                    inspection_id,
                    batch_id,
                    product_id,
                    result,
                    inspector_id,
                } => Self::QualityInspectionCompleted {
                    inspection_id: *inspection_id,
                    batch_id: *batch_id,
                    product_id: *product_id,
                    result: result.clone(),
                    inspector_id: *inspector_id,
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
                EventPayload::SalesOrderSubmitted {
                    order_id,
                    customer_id,
                    user_id,
                } => Self::SalesOrderSubmitted {
                    order_id,
                    customer_id,
                    user_id,
                },
                EventPayload::SalesOrderApproved {
                    order_id,
                    customer_id,
                    user_id,
                } => Self::SalesOrderApproved {
                    order_id,
                    customer_id,
                    user_id,
                },
                EventPayload::SalesOrderCompleted {
                    order_id,
                    customer_id,
                    user_id,
                } => Self::SalesOrderCompleted {
                    order_id,
                    customer_id,
                    user_id,
                },
                EventPayload::SalesOrderCancelled {
                    order_id,
                    customer_id,
                    user_id,
                } => Self::SalesOrderCancelled {
                    order_id,
                    customer_id,
                    user_id,
                },
                EventPayload::SalesOrderRejected {
                    order_id,
                    customer_id,
                    user_id,
                } => Self::SalesOrderRejected {
                    order_id,
                    customer_id,
                    user_id,
                },
                EventPayload::PaymentCompleted {
                    payment_id,
                    invoice_id,
                    amount,
                    user_id,
                } => Self::PaymentCompleted {
                    payment_id,
                    invoice_id,
                    amount,
                    user_id,
                },
                EventPayload::CollectionCompleted {
                    collection_id,
                    invoice_id,
                    amount,
                    user_id,
                } => Self::CollectionCompleted {
                    collection_id,
                    invoice_id,
                    amount,
                    user_id,
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
                    approver_id,
                } => Self::BpmProcessFinished {
                    business_type,
                    business_id,
                    approved,
                    approver_id,
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
                EventPayload::CustomerUpdated {
                    customer_id,
                    customer_name,
                    user_id,
                } => Self::CustomerUpdated {
                    customer_id,
                    customer_name,
                    user_id,
                },
                EventPayload::SupplierUpdated {
                    supplier_id,
                    supplier_name,
                    user_id,
                } => Self::SupplierUpdated {
                    supplier_id,
                    supplier_name,
                    user_id,
                },
                // v14 批次 420 修复 T-P1-3：染色完成/质检完成事件反向转换
                EventPayload::DyeBatchCompleted {
                    batch_id,
                    batch_no,
                    color_no,
                    greige_fabric_id,
                    planned_quantity,
                    completed_by,
                } => Self::DyeBatchCompleted {
                    batch_id,
                    batch_no,
                    color_no,
                    greige_fabric_id,
                    planned_quantity,
                    completed_by,
                },
                EventPayload::QualityInspectionCompleted {
                    inspection_id,
                    batch_id,
                    product_id,
                    result,
                    inspector_id,
                } => Self::QualityInspectionCompleted {
                    inspection_id,
                    batch_id,
                    product_id,
                    result,
                    inspector_id,
                },
            })
        }
    }
}

// 重导出 EventPayload 给外部直接访问
pub use payload_serde::EventPayload;
