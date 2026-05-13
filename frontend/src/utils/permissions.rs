use serde::{Deserialize, Serialize};
use crate::state::app_state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserPermission {
    pub resource: String,
    pub action: String,
    pub resource_id: Option<i32>,
}

pub fn load_user_permissions() -> Vec<UserPermission> {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(json)) = storage.get_item("user_permissions") {
                return serde_json::from_str(&json).unwrap_or_default();
            }
        }
    }
    vec![]
}

pub fn has_permission(state: &AppState, resource: &str, action: &str) -> bool {
    state.permissions.iter().any(|p| {
        p.resource == resource && (p.action == action || p.action == "*")
    })
}

pub fn has_permission_for_resource(state: &AppState, resource: &str, action: &str, resource_id: i32) -> bool {
    state.permissions.iter().any(|p| {
        p.resource == resource && (p.action == action || p.action == "*") && p.resource_id == Some(resource_id)
    })
}

pub fn has_any_permission(state: &AppState, resource: &str) -> bool {
    state.permissions.iter().any(|p| p.resource == resource)
}

pub fn get_user_resources(state: &AppState) -> std::collections::HashSet<String> {
    state.permissions.iter().map(|p| p.resource.clone()).collect()
}
