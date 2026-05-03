use yew::prelude::*;
use std::rc::Rc;
use gloo_events::EventListener;

#[derive(Clone, PartialEq, Default)]
pub struct LoadingState {
    pub request_count: u32,
}

pub enum LoadingAction {
    Start,
    Stop,
}

impl Reducible for LoadingState {
    type Action = LoadingAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let count = match action {
            LoadingAction::Start => self.request_count + 1,
            LoadingAction::Stop => self.request_count.saturating_sub(1),
        };
        Rc::new(LoadingState { request_count: count })
    }
}

#[function_component(LoadingProvider)]
pub fn loading_provider(props: &yew::html::ChildrenProps) -> Html {
    let state = use_reducer(LoadingState::default);
    
    {
        let state = state.clone();
        use_effect_with((), move |_| {
            let window = web_sys::window().unwrap();
            
            let start_listener = EventListener::new(&window, "api_start_loading", {
                let state = state.clone();
                move |_| {
                    state.dispatch(LoadingAction::Start);
                }
            });
            
            let stop_listener = EventListener::new(&window, "api_stop_loading", {
                let state = state.clone();
                move |_| {
                    state.dispatch(LoadingAction::Stop);
                }
            });
            
            || {
                drop(start_listener);
                drop(stop_listener);
            }
        });
    }
    
    let is_loading = state.request_count > 0;
    
    html! {
        <ContextProvider<UseReducerHandle<LoadingState>> context={state.clone()}>
            { props.children.clone() }
            
            if is_loading {
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-30">
                    <div class="flex flex-col items-center p-6 bg-white rounded-lg shadow-xl">
                        <div class="w-12 h-12 border-4 border-indigo-500 border-t-transparent rounded-full animate-spin"></div>
                        <p class="mt-4 text-gray-700 font-medium">{"正在处理中..."}</p>
                    </div>
                </div>
            }
        </ContextProvider<UseReducerHandle<LoadingState>>>
    }
}
