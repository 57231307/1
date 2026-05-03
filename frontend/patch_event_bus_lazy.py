with open("/home/root0/桌面/121/1/backend/src/services/event_bus.rs", "r") as f:
    content = f.read()

lazy_bus = """
use once_cell::sync::Lazy;

pub static EVENT_BUS: Lazy<EventBus> = Lazy::new(|| EventBus::new());
"""

if "pub static EVENT_BUS" not in content:
    content = content.replace("use std::sync::Arc;", "use std::sync::Arc;\nuse once_cell::sync::Lazy;")
    content = content.replace("pub struct EventBus {", "pub static EVENT_BUS: Lazy<EventBus> = Lazy::new(|| EventBus::new());\n\npub struct EventBus {")

with open("/home/root0/桌面/121/1/backend/src/services/event_bus.rs", "w") as f:
    f.write(content)
