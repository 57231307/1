with open("/home/root0/桌面/121/1/backend/src/handlers/purchase_receipt_handler.rs", "r") as f:
    content = f.read()

content = content.replace("use crate::services::event_bus::BusinessEvent;", "use crate::services::event_bus::{BusinessEvent, EVENT_BUS};")
content = content.replace("state.event_bus.publish(BusinessEvent::PurchaseReceiptCompleted", "EVENT_BUS.publish(BusinessEvent::PurchaseReceiptCompleted")

with open("/home/root0/桌面/121/1/backend/src/handlers/purchase_receipt_handler.rs", "w") as f:
    f.write(content)
