use crate::app::Route;
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
                MenuItem { name: "色卡管理", route: Route::ColorCard },
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
                                                    classes={classes!(format!("block pl-[44px] pr-3 py-2 text-[14px] rounded transition-colors {}", if is_active { "text-[#165DFF] bg-[#E8F3FF] font-medium" } else { "text-[#4E5969] hover:bg-[#F5F7FA] hover:text-[#165DFF]" }))}
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
            <Link<Route> to={Route::Dashboard} classes={classes!(format!("flex flex-col items-center justify-center w-full h-full text-[10px] {}", if props.current_page == "仪表板" { "text-[#165DFF]" } else { "text-[#4E5969]" }))}>
                {get_icon("首页")}
                <span>{"首页"}</span>
            </Link<Route>>
            <Link<Route> to={Route::Inventory} classes={classes!(format!("flex flex-col items-center justify-center w-full h-full text-[10px] {}", if props.current_page == "库存查询" { "text-[#165DFF]" } else { "text-[#4E5969]" }))}>
                {get_icon("库存")}
                <span>{"库存"}</span>
            </Link<Route>>
            <Link<Route> to={Route::Sales} classes={classes!(format!("flex flex-col items-center justify-center w-full h-full text-[10px] {}", if props.current_page == "销售订单" { "text-[#165DFF]" } else { "text-[#4E5969]" }))}>
                {get_icon("开单")}
                <span>{"开单"}</span>
            </Link<Route>>
            <Link<Route> to={Route::CustomerStatement} classes={classes!(format!("flex flex-col items-center justify-center w-full h-full text-[10px] {}", if props.current_page == "客户对账单" { "text-[#165DFF]" } else { "text-[#4E5969]" }))}>
                {get_icon("报表")}
                <span>{"报表"}</span>
            </Link<Route>>
            <Link<Route> to={Route::Users} classes={classes!(format!("flex flex-col items-center justify-center w-full h-full text-[10px] {}", if props.current_page == "用户管理" { "text-[#165DFF]" } else { "text-[#4E5969]" }))}>
                {get_icon("我的")}
                <span>{"我的"}</span>
            </Link<Route>>
        </nav>
    }
}
