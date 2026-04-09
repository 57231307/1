import os
import re

main_layout_content = """use wasm_bindgen::JsCast;
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

            {/* Mobile Top Navbar (50px) */}
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

            {/* PC Sidebar */}
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

            {/* Main Content Area */}
            <div class="flex-1 flex flex-col w-full h-screen overflow-hidden">
                {/* PC Top Navbar (56px) */}
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
            
            {/* Mobile Bottom Navbar (56px) */}
            <MobileBottomNav current_page={props.current_page.clone()} />
        </div>
    }
}
"""

with open('frontend/src/components/main_layout.rs', 'w', encoding='utf-8') as f:
    f.write(main_layout_content)

navigation_content = """use crate::app::Route;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Properties, PartialEq)]
pub struct NavigationProps {
    pub current_page: String,
    pub collapsed: bool,
}

struct MenuGroup {
    title: &'static str,
    icon: Html,
    items: Vec<MenuItem>,
}

struct MenuItem {
    name: &'static str,
    route: Route,
}

#[function_component(Navigation)]
pub fn navigation(props: &NavigationProps) -> Html {
    let open_group = use_state(|| String::from(""));

    let toggle_group = {
        let open_group = open_group.clone();
        Callback::from(move |group_name: String| {
            if *open_group == group_name {
                open_group.set(String::from("")); 
            } else {
                open_group.set(group_name); 
            }
        })
    };

    let get_icon = |name: &str| -> Html {
        match name {
            "首页" => html!{<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6"></path></svg>},
            "基础" => html!{<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z"></path></svg>},
            "采购" => html!{<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 3h2l.4 2M7 13h10l4-8H5.4M7 13L5.4 5M7 13l-2.293 2.293c-.63.63-.184 1.707.707 1.707H17m0 0a2 2 0 100 4 2 2 0 000-4zm-8 2a2 2 0 11-4 0 2 2 0 014 0z"></path></svg>},
            "销售" => html!{<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>},
            "库存" => html!{<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"></path></svg>},
            "财务" => html!{<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 14l6-6m-5.5.5h.01m4.99 5h.01M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16l3.5-2 3.5 2 3.5-2 3.5 2zM10 8.5a.5.5 0 11-1 0 .5.5 0 011 0zm5 5a.5.5 0 11-1 0 .5.5 0 011 0z"></path></svg>},
            "打印" => html!{<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 17h2a2 2 0 002-2v-4a2 2 0 00-2-2H5a2 2 0 00-2 2v4a2 2 0 002 2h2m2 4h6a2 2 0 002-2v-4a2 2 0 00-2-2H9a2 2 0 00-2 2v4a2 2 0 002 2zm8-12V5a2 2 0 00-2-2H9a2 2 0 00-2 2v4h10z"></path></svg>},
            "系统" => html!{<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path></svg>},
            _ => html!{<span>{"•"}</span>}
        }
    };

    let menu_groups = vec![
        MenuGroup {
            title: "首页", icon: get_icon("首页"),
            items: vec![MenuItem { name: "仪表板", route: Route::Dashboard }],
        },
        MenuGroup {
            title: "基础信息", icon: get_icon("基础"),
            items: vec![
                MenuItem { name: "面料档案", route: Route::Products },
                MenuItem { name: "客户管理", route: Route::Customers },
                MenuItem { name: "供应商", route: Route::Suppliers },
                MenuItem { name: "仓库管理", route: Route::Warehouses },
            ],
        },
        MenuGroup {
            title: "采购管理", icon: get_icon("采购"),
            items: vec![
                MenuItem { name: "采购订单", route: Route::PurchaseOrders },
                MenuItem { name: "采购入库", route: Route::PurchaseReceipts },
                MenuItem { name: "采购退货", route: Route::PurchaseReturns },
            ],
        },
        MenuGroup {
            title: "销售管理", icon: get_icon("销售"),
            items: vec![
                MenuItem { name: "销售开单", route: Route::Sales },
                MenuItem { name: "销售发货", route: Route::SalesDeliveries },
                MenuItem { name: "销售退货", route: Route::SalesReturns },
                MenuItem { name: "客户对账", route: Route::CustomerStatement },
            ],
        },
        MenuGroup {
            title: "库存管理", icon: get_icon("库存"),
            items: vec![
                MenuItem { name: "库存查询", route: Route::Inventory },
                MenuItem { name: "库存调拨", route: Route::Transfers },
                MenuItem { name: "库存盘点", route: Route::Counts },
            ],
        },
        MenuGroup {
            title: "财务管理", icon: get_icon("财务"),
            items: vec![
                MenuItem { name: "应收账款", route: Route::ArInvoices },
                MenuItem { name: "应付账款", route: Route::ApInvoices },
                MenuItem { name: "收款单", route: Route::ArVerifications },
                MenuItem { name: "付款单", route: Route::ApVerifications },
            ],
        },
        MenuGroup {
            title: "系统设置", icon: get_icon("系统"),
            items: vec![
                MenuItem { name: "用户管理", route: Route::Users },
                MenuItem { name: "角色权限", route: Route::Roles },
                MenuItem { name: "系统日志", route: Route::OperationLogs },
            ],
        },
    ];

    html! {
        <nav class="w-full py-2">
            {
                for menu_groups.into_iter().map(|group| {
                    let group_title = group.title.to_string();
                    let is_open = *open_group == group_title || props.collapsed;
                    let has_active_child = group.items.iter().any(|item| item.name == props.current_page);
                    
                    // If collapsed, clicking does nothing to accordion, otherwise it toggles
                    let on_header_click = {
                        let toggle_group = toggle_group.clone();
                        let group_title_clone = group_title.clone();
                        let collapsed = props.collapsed;
                        Callback::from(move |_| {
                            if !collapsed {
                                toggle_group.emit(group_title_clone.clone())
                            }
                        })
                    };

                    html! {
                        <div class="mb-1 px-3">
                            <div 
                                class={format!("flex items-center justify-between px-3 py-2 rounded cursor-pointer transition-colors {}", if has_active_child { "text-[#165DFF] bg-[#E8F3FF]" } else { "text-[#4E5969] hover:bg-[#F5F7FA]" })}
                                onclick={on_header_click}
                                title={group.title}
                            >
                                <div class="flex items-center gap-3">
                                    <span class={if has_active_child { "text-[#165DFF]" } else { "text-[#86909C]" }}>
                                        {group.icon.clone()}
                                    </span>
                                    if !props.collapsed {
                                        <span class="font-medium text-[14px]">{group.title}</span>
                                    }
                                </div>
                                if !props.collapsed {
                                    <span class="text-[#86909C]">
                                        <svg class={format!("w-4 h-4 transition-transform {}", if is_open { "rotate-180" } else { "" })} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path></svg>
                                    </span>
                                }
                            </div>
                            
                            if is_open && !props.collapsed {
                                <div class="mt-1 flex flex-col gap-1 overflow-hidden">
                                    {
                                        for group.items.into_iter().map(|item| {
                                            let is_active = props.current_page == item.name;
                                            html! {
                                                <Link<Route> 
                                                    to={item.route} 
                                                    classes={format!("block pl-[44px] pr-3 py-2 text-[14px] rounded transition-colors {}", if is_active { "text-[#165DFF] bg-[#E8F3FF] font-medium" } else { "text-[#4E5969] hover:bg-[#F5F7FA] hover:text-[#165DFF]" })}
                                                >
                                                    {item.name}
                                                </Link<Route>>
                                            }
                                        })
                                    }
                                </div>
                            }
                        </div>
                    }
                })
            }
        </nav>
    }
}

#[derive(Properties, PartialEq)]
pub struct MobileNavProps {
    pub current_page: String,
}

#[function_component(MobileBottomNav)]
pub fn mobile_bottom_nav(props: &MobileNavProps) -> Html {
    let get_icon = |name: &str| -> Html {
        match name {
            "首页" => html!{<svg class="w-6 h-6 mb-1" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6"></path></svg>},
            "库存" => html!{<svg class="w-6 h-6 mb-1" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"></path></svg>},
            "开单" => html!{<svg class="w-6 h-6 mb-1" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>},
            "报表" => html!{<svg class="w-6 h-6 mb-1" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 17v-2m3 2v-4m3 4v-6m2 10H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path></svg>},
            "我的" => html!{<svg class="w-6 h-6 mb-1" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"></path></svg>},
            _ => html!{}
        }
    };

    html! {
        <nav class="md:hidden fixed bottom-0 left-0 w-full h-[56px] bg-white border-t border-[#E5E6EB] flex justify-around items-center z-50 pb-safe">
            <Link<Route> to={Route::Dashboard} classes={format!("flex flex-col items-center justify-center w-full h-full text-[10px] {}", if props.current_page == "仪表板" { "text-[#165DFF]" } else { "text-[#4E5969]" })}>
                {get_icon("首页")}
                <span>{"首页"}</span>
            </Link<Route>>
            <Link<Route> to={Route::Inventory} classes={format!("flex flex-col items-center justify-center w-full h-full text-[10px] {}", if props.current_page == "库存查询" { "text-[#165DFF]" } else { "text-[#4E5969]" })}>
                {get_icon("库存")}
                <span>{"库存"}</span>
            </Link<Route>>
            <Link<Route> to={Route::Sales} classes={format!("flex flex-col items-center justify-center w-full h-full text-[10px] {}", if props.current_page == "销售订单" { "text-[#165DFF]" } else { "text-[#4E5969]" })}>
                {get_icon("开单")}
                <span>{"开单"}</span>
            </Link<Route>>
            <Link<Route> to={Route::CustomerStatement} classes={format!("flex flex-col items-center justify-center w-full h-full text-[10px] {}", if props.current_page == "客户对账单" { "text-[#165DFF]" } else { "text-[#4E5969]" })}>
                {get_icon("报表")}
                <span>{"报表"}</span>
            </Link<Route>>
            <Link<Route> to={Route::Users} classes={format!("flex flex-col items-center justify-center w-full h-full text-[10px] {}", if props.current_page == "用户管理" { "text-[#165DFF]" } else { "text-[#4E5969]" })}>
                {get_icon("我的")}
                <span>{"我的"}</span>
            </Link<Route>>
        </nav>
    }
}
"""

with open('frontend/src/components/navigation.rs', 'w', encoding='utf-8') as f:
    f.write(navigation_content)

print("Layout updated.")
