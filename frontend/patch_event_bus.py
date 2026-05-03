with open("/home/root0/桌面/121/1/backend/src/services/event_bus.rs", "r") as f:
    content = f.read()

listener_code = """
use crate::utils::app_state::AppState;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set};

pub async fn start_event_listener(state: Arc<AppState>) {
    let mut receiver = state.event_bus.subscribe();
    let db = state.db.clone();
    
    tokio::spawn(async move {
        while let Ok(event) = receiver.recv().await {
            match event {
                BusinessEvent::PurchaseReceiptCompleted { order_id } => {
                    tracing::info!("Event received: PurchaseReceiptCompleted for order {}", order_id);
                    // We can do real DB updates here, e.g. update_purchase_order_status
                }
                BusinessEvent::SalesOrderShipped { order_id } => {
                    tracing::info!("Event received: SalesOrderShipped for order {}", order_id);
                }
                BusinessEvent::PaymentCompleted { invoice_id } => {
                    tracing::info!("Event received: PaymentCompleted for invoice {}", invoice_id);
                }
            }
        }
    });
}
"""
if "pub async fn start_event_listener" not in content:
    content += listener_code

with open("/home/root0/桌面/121/1/backend/src/services/event_bus.rs", "w") as f:
    f.write(content)
