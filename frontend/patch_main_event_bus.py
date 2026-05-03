with open("/home/root0/桌面/121/1/backend/src/main.rs", "r") as f:
    content = f.read()

# Add event listener start
if "crate::services::event_bus::start_event_listener" not in content:
    content = content.replace("let app_state_clone = app_state.clone();", "let app_state_clone = app_state.clone();\n            crate::services::event_bus::start_event_listener(Arc::new(app_state.clone())).await;")

with open("/home/root0/桌面/121/1/backend/src/main.rs", "w") as f:
    f.write(content)
