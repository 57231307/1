use wasm_bindgen::JsCast;
use crate::components::navigation::Navigation;
use crate::components::command_palette::CommandPalette;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct MainLayoutProps {
    pub current_page: String,
    pub children: Children,
}

#[function_component(MainLayout)]
pub fn main_layout(props: &MainLayoutProps) -> Html {
    let is_mobile_menu_open = use_state(|| false);
    let is_cmd_palette_open = use_state(|| false);

    let toggle_menu = {
        let is_mobile_menu_open = is_mobile_menu_open.clone();
        Callback::from(move |_| {
            is_mobile_menu_open.set(!*is_mobile_menu_open);
        })
    };

    let toggle_cmd_palette = {
        let is_cmd_palette_open = is_cmd_palette_open.clone();
        Callback::from(move |_| {
            is_cmd_palette_open.set(!*is_cmd_palette_open);
        })
    };

    let close_cmd_palette = {
        let is_cmd_palette_open = is_cmd_palette_open.clone();
        Callback::from(move |_| {
            is_cmd_palette_open.set(false);
        })
    };

    // Global keyboard listener for Ctrl+K / Cmd+K
    {
        let is_cmd_palette_open = is_cmd_palette_open.clone();
        use_effect_with((), move |_| {
            let document = web_sys::window().unwrap().document().unwrap();
            let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                if (event.ctrl_key() || event.meta_key()) && event.key() == "k" {
                    event.prevent_default();
                    is_cmd_palette_open.set(true);
                }
                if event.key() == "Escape" {
                    is_cmd_palette_open.set(false);
                }
            }) as Box<dyn FnMut(_)>);
            
            document.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref()).unwrap();
            closure.forget(); // Memory leak for simplicity in this MVP
            || ()
        });
    }

    html! {
        <div class="app-container min-h-screen bg-slate-50 flex flex-col md:flex-row">
            <CommandPalette is_open={*is_cmd_palette_open} on_close={close_cmd_palette} />
            
            <header class="md:hidden bg-white border-b border-slate-200 p-4 flex justify-between items-center sticky top-0 z-40">
                <div class="font-bold text-lg text-slate-800">{"面料二批 ERP"}</div>
                <div class="flex items-center gap-2">
                    <button onclick={toggle_cmd_palette.clone()} class="p-2 text-slate-600 hover:bg-slate-100 rounded-md">
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path></svg>
                    </button>
                    <button onclick={toggle_menu.clone()} class="p-2 text-slate-600 hover:bg-slate-100 rounded-md">
                        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16"></path></svg>
                    </button>
                </div>
            </header>
            
            if *is_mobile_menu_open {
                <div class="md:hidden fixed inset-0 bg-slate-900/50 z-40" onclick={toggle_menu.clone()}></div>
            }
            
            <div class={format!("fixed inset-y-0 left-0 z-50 w-64 bg-white border-r border-slate-200 transform transition-transform duration-200 ease-in-out flex flex-col md:relative md:transform-none md:flex-shrink-0 {}", if *is_mobile_menu_open { "translate-x-0" } else { "-translate-x-full md:translate-x-0" })}>
                <div class="p-4 border-b border-slate-100 hidden md:block">
                    <button onclick={toggle_cmd_palette} class="w-full flex items-center justify-between bg-slate-50 border border-slate-200 rounded-md px-3 py-2 text-sm text-slate-500 hover:border-indigo-300 hover:ring-1 hover:ring-indigo-100 transition-all">
                        <span class="flex items-center"><svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path></svg> {"搜索或跳转..."}</span>
                        <kbd class="hidden md:inline-block font-sans text-xs font-semibold">{"⌘K"}</kbd>
                    </button>
                </div>
                <div class="flex-1 overflow-y-auto">
                    <Navigation current_page={props.current_page.clone()} />
                </div>
            </div>
            
            <main class="flex-1 overflow-x-hidden overflow-y-auto bg-slate-50 p-4 md:p-6 w-full">
                <div class="max-w-7xl mx-auto space-y-6">
                    {props.children.clone()}
                </div>
            </main>
        </div>
    }
}
