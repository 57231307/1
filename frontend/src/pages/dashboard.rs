use crate::components::main_layout::MainLayout;
use yew::prelude::*;
use web_sys::window;
use wasm_bindgen::JsCast;

#[function_component(DashboardPage)]
pub fn dashboard_page() -> Html {
    let is_mobile = use_state(|| {
        window().unwrap().inner_width().unwrap().as_f64().unwrap() < 768.0
    });
    let active_chart_tab = use_state(|| "trend".to_string());

    {
        let is_mobile = is_mobile.clone();
        use_effect_with((), move |_| {
            let win = window().unwrap();
            let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
                is_mobile.set(window().unwrap().inner_width().unwrap().as_f64().unwrap() < 768.0);
            }) as Box<dyn FnMut()>);
            win.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref()).unwrap();
            closure.forget();
            || ()
        });
    }

    let on_chart_tab_change = {
        let active_chart_tab = active_chart_tab.clone();
        Callback::from(move |tab: &str| {
            active_chart_tab.set(tab.to_string());
        })
    };

    html! {
        <MainLayout current_page="仪表板">
            <div class="space-y-4 md:space-y-6">
                
                
                <div class="grid grid-cols-2 md:grid-cols-4 gap-3 md:gap-4">
                    
                    <div class="card bg-white p-4 md:p-5 flex flex-col justify-between">
                        <div class="flex justify-between items-center mb-2">
                            <span class="text-sm text-[#4E5969] font-medium">{"今日销售额"}</span>
                            <div class="w-8 h-8 rounded-full bg-[#E8F3FF] text-[#165DFF] flex items-center justify-center">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
                            </div>
                        </div>
                        <div class="text-[24px] font-bold text-[#1D2129]">{"¥124,500.00"}</div>
                        <div class="text-xs text-[#00B42A] mt-1 flex items-center">
                            <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 10l7-7m0 0l7 7m-7-7v18"></path></svg>
                            {"同比昨日 +12.5%"}
                        </div>
                    </div>
                    
                    <div class="card bg-white p-4 md:p-5 flex flex-col justify-between">
                        <div class="flex justify-between items-center mb-2">
                            <span class="text-sm text-[#4E5969] font-medium">{"今日销售单数"}</span>
                            <div class="w-8 h-8 rounded-full bg-[#E8F3FF] text-[#165DFF] flex items-center justify-center">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"></path></svg>
                            </div>
                        </div>
                        <div class="text-[24px] font-bold text-[#1D2129]">{"45"}</div>
                        <div class="text-xs text-[#F53F3F] mt-1 flex items-center">
                            <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 14l-7 7m0 0l-7-7m7 7V3"></path></svg>
                            {"同比昨日 -2.1%"}
                        </div>
                    </div>
                    
                    <div class="card bg-white p-4 md:p-5 flex flex-col justify-between">
                        <div class="flex justify-between items-center mb-2">
                            <span class="text-sm text-[#4E5969] font-medium">{"当前库存总额"}</span>
                            <div class="w-8 h-8 rounded-full bg-[#E8F3FF] text-[#165DFF] flex items-center justify-center">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"></path></svg>
                            </div>
                        </div>
                        <div class="text-[24px] font-bold text-[#1D2129]">{"¥3,450,200"}</div>
                        <div class="text-xs text-[#86909C] mt-1 flex gap-2">
                            <span class="badge-knit px-1 py-0 rounded text-[10px]">{"针 65%"}</span>
                            <span class="badge-woven px-1 py-0 rounded text-[10px]">{"梭 35%"}</span>
                        </div>
                    </div>
                    
                    <div class="card bg-white p-4 md:p-5 flex flex-col justify-between">
                        <div class="flex justify-between items-center mb-2">
                            <span class="text-sm text-[#4E5969] font-medium">{"逾期应收金额"}</span>
                            <div class="w-8 h-8 rounded-full bg-[#FFECE8] text-[#F53F3F] flex items-center justify-center">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
                            </div>
                        </div>
                        <div class="text-[24px] font-bold text-[#F53F3F]">{"¥85,600.00"}</div>
                        <div class="text-xs text-[#F53F3F] mt-1">{"共计 4 笔逾期款项"}</div>
                    </div>
                </div>

                
                <div class="grid grid-cols-1 md:grid-cols-2 gap-3 md:gap-4">
                    
                    <div class="card bg-white p-0 overflow-hidden">
                        <div class="px-4 py-3 border-b border-[#E5E6EB] font-bold text-[16px] text-[#1D2129]">{"待处理事项"}</div>
                        <div class="divide-y divide-[#E5E6EB]">
                            <div class="flex items-center justify-between p-4 hover:bg-[#F5F7FA] transition-colors">
                                <div class="flex items-center gap-3">
                                    <div class="w-8 h-8 rounded bg-[#FFF3E8] text-[#FF7D00] flex items-center justify-center"><svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"></path></svg></div>
                                    <div>
                                        <div class="text-[14px] text-[#1D2129]">{"待审核订单"}</div>
                                        <div class="text-[12px] text-[#FF7D00] mt-0.5">{"12 笔待审核"}</div>
                                    </div>
                                </div>
                                <button class="btn-text text-[14px]">{"去处理"}</button>
                            </div>
                            <div class="flex items-center justify-between p-4 hover:bg-[#F5F7FA] transition-colors">
                                <div class="flex items-center gap-3">
                                    <div class="w-8 h-8 rounded bg-[#FFF3E8] text-[#FF7D00] flex items-center justify-center"><svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4"></path></svg></div>
                                    <div>
                                        <div class="text-[14px] text-[#1D2129]">{"待发货订单"}</div>
                                        <div class="text-[12px] text-[#FF7D00] mt-0.5">{"8 笔待发货"}</div>
                                    </div>
                                </div>
                                <button class="btn-text text-[14px]">{"去处理"}</button>
                            </div>
                            <div class="flex items-center justify-between p-4 hover:bg-[#F5F7FA] transition-colors">
                                <div class="flex items-center gap-3">
                                    <div class="w-8 h-8 rounded bg-[#FFECE8] text-[#F53F3F] flex items-center justify-center"><svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path></svg></div>
                                    <div>
                                        <div class="text-[14px] text-[#1D2129]">{"库存预警"}</div>
                                        <div class="text-[12px] text-[#F53F3F] mt-0.5">{"5 款面料不足"}</div>
                                    </div>
                                </div>
                                <button class="btn-text text-[14px]">{"去查看"}</button>
                            </div>
                            <div class="flex items-center justify-between p-4 hover:bg-[#F5F7FA] transition-colors">
                                <div class="flex items-center gap-3">
                                    <div class="w-8 h-8 rounded bg-[#FFF3E8] text-[#FF7D00] flex items-center justify-center"><svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 14l6-6m-5.5.5h.01m4.99 5h.01M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16l3.5-2 3.5 2 3.5-2 3.5 2zM10 8.5a.5.5 0 11-1 0 .5.5 0 011 0zm5 5a.5.5 0 11-1 0 .5.5 0 011 0z"></path></svg></div>
                                    <div>
                                        <div class="text-[14px] text-[#1D2129]">{"待对账单据"}</div>
                                        <div class="text-[12px] text-[#FF7D00] mt-0.5">{"3 家客户待对"}</div>
                                    </div>
                                </div>
                                <button class="btn-text text-[14px]">{"去对账"}</button>
                            </div>
                        </div>
                    </div>
                    
                    <div class="card bg-white p-0 overflow-hidden">
                        <div class="px-4 py-3 border-b border-[#E5E6EB] font-bold text-[16px] text-[#1D2129]">{"常用操作"}</div>
                        <div class="grid grid-cols-2 gap-px bg-[#E5E6EB]">
                            <div class="bg-white p-6 flex flex-col items-center justify-center cursor-pointer hover:bg-[#F5F7FA] transition-colors group">
                                <div class="w-10 h-10 rounded-full bg-[#E8F3FF] text-[#165DFF] flex items-center justify-center mb-2 group-hover:scale-110 transition-transform"><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path></svg></div>
                                <span class="text-[14px] text-[#1D2129]">{"新增销售单"}</span>
                            </div>
                            <div class="bg-white p-6 flex flex-col items-center justify-center cursor-pointer hover:bg-[#F5F7FA] transition-colors group">
                                <div class="w-10 h-10 rounded-full bg-[#E8F3FF] text-[#165DFF] flex items-center justify-center mb-2 group-hover:scale-110 transition-transform"><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 3h2l.4 2M7 13h10l4-8H5.4M7 13L5.4 5M7 13l-2.293 2.293c-.63.63-.184 1.707.707 1.707H17m0 0a2 2 0 100 4 2 2 0 000-4zm-8 2a2 2 0 11-4 0 2 2 0 014 0z"></path></svg></div>
                                <span class="text-[14px] text-[#1D2129]">{"新增采购单"}</span>
                            </div>
                            <div class="bg-white p-6 flex flex-col items-center justify-center cursor-pointer hover:bg-[#F5F7FA] transition-colors group">
                                <div class="w-10 h-10 rounded-full bg-[#E8F3FF] text-[#165DFF] flex items-center justify-center mb-2 group-hover:scale-110 transition-transform"><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path></svg></div>
                                <span class="text-[14px] text-[#1D2129]">{"库存查询"}</span>
                            </div>
                            <div class="bg-white p-6 flex flex-col items-center justify-center cursor-pointer hover:bg-[#F5F7FA] transition-colors group">
                                <div class="w-10 h-10 rounded-full bg-[#E8F3FF] text-[#165DFF] flex items-center justify-center mb-2 group-hover:scale-110 transition-transform"><svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 17h2a2 2 0 002-2v-4a2 2 0 00-2-2H5a2 2 0 00-2 2v4a2 2 0 002 2h2m2 4h6a2 2 0 002-2v-4a2 2 0 00-2-2H9a2 2 0 00-2 2v4a2 2 0 002 2zm8-12V5a2 2 0 00-2-2H9a2 2 0 00-2 2v4h10z"></path></svg></div>
                                <span class="text-[14px] text-[#1D2129]">{"打印中心"}</span>
                            </div>
                        </div>
                    </div>
                </div>

                
                if *is_mobile {
                    <div class="card bg-white p-0 pb-20">
                        <div class="flex items-center justify-between p-2 border-b border-[#E5E6EB] bg-[#F5F7FA] text-[14px]">
                            <button
                                onclick={let on_chart_tab_change = on_chart_tab_change.clone(); Callback::from(move |_| on_chart_tab_change.emit("trend"))}
                                class={format!("flex-1 py-2 text-center rounded transition-colors {}", if *active_chart_tab == "trend" { "bg-white text-[#165DFF] shadow-sm font-bold" } else { "text-[#4E5969]" })}>
                                {"销售趋势"}
                            </button>
                            <button
                                onclick={let on_chart_tab_change = on_chart_tab_change.clone(); Callback::from(move |_| on_chart_tab_change.emit("ratio"))}
                                class={format!("flex-1 py-2 text-center rounded transition-colors {}", if *active_chart_tab == "ratio" { "bg-white text-[#165DFF] shadow-sm font-bold" } else { "text-[#4E5969]" })}>
                                {"品类占比"}
                            </button>
                            <button
                                onclick={let on_chart_tab_change = on_chart_tab_change.clone(); Callback::from(move |_| on_chart_tab_change.emit("top10"))}
                                class={format!("flex-1 py-2 text-center rounded transition-colors {}", if *active_chart_tab == "top10" { "bg-white text-[#165DFF] shadow-sm font-bold" } else { "text-[#4E5969]" })}>
                                {"热销面料"}
                            </button>
                        </div>
                        <div class="p-4">
                            if *active_chart_tab == "trend" {
                                <div class="flex justify-end mb-2">
                                    <select class="w-24 h-7 text-xs py-0 border-[#E5E6EB] rounded">
                                        <option>{"销售额"}</option>
                                        <option>{"订单数"}</option>
                                    </select>
                                </div>
                                <div class="h-48 flex items-end justify-between bg-[#F5F7FA] rounded p-4 pb-0 mt-2">
                                    <div class="w-1/6 bg-[#165DFF] h-[40%] rounded-t opacity-80 hover:opacity-100 transition-all relative group"><div class="absolute -top-6 left-1/2 -translate-x-1/2 text-xs text-[#4E5969] hidden group-hover:block">{"2.1k"}</div></div>
                                    <div class="w-1/6 bg-[#165DFF] h-[60%] rounded-t opacity-80 hover:opacity-100 transition-all relative group"><div class="absolute -top-6 left-1/2 -translate-x-1/2 text-xs text-[#4E5969] hidden group-hover:block">{"3.2k"}</div></div>
                                    <div class="w-1/6 bg-[#165DFF] h-[50%] rounded-t opacity-80 hover:opacity-100 transition-all relative group"><div class="absolute -top-6 left-1/2 -translate-x-1/2 text-xs text-[#4E5969] hidden group-hover:block">{"2.8k"}</div></div>
                                    <div class="w-1/6 bg-[#165DFF] h-[80%] rounded-t opacity-80 hover:opacity-100 transition-all relative group"><div class="absolute -top-6 left-1/2 -translate-x-1/2 text-xs text-[#4E5969] hidden group-hover:block">{"4.5k"}</div></div>
                                    <div class="w-1/6 bg-[#165DFF] h-[100%] rounded-t opacity-80 hover:opacity-100 transition-all relative group"><div class="absolute -top-6 left-1/2 -translate-x-1/2 text-xs text-[#1D2129] font-bold hidden group-hover:block">{"5.1k"}</div></div>
                                </div>
                                <div class="flex justify-between mt-2 text-[10px] text-[#86909C] px-2">
                                    <span>{"04-05"}</span><span>{"04-06"}</span><span>{"04-07"}</span><span>{"04-08"}</span><span>{"04-09"}</span>
                                </div>
                            } else if *active_chart_tab == "ratio" {
                                <div class="h-48 flex items-center justify-center bg-[#F5F7FA] rounded mt-4 relative">
                                    <div class="w-32 h-32 rounded-full border-[16px] border-[#4CAF50] border-r-[#2196F3] border-b-[#2196F3] transform -rotate-45 relative">
                                        <div class="absolute inset-0 m-auto w-full h-full flex flex-col items-center justify-center rotate-45">
                                            <span class="text-xs text-[#86909C]">{"总计"}</span>
                                            <span class="text-sm font-bold text-[#1D2129]">{"3,450"}</span>
                                        </div>
                                    </div>
                                    <div class="absolute right-4 top-1/2 -translate-y-1/2 flex flex-col gap-2">
                                        <div class="flex items-center gap-1 text-xs"><span class="w-2 h-2 rounded-full bg-[#4CAF50]"></span>{"针织 65%"}</div>
                                        <div class="flex items-center gap-1 text-xs"><span class="w-2 h-2 rounded-full bg-[#2196F3]"></span>{"梭织 35%"}</div>
                                    </div>
                                </div>
                            } else if *active_chart_tab == "top10" {
                                <div class="h-48 flex flex-col justify-center gap-3 bg-[#F5F7FA] rounded mt-4 p-4">
                                    <div class="flex items-center gap-2">
                                        <span class="text-xs text-[#4E5969] w-12 truncate">{"32S精梳棉"}</span>
                                        <div class="h-4 bg-[#165DFF] rounded opacity-100" style="width: 80%;"></div>
                                        <span class="text-xs text-[#1D2129] font-bold">{"1.2k"}</span>
                                    </div>
                                    <div class="flex items-center gap-2">
                                        <span class="text-xs text-[#4E5969] w-12 truncate">{"莫代尔拉架"}</span>
                                        <div class="h-4 bg-[#165DFF] rounded opacity-80" style="width: 65%;"></div>
                                        <span class="text-xs text-[#1D2129] font-bold">{"950"}</span>
                                    </div>
                                    <div class="flex items-center gap-2">
                                        <span class="text-xs text-[#4E5969] w-12 truncate">{"奥代尔平纹"}</span>
                                        <div class="h-4 bg-[#165DFF] rounded opacity-60" style="width: 45%;"></div>
                                        <span class="text-xs text-[#1D2129] font-bold">{"620"}</span>
                                    </div>
                                    <div class="flex items-center gap-2">
                                        <span class="text-xs text-[#4E5969] w-12 truncate">{"罗马布"}</span>
                                        <div class="h-4 bg-[#165DFF] rounded opacity-40" style="width: 30%;"></div>
                                        <span class="text-xs text-[#1D2129] font-bold">{"410"}</span>
                                    </div>
                                </div>
                            }
                        </div>
                    </div>
                } else {
                    <div class="grid grid-cols-1 md:grid-cols-3 gap-3 md:gap-4">
                        <div class="card bg-white p-0">
                            <div class="px-4 py-3 border-b border-[#E5E6EB] font-bold text-[16px] text-[#1D2129] flex justify-between items-center">
                                {"近7日销售趋势"}
                                <select class="w-24 h-7 text-xs py-0 border-[#E5E6EB] rounded">
                                    <option>{"销售额"}</option>
                                    <option>{"订单数"}</option>
                                </select>
                            </div>
                            <div class="h-48 p-4 flex flex-col justify-end bg-[#F5F7FA] m-4 rounded relative">
                                <div class="flex items-end justify-between h-full px-2">
                                    <div class="w-[10%] bg-[#165DFF] h-[40%] rounded-t opacity-80 hover:opacity-100 transition-all relative group"><div class="absolute -top-6 left-1/2 -translate-x-1/2 text-xs text-[#4E5969] hidden group-hover:block">{"2.1k"}</div></div>
                                    <div class="w-[10%] bg-[#165DFF] h-[60%] rounded-t opacity-80 hover:opacity-100 transition-all relative group"><div class="absolute -top-6 left-1/2 -translate-x-1/2 text-xs text-[#4E5969] hidden group-hover:block">{"3.2k"}</div></div>
                                    <div class="w-[10%] bg-[#165DFF] h-[50%] rounded-t opacity-80 hover:opacity-100 transition-all relative group"><div class="absolute -top-6 left-1/2 -translate-x-1/2 text-xs text-[#4E5969] hidden group-hover:block">{"2.8k"}</div></div>
                                    <div class="w-[10%] bg-[#165DFF] h-[80%] rounded-t opacity-80 hover:opacity-100 transition-all relative group"><div class="absolute -top-6 left-1/2 -translate-x-1/2 text-xs text-[#4E5969] hidden group-hover:block">{"4.5k"}</div></div>
                                    <div class="w-[10%] bg-[#165DFF] h-[100%] rounded-t opacity-80 hover:opacity-100 transition-all relative group"><div class="absolute -top-6 left-1/2 -translate-x-1/2 text-xs text-[#1D2129] font-bold hidden group-hover:block">{"5.1k"}</div></div>
                                    <div class="w-[10%] bg-[#165DFF] h-[70%] rounded-t opacity-80 hover:opacity-100 transition-all relative group"><div class="absolute -top-6 left-1/2 -translate-x-1/2 text-xs text-[#4E5969] hidden group-hover:block">{"3.8k"}</div></div>
                                    <div class="w-[10%] bg-[#165DFF] h-[90%] rounded-t opacity-80 hover:opacity-100 transition-all relative group"><div class="absolute -top-6 left-1/2 -translate-x-1/2 text-xs text-[#1D2129] font-bold hidden group-hover:block">{"4.9k"}</div></div>
                                </div>
                                <div class="flex justify-between mt-2 text-[10px] text-[#86909C] px-2">
                                    <span>{"04-03"}</span><span>{"04-04"}</span><span>{"04-05"}</span><span>{"04-06"}</span><span>{"04-07"}</span><span>{"04-08"}</span><span>{"04-09"}</span>
                                </div>
                            </div>
                        </div>

                        <div class="card bg-white p-0">
                            <div class="px-4 py-3 border-b border-[#E5E6EB] font-bold text-[16px] text-[#1D2129]">{"针织/梭织销量占比"}</div>
                            <div class="h-48 p-4 flex items-center justify-center bg-[#F5F7FA] m-4 rounded relative">
                                <div class="w-32 h-32 rounded-full border-[16px] border-[#4CAF50] border-r-[#2196F3] border-b-[#2196F3] transform -rotate-45 relative">
                                    <div class="absolute inset-0 m-auto w-full h-full flex flex-col items-center justify-center rotate-45">
                                        <span class="text-xs text-[#86909C]">{"总计"}</span>
                                        <span class="text-sm font-bold text-[#1D2129]">{"3,450"}</span>
                                    </div>
                                </div>
                                <div class="absolute right-4 top-1/2 -translate-y-1/2 flex flex-col gap-2">
                                    <div class="flex items-center gap-1 text-xs"><span class="w-2 h-2 rounded-full bg-[#4CAF50]"></span>{"针织 65%"}</div>
                                    <div class="flex items-center gap-1 text-xs"><span class="w-2 h-2 rounded-full bg-[#2196F3]"></span>{"梭织 35%"}</div>
                                </div>
                            </div>
                        </div>

                        <div class="card bg-white p-0">
                            <div class="px-4 py-3 border-b border-[#E5E6EB] font-bold text-[16px] text-[#1D2129]">{"热销面料 TOP 10"}</div>
                            <div class="h-48 p-4 flex flex-col justify-center gap-3 bg-[#F5F7FA] m-4 rounded">
                                <div class="flex items-center gap-2">
                                    <span class="text-xs text-[#4E5969] w-16 truncate">{"32S精梳棉"}</span>
                                    <div class="h-4 bg-[#165DFF] rounded opacity-100" style="width: 80%;"></div>
                                    <span class="text-xs text-[#1D2129] font-bold">{"1.2k"}</span>
                                </div>
                                <div class="flex items-center gap-2">
                                    <span class="text-xs text-[#4E5969] w-16 truncate">{"莫代尔拉架"}</span>
                                    <div class="h-4 bg-[#165DFF] rounded opacity-80" style="width: 65%;"></div>
                                    <span class="text-xs text-[#1D2129] font-bold">{"950"}</span>
                                </div>
                                <div class="flex items-center gap-2">
                                    <span class="text-xs text-[#4E5969] w-16 truncate">{"奥代尔平纹"}</span>
                                    <div class="h-4 bg-[#165DFF] rounded opacity-60" style="width: 45%;"></div>
                                    <span class="text-xs text-[#1D2129] font-bold">{"620"}</span>
                                </div>
                                <div class="flex items-center gap-2">
                                    <span class="text-xs text-[#4E5969] w-16 truncate">{"罗马布"}</span>
                                    <div class="h-4 bg-[#165DFF] rounded opacity-40" style="width: 30%;"></div>
                                    <span class="text-xs text-[#1D2129] font-bold">{"410"}</span>
                                </div>
                            </div>
                        </div>
                    </div>
                }

            </div>
        </MainLayout>
    }
}
