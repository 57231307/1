with open("/home/root0/桌面/121/1/backend/src/services/ap_payment_service.rs", "r") as f:
    content = f.read()

# Add import
if "use crate::services::event_bus::{BusinessEvent, EVENT_BUS};" not in content:
    content = content.replace("use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, TransactionTrait};", "use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, TransactionTrait};\nuse crate::services::event_bus::{BusinessEvent, EVENT_BUS};")

# Inject event publishing
event_logic = """
        let payment = payment_active.update(&txn).await?;

        // 发布付款完成事件
        if let Some(request_id) = payment.request_id {
            let items = ap_payment_request_item::Entity::find()
                .filter(ap_payment_request_item::Column::RequestId.eq(request_id))
                .all(&txn)
                .await?;
            for item in items {
                EVENT_BUS.publish(BusinessEvent::PaymentCompleted { invoice_id: item.invoice_id });
            }
        }
"""
content = content.replace("        let payment = payment_active.update(&txn).await?;", event_logic)

with open("/home/root0/桌面/121/1/backend/src/services/ap_payment_service.rs", "w") as f:
    f.write(content)
