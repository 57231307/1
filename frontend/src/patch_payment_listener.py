with open("src/services/event_bus.rs", "r") as f:
    content = f.read()

update_logic = """
                BusinessEvent::PaymentCompleted { invoice_id } => {
                    tracing::info!("Event received: PaymentCompleted for invoice {}", invoice_id);
                    if let Ok(Some(invoice)) = crate::models::ap_invoice::Entity::find_by_id(invoice_id).one(db.as_ref()).await {
                        let mut active_invoice: crate::models::ap_invoice::ActiveModel = invoice.into();
                        // You can calculate if fully paid and set status
                        active_invoice.invoice_status = Set("PAID".to_string());
                        if let Err(e) = active_invoice.update(db.as_ref()).await {
                            tracing::error!("Failed to update ap_invoice {}: {}", invoice_id, e);
                        } else {
                            tracing::info!("Successfully updated ap_invoice {} status to PAID", invoice_id);
                        }
                    }
                }
"""
content = content.replace("""                BusinessEvent::PaymentCompleted { invoice_id } => {
                    tracing::info!("Event received: PaymentCompleted for invoice {}", invoice_id);
                }""", update_logic)

with open("src/services/event_bus.rs", "w") as f:
    f.write(content)
