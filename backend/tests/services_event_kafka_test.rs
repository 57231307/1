#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use std::str::FromStr;
    use super::*;

    fn _sample_event() -> BusinessEvent {
        BusinessEvent::PaymentCompleted {
            payment_id: 1,
            invoice_id: 2,
            amount: Decimal::from_str("100.50").unwrap(),
            user_id: 100,
        }
    }

    /// 验证 `EventPayload` 双向转换覆盖所有 variant
    #[test]
    fn test_payload_all_variants_round_trip() {
        let cases: Vec<BusinessEvent> = vec![
            BusinessEvent::PurchaseReceiptCompleted {
                receipt_id: 1,
                order_id: 2,
                supplier_id: 3,
            },
            BusinessEvent::SalesOrderShipped {
                order_id: 1,
                customer_id: 2,
                items: vec![ShippedItem {
                    product_id: 3,
                    quantity: Decimal::from(5),
                }],
            },
            BusinessEvent::SalesOrderSubmitted {
                order_id: 1,
                customer_id: 2,
                user_id: 10,
            },
            BusinessEvent::SalesOrderApproved {
                order_id: 1,
                customer_id: 2,
                user_id: 10,
            },
            BusinessEvent::SalesOrderCompleted {
                order_id: 1,
                customer_id: 2,
                user_id: 10,
            },
            BusinessEvent::SalesOrderCancelled {
                order_id: 1,
                customer_id: 2,
                user_id: 10,
            },
            BusinessEvent::SalesOrderRejected {
                order_id: 1,
                customer_id: 2,
                user_id: 10,
            },
            BusinessEvent::PaymentCompleted {
                payment_id: 1,
                invoice_id: 2,
                amount: Decimal::from(10),
                user_id: 10,
            },
            BusinessEvent::CollectionCompleted {
                collection_id: 1,
                invoice_id: Some(2),
                amount: Decimal::from(20),
                user_id: 0,
            },
            BusinessEvent::PurchaseOrderApproved {
                order_id: 1,
                supplier_id: 2,
            },
            BusinessEvent::InventoryCountCompleted {
                count_id: 1,
                variance_count: 3,
            },
            BusinessEvent::BpmProcessFinished {
                business_type: "purchase_order".to_string(),
                business_id: 1,
                approved: true,
                approver_id: 0,
            },
            BusinessEvent::LowStockAlert {
                product_id: 1,
                warehouse_id: 2,
                current_quantity: Decimal::from(1),
                reorder_point: Decimal::from(5),
                reorder_quantity: Decimal::from(10),
            },
            BusinessEvent::FinancialIndicatorUpdate {
                period: "2026-Q2".to_string(),
                trigger_source: "test".to_string(),
            },
            BusinessEvent::MaterialShortageAlert {
                material_id: 1,
                material_name: "棉布".to_string(),
                material_code: "COT-001".to_string(),
                required_quantity: Decimal::from(100),
                available_quantity: Decimal::from(20),
                shortage_quantity: Decimal::from(80),
                shortage_level: "HIGH".to_string(),
                affected_orders_count: 3,
            },
            BusinessEvent::InventoryTransactionCreated {
                transaction_id: 1,
                transaction_type: "PURCHASE_RECEIPT".to_string(),
                product_id: 2,
                warehouse_id: 3,
                quantity_meters: Decimal::from(50),
                quantity_kg: Decimal::from(10),
                source_bill_type: Some("PO".to_string()),
                source_bill_no: Some("PO-001".to_string()),
                source_bill_id: Some(11),
                batch_no: "B-1".to_string(),
                color_no: "RED".to_string(),
                created_by: Some(7),
            },
        ];
        for event in &cases {
            let payload = EventPayload::from(event);
            let bytes = serde_json::to_vec(&payload).expect("序列化失败");
            let restored_payload: EventPayload =
                serde_json::from_slice(&bytes).expect("反序列化失败");
            let restored = BusinessEvent::try_from(restored_payload).expect("转换失败");
            let event_type = event_type_name(event);
            let restored_type = event_type_name(&restored);
            assert_eq!(event_type, restored_type, "事件类型不匹配: {}", event_type);
        }
    }
}