with open("/home/root0/桌面/121/1/backend/src/services/event_bus.rs", "r") as f:
    content = f.read()

# Implement actual DB updates
update_logic = """
                    tracing::info!("Event received: PurchaseReceiptCompleted for order {}", order_id);
                    if let Ok(Some(order)) = crate::models::purchase_order::Entity::find_by_id(order_id).one(db.as_ref()).await {
                        let mut active_order: crate::models::purchase_order::ActiveModel = order.into();
                        active_order.status = Set("RECEIVED".to_string());
                        if let Err(e) = active_order.update(db.as_ref()).await {
                            tracing::error!("Failed to update purchase order {}: {}", order_id, e);
                        } else {
                            tracing::info!("Successfully updated purchase order {} status to RECEIVED", order_id);
                        }
                    }
"""
content = content.replace('                    tracing::info!("Event received: PurchaseReceiptCompleted for order {}", order_id);\n                    // We can do real DB updates here, e.g. update_purchase_order_status', update_logic)

with open("/home/root0/桌面/121/1/backend/src/services/event_bus.rs", "w") as f:
    f.write(content)
