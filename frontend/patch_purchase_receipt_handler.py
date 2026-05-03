with open("/home/root0/桌面/121/1/backend/src/handlers/purchase_receipt_handler.rs", "r") as f:
    content = f.read()

# Add event bus import
if "use crate::services::event_bus::BusinessEvent;" not in content:
    content = content.replace("use crate::utils::app_state::AppState;", "use crate::utils::app_state::AppState;\nuse crate::services::event_bus::BusinessEvent;")

publish_code = """
    let receipt = service.create_receipt(req, user_id).await?;
    
    // 发布采购收货完成事件
    if let Some(order_id) = receipt.order_id {
        state.event_bus.publish(BusinessEvent::PurchaseReceiptCompleted { order_id });
    }
"""

content = content.replace("    let receipt = service.create_receipt(req, user_id).await?;", publish_code)

with open("/home/root0/桌面/121/1/backend/src/handlers/purchase_receipt_handler.rs", "w") as f:
    f.write(content)
