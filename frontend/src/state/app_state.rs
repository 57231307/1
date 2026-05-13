use yew::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::time::Duration;
use crate::models::auth::UserInfo;
use crate::utils::permissions::UserPermission;

#[derive(Clone, PartialEq)]
pub struct ApiCache;

impl ApiCache {
    pub fn new(_capacity: usize, _ttl: Duration) -> Self {
        Self
    }
}

#[derive(Clone, PartialEq)]
pub struct AppState {
    pub user: Option<UserInfo>,
    pub permissions: Vec<UserPermission>,
    pub is_loading: bool,
    pub cache: Rc<RefCell<ApiCache>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            user: None,
            permissions: Vec::new(),
            is_loading: false,
            cache: Rc::new(RefCell::new(ApiCache::new(100, Duration::from_secs(300)))),
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct AppStateProviderProps {
    pub children: Children,
}

#[function_component(AppStateProvider)]
pub fn app_state_provider(props: &AppStateProviderProps) -> Html {
    let state = use_state(AppState::default);
    html! {
        <ContextProvider<UseStateHandle<AppState>> context={state}>
            { props.children.clone() }
        </ContextProvider<UseStateHandle<AppState>>>
    }
}

#[hook]
pub fn use_app_state() -> Option<UseStateHandle<AppState>> {
    use_context::<UseStateHandle<AppState>>()
}
