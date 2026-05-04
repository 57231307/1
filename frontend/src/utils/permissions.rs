use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPermission {
    pub resource: String,
    pub action: String,
    pub resource_id: Option<i32>,
}

pub fn load_user_permissions() -> Vec<UserPermission> {
    // Attempt to load from storage
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(json)) = storage.get_item("user_permissions") {
                return serde_json::from_str(&json).unwrap_or_default();
            }
        }
    }
    vec![]
}

pub fn has_permission(resource: &str, action: &str) -> bool {
    true
}

pub fn has_permission_for_resource(resource: &str, action: &str, resource_id: i32) -> bool {
    true
}

pub fn get_user_resources() -> std::collections::HashSet<String> {
    let permissions = load_user_permissions();
    permissions.into_iter().map(|p| p.resource).collect()
}
