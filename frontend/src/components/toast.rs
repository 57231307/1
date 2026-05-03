use yew::prelude::*;
use std::rc::Rc;

#[derive(Clone, PartialEq, Debug)]
pub enum ToastType {
    Success,
    Error,
    Info,
    Warning,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ToastMessage {
    pub id: u32,
    pub message: String,
    pub toast_type: ToastType,
}

pub enum ToastAction {
    Add(ToastMessage),
    Remove(u32),
}

impl Reducible for ToastState {
    type Action = ToastAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut toasts = self.toasts.clone();
        match action {
            ToastAction::Add(toast) => {
                toasts.push(toast);
            }
            ToastAction::Remove(id) => {
                toasts.retain(|t| t.id != id);
            }
        }
        Rc::new(ToastState { toasts })
    }
}

#[derive(Clone, PartialEq, Default)]
pub struct ToastState {
    pub toasts: Vec<ToastMessage>,
}

#[function_component(ToastProvider)]
pub fn toast_provider(props: &yew::html::ChildrenProps) -> Html {
    let state = use_reducer(ToastState::default);
    
    html! {
        <ContextProvider<UseReducerHandle<ToastState>> context={state.clone()}>
            { props.children.clone() }
            <div class="fixed top-4 right-4 z-50 flex flex-col gap-2">
                { for state.toasts.iter().map(|toast| {
                    let bg_color = match toast.toast_type {
                        ToastType::Success => "bg-green-500",
                        ToastType::Error => "bg-red-500",
                        ToastType::Info => "bg-blue-500",
                        ToastType::Warning => "bg-yellow-500",
                    };
                    let id = toast.id;
                    let dispatcher = state.dispatcher();
                    let onclick = Callback::from(move |_| {
                        dispatcher.dispatch(ToastAction::Remove(id));
                    });
                    
                    html! {
                        <div class={format!("{} text-white px-4 py-2 rounded shadow-lg flex justify-between items-center min-w-[200px] animate-fade-in-down", bg_color)}>
                            <span>{ &toast.message }</span>
                            <button {onclick} class="ml-4 font-bold focus:outline-none hover:text-gray-200">{"×"}</button>
                        </div>
                    }
                }) }
            </div>
        </ContextProvider<UseReducerHandle<ToastState>>>
    }
}

use std::sync::atomic::{AtomicU32, Ordering};
static TOAST_COUNTER: AtomicU32 = AtomicU32::new(0);

pub fn show_toast(dispatcher: UseReducerHandle<ToastState>, message: impl Into<String>, toast_type: ToastType) {
    let id = TOAST_COUNTER.fetch_add(1, Ordering::SeqCst);
    dispatcher.dispatch(ToastAction::Add(ToastMessage {
        id,
        message: message.into(),
        toast_type,
    }));
    
    // Yew doesn't easily support setTimeout in generic functions without WASM binds,
    // so in a real app we'd use gloo_timers here to auto-remove after 3s.
    // For simplicity, we just let them click 'x' or we implement a hook.
    let dispatcher_clone = dispatcher.clone();
    gloo_timers::callback::Timeout::new(3000, move || {
        dispatcher_clone.dispatch(ToastAction::Remove(id));
    }).forget();
}
