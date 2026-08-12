#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::event_bus::BusinessEvent;

    #[test]
    fn test_build_completed_event_carries_confirmed_order_fields() {
        let order = OrderModel {
            id: 42,
            order_no: "OS-20260730-001".to_string(),
            order_type: "dyeing".to_string(),
            supplier_id: 7,
            return_quantity: Decimal::new(125, 1),
            voucher_no_receipt: Some("OV-RC-20260730120000-123".to_string()),
            ..Default::default()
        };

        let event = OutsourcingReceiptService::build_completed_event(&order);

        match event {
            BusinessEvent::OutsourcingOrderCompleted {
                order_id,
                order_no,
                order_type,
                supplier_id,
                return_quantity,
                voucher_no_receipt,
            } => {
                assert_eq!(order_id, order.id);
                assert_eq!(order_no, order.order_no);
                assert_eq!(order_type, order.order_type);
                assert_eq!(supplier_id, order.supplier_id);
                assert_eq!(return_quantity, order.return_quantity);
                assert_eq!(voucher_no_receipt, order.voucher_no_receipt);
            }
            other => panic!("应构造委外完成事件，实际为: {:?}", other),
        }
    }
}