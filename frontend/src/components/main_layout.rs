use crate::components::navigation::Navigation;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct MainLayoutProps {
    pub current_page: String,
    pub children: Children,
}

#[function_component(MainLayout)]
pub fn main_layout(props: &MainLayoutProps) -> Html {
    let is_mobile_menu_open = use_state(|| false);

    let toggle_menu = {
        let is_mobile_menu_open = is_mobile_menu_open.clone();
        Callback::from(move |_| {
            is_mobile_menu_open.set(!*is_mobile_menu_open);
        })
    };

    html! {
        <div class="app-container min-h-screen bg-slate-50 flex flex-col md:flex-row">
            <header class="md:hidden bg-white border-b border-slate-200 p-4 flex justify-between items-center sticky top-0 z-40">
                <div class="font-bold text-lg text-slate-800">{"面料二批 ERP"}</div>
                <button onclick={toggle_menu.clone()} class="p-2 text-slate-600 hover:bg-slate-100 rounded-md">
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16"></path></svg>
                </button>
            </header>
            
            if *is_mobile_menu_open {
                <div class="md:hidden fixed inset-0 bg-slate-900/50 z-40" onclick={toggle_menu.clone()}></div>
            }
            
            <div class={format!("fixed inset-y-0 left-0 z-50 w-64 bg-white border-r border-slate-200 transform transition-transform duration-200 ease-in-out md:relative md:transform-none md:flex-shrink-0 {}", if *is_mobile_menu_open { "translate-x-0" } else { "-translate-x-full md:translate-x-0" })}>
                <div class="h-full overflow-y-auto">
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
