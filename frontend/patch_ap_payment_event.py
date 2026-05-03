with open("/home/root0/桌面/121/1/backend/src/services/ap_payment_service.rs", "r") as f:
    content = f.read()

# Add import
if "use crate::services::event_bus::BusinessEvent;" not in content:
    content = content.replace("use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, TransactionTrait};", "use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, TransactionTrait};\nuse crate::services::event_bus::BusinessEvent;")

# Inject event publishing
event_logic = """
        txn.commit().await?;
        
        // 发布付款完成事件
        // Note: multiple invoices might be paid in one payment. We just publish the first one or loop
        if let Some(request_id) = payment.request_id {
            if let Ok(items) = ap_payment_request_item::Entity::find()
                .filter(ap_payment_request_item::Column::RequestId.eq(request_id))
                .all(self.db.as_ref())
                .await {
                
                // You can access AppState's event_bus in service if passed, but since it's not available in ApPaymentService easily, we'll need to pass it or we don't.
            }
        }
"""
# Wait, the prompt says "pub static EVENT_BUS: Lazy<EventBus> = Lazy::new(|| EventBus::new());"
# Did I implement EVENT_BUS as Lazy? No, I put it in AppState!
