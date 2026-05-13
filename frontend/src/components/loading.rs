use yew::prelude::*;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

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

            let start_callback = Closure::wrap(Box::new({
                let state = state.clone();
                move |_event: web_sys::Event| {
                    state.dispatch(LoadingAction::Start);
                }
            }) as Box<dyn FnMut(web_sys::Event)>);

            let stop_callback = Closure::wrap(Box::new({
                let state = state.clone();
                move |_event: web_sys::Event| {
                    state.dispatch(LoadingAction::Stop);
                }
            }) as Box<dyn FnMut(web_sys::Event)>);

            window
                .add_event_listener_with_callback("api_start_loading", start_callback.as_ref().unchecked_ref())
                .unwrap();
            window
                .add_event_listener_with_callback("api_stop_loading", stop_callback.as_ref().unchecked_ref())
                .unwrap();

            move || {
                window
                    .remove_event_listener_with_callback("api_start_loading", start_callback.as_ref().unchecked_ref())
                    .unwrap();
                window
                    .remove_event_listener_with_callback("api_stop_loading", stop_callback.as_ref().unchecked_ref())
                    .unwrap();
                start_callback.forget();
                stop_callback.forget();
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
