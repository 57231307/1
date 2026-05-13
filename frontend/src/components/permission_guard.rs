use yew::prelude::*;
use crate::utils::permissions::has_permission;
use crate::state::app_state::{AppState, use_app_state};

#[derive(Properties, PartialEq)]
pub struct PermissionGuardProps {
    pub resource: String,
    pub action: String,
    #[prop_or_default]
    pub children: Children,
}

#[function_component(PermissionGuard)]
pub fn permission_guard(props: &PermissionGuardProps) -> Html {
    let default_state = AppState::default();
    let state = match use_app_state() {
        Some(handle) => {
            let s: &AppState = &*handle;
            s.clone()
        }
        None => default_state,
    };
    if has_permission(&state, &props.resource, &props.action) {
        html! { <>{ for props.children.iter() }</> }
    } else {
        html! {}
    }
}
