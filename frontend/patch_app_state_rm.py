with open("/home/root0/桌面/121/1/backend/src/utils/app_state.rs", "r") as f:
    content = f.read()

content = content.replace("use crate::services::event_bus::EventBus;\n", "")
content = content.replace("pub event_bus: Arc<EventBus>,\n", "")
content = content.replace("event_bus: Arc::new(EventBus::new()),\n", "")

with open("/home/root0/桌面/121/1/backend/src/utils/app_state.rs", "w") as f:
    f.write(content)

with open("/home/root0/桌面/121/1/backend/src/main.rs", "r") as f:
    content = f.read()

content = content.replace("crate::services::event_bus::start_event_listener(Arc::new(app_state.clone())).await;", "crate::services::event_bus::start_event_listener(app_state.db.clone()).await;")

with open("/home/root0/桌面/121/1/backend/src/main.rs", "w") as f:
    f.write(content)

with open("/home/root0/桌面/121/1/backend/src/services/event_bus.rs", "r") as f:
    content = f.read()

content = content.replace("pub async fn start_event_listener(state: Arc<AppState>) {", "pub async fn start_event_listener(db: Arc<DatabaseConnection>) {")
content = content.replace("    let mut receiver = state.event_bus.subscribe();", "    let mut receiver = EVENT_BUS.subscribe();")
content = content.replace("    let db = state.db.clone();", "")

with open("/home/root0/桌面/121/1/backend/src/services/event_bus.rs", "w") as f:
    f.write(content)
