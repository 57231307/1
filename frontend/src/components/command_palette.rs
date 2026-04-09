use yew::prelude::*;
use crate::app::Route;
use yew_router::prelude::*;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub is_open: bool,
    pub on_close: Callback<()>,
}

#[function_component(CommandPalette)]
pub fn command_palette(props: &Props) -> Html {
    let search_query = use_state(|| String::new());
    let navigator = use_navigator().unwrap();

    let on_input = {
        let search_query = search_query.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                search_query.set(input.value().to_lowercase());
            }
        })
    };

    let on_bg_click = {
        let on_close = props.on_close.clone();
        Callback::from(move |_| on_close.emit(()))
    };

    let navigate_to = {
        let nav = navigator.clone();
        let on_close = props.on_close.clone();
        Callback::from(move |route: Route| {
            nav.push(&route);
            on_close.emit(());
        })
    };

    // Pre-defined search index for the B2B ERP
    let commands = vec![
        ("首页仪表盘 (Dashboard)", "查看销售与库存概览", Route::Dashboard),
        ("面料产品档案 (Products)", "管理针织/梭织面料、克重、门幅等基础数据", Route::Products),
        ("客户管理 (Customers)", "管理下游客户、账期、等级", Route::Customers),
        ("销售开单 (Sales Order)", "创建面料批发销售单据", Route::Sales),
        ("库存查询 (Inventory)", "按批次、条码、仓库查询面料实时库存", Route::Inventory),
        ("采购入库 (Purchase Receipt)", "供应商面料到货入库、条码生成", Route::PurchaseReceipts),
        ("客户对账单 (Customer Statement)", "导出和打印客户期初、发货、收款明细对账", Route::CustomerStatement),
        ("应收账款 (AR Invoice)", "管理客户欠款与核销", Route::ArInvoices),
        ("系统设置 (Settings)", "角色权限与数据字典配置", Route::Users),
    ];

    let filtered_commands: Vec<_> = commands.into_iter()
        .filter(|(title, desc, _)| {
            search_query.is_empty() || title.to_lowercase().contains(&*search_query) || desc.to_lowercase().contains(&*search_query)
        })
        .collect();

    if !props.is_open {
        return html! {};
    }

    html! {
        <div class="fixed inset-0 z-[100] flex items-start justify-center pt-16 sm:pt-24">
            // Backdrop
            <div class="fixed inset-0 bg-slate-900/40 backdrop-blur-sm transition-opacity" onclick={on_bg_click}></div>
            
            // Command Palette Dialog
            <div class="relative w-full max-w-xl transform overflow-hidden rounded-xl bg-white shadow-2xl ring-1 ring-black ring-opacity-5 transition-all sm:mx-4">
                <div class="flex items-center border-b border-slate-200 px-4 py-3">
                    <svg class="h-5 w-5 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path></svg>
                    <input 
                        type="text" 
                        class="h-10 w-full border-0 bg-transparent pl-4 pr-4 text-slate-900 placeholder:text-slate-400 focus:ring-0 sm:text-sm" 
                        placeholder="输入拼音、功能名称或单号进行全局搜索 (Esc 退出)..." 
                        oninput={on_input}
                        autofocus=true
                    />
                    <kbd class="hidden sm:inline-block rounded border border-slate-200 bg-slate-50 px-2 py-0.5 text-xs font-sans text-slate-400">{"ESC"}</kbd>
                </div>

                <ul class="max-h-80 scroll-py-2 overflow-y-auto p-2 text-sm text-slate-700">
                    {for filtered_commands.into_iter().map(|(title, desc, route)| {
                        let nav = navigate_to.clone();
                        html! {
                            <li 
                                class="cursor-pointer select-none rounded-md px-4 py-3 hover:bg-indigo-50 hover:text-indigo-900"
                                onclick={move |_| nav.emit(route.clone())}
                            >
                                <div class="flex items-center">
                                    <svg class="h-5 w-5 text-slate-400 group-hover:text-indigo-500 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"></path></svg>
                                    <div class="flex flex-col">
                                        <span class="font-medium">{title}</span>
                                        <span class="text-xs text-slate-500 mt-0.5">{desc}</span>
                                    </div>
                                </div>
                            </li>
                        }
                    })}
                    
                    if search_query.len() > 0 {
                        <li class="px-4 py-4 text-center text-sm text-slate-500">
                            {"没有找到其他结果。"}
                        </li>
                    }
                </ul>
            </div>
        </div>
    }
}
