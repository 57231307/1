use wasm_bindgen::JsCast;
use crate::components::navigation::{Navigation, MobileBottomNav};
use crate::components::command_palette::CommandPalette;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct MainLayoutProps {
    pub current_page: String,
    pub children: Children,
}

#[function_component(MainLayout)]
pub fn main_layout(props: &MainLayoutProps) -> Html {
    let is_sidebar_collapsed = use_state(|| false);
    let is_cmd_palette_open = use_state(|| false);

    let toggle_sidebar = {
        let is_sidebar_collapsed = is_sidebar_collapsed.clone();
        Callback::from(move |_| {
            is_sidebar_collapsed.set(!*is_sidebar_collapsed);
        })
    };

    let toggle_cmd_palette = {
        let is_cmd_palette_open = is_cmd_palette_open.clone();
        Callback::from(move |_| {
            is_cmd_palette_open.set(true);
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
            closure.forget(); 
            || ()
        });
    }

    let sidebar_width = if *is_sidebar_collapsed { "w-[80px]" } else { "w-[220px]" };

    html! {
        <div class="app-container min-h-screen bg-[#F5F7FA] flex flex-col md:flex-row font-sans text-[#1D2129]">
            <CommandPalette is_open={*is_cmd_palette_open} on_close={close_cmd_palette} />

            <header class="md:hidden bg-white border-b border-[#E5E6EB] h-[50px] flex justify-between items-center px-3 sticky top-0 z-40">
                <button class="text-[#4E5969] p-1">
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"></path></svg>
                </button>
                <div class="font-bold text-base text-[#1D2129]">{&props.current_page}</div>
                <div class="flex gap-2">
                    <button onclick={toggle_cmd_palette.clone()} class="text-[#4E5969] p-1">
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path></svg>
                    </button>
                    <button class="text-[#4E5969] p-1 relative">
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"></path></svg>
                        <span class="absolute top-1 right-1 w-2 h-2 bg-[#F53F3F] rounded-full"></span>
                    </button>
                </div>
            </header>

            <div class={format!("hidden md:flex flex-col bg-white border-r border-[#E5E6EB] transition-all duration-200 z-50 {}", sidebar_width)}>
                <div class="h-[80px] flex items-center justify-center border-b border-[#E5E6EB] cursor-pointer" onclick={toggle_sidebar}>
                    if *is_sidebar_collapsed {
                        <div class="w-10 h-10 bg-[#165DFF] text-white rounded flex items-center justify-center font-bold">{"ERP"}</div>
                    } else {
                        <div class="flex items-center gap-2">
                            <div class="w-8 h-8 bg-[#165DFF] text-white rounded flex items-center justify-center font-bold">{"ERP"}</div>
                            <span class="font-bold text-lg text-[#1D2129]">{"面料二批"}</span>
                        </div>
                    }
                </div>
                <div class="flex-1 overflow-y-auto py-4 custom-scrollbar">
                    <Navigation current_page={props.current_page.clone()} collapsed={*is_sidebar_collapsed} />
                </div>
                <div class="p-4 border-t border-[#E5E6EB] text-sm text-[#4E5969]">
                    if !*is_sidebar_collapsed {
                        <div class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                <div class="w-6 h-6 bg-gray-200 rounded-full"></div>
                                <span>{"管理员"}</span>
                            </div>
                            <button class="hover:text-[#F53F3F]">{"退出"}</button>
                        </div>
                    } else {
                        <div class="w-6 h-6 bg-gray-200 rounded-full mx-auto" title="管理员"></div>
                    }
                </div>
            </div>

            <div class="flex-1 flex flex-col w-full h-screen overflow-hidden">
                <header class="hidden md:flex bg-white h-[56px] border-b border-[#E5E6EB] items-center justify-between px-6 shrink-0 z-40">
                    <div class="flex items-center text-sm text-[#4E5969]">
                        <span>{"首页"}</span>
                        <span class="mx-2">{"/"}</span>
                        <span class="text-[#1D2129] font-medium">{&props.current_page}</span>
                    </div>
                    <div class="flex-1 max-w-[300px] mx-8">
                        <div class="relative" onclick={toggle_cmd_palette.clone()}>
                            <input type="text" readonly=true placeholder="搜索面料/客户/供应商/单据号" class="w-full h-8 pl-8 pr-12 bg-[#F5F7FA] border-none rounded cursor-pointer text-sm" />
                            <svg class="w-4 h-4 text-[#86909C] absolute left-2.5 top-2" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path></svg>
                            <kbd class="absolute right-2 top-1.5 text-xs text-[#86909C] border border-[#E5E6EB] rounded px-1">{"⌘K"}</kbd>
                        </div>
                    </div>
                    <div class="flex items-center gap-4">
                        <button class="relative text-[#4E5969] hover:text-[#165DFF]">
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"></path></svg>
                            <span class="absolute -top-1 -right-1 w-2 h-2 bg-[#F53F3F] rounded-full"></span>
                        </button>
                        <div class="flex items-center gap-2 cursor-pointer">
                            <div class="w-8 h-8 bg-indigo-100 text-indigo-600 rounded-full flex items-center justify-center font-bold">{"A"}</div>
                            <span class="text-sm text-[#1D2129]">{"Admin"}</span>
                        </div>
                    </div>
                </header>

                <main class="flex-1 overflow-x-hidden overflow-y-auto p-4 md:p-[20px] pb-[80px] md:pb-[20px]">
                    <div class="w-full mx-auto">
                        {props.children.clone()}
                    </div>
                </main>
            </div>
            
            <MobileBottomNav current_page={props.current_page.clone()} />
        </div>
    }
}
