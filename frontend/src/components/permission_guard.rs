use yew::prelude::*;
use crate::utils::permissions::has_permission;

#[derive(Properties, PartialEq)]
pub struct PermissionGuardProps {
    pub resource: String,
    pub action: String,
    #[prop_or_default]
    pub children: Children,
}

#[function_component(PermissionGuard)]
pub fn permission_guard(props: &PermissionGuardProps) -> Html {
    if has_permission(&props.resource, &props.action) {
        html! { <>{ for props.children.iter() }</> }
    } else {
        html! {}
    }
}
