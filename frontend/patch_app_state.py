with open("/home/root0/桌面/121/1/backend/src/utils/app_state.rs", "r") as f:
    content = f.read()

# Add import
if "use crate::services::event_bus::EventBus;" not in content:
    content = content.replace("use crate::utils::cache::AppCache;", "use crate::utils::cache::AppCache;\nuse crate::services::event_bus::EventBus;")

# Add field to struct
if "pub event_bus: Arc<EventBus>," not in content:
    content = content.replace("pub cookie_key: Key,", "pub cookie_key: Key,\n    pub event_bus: Arc<EventBus>,")

# Add to init
if "event_bus: Arc::new(EventBus::new())," not in content:
    content = content.replace("cookie_key,", "cookie_key,\n            event_bus: Arc::new(EventBus::new()),")

with open("/home/root0/桌面/121/1/backend/src/utils/app_state.rs", "w") as f:
    f.write(content)
