use crate::components::main_layout::MainLayout;
use yew::prelude::*;

#[function_component(DashboardPage)]
pub fn dashboard_page() -> Html {
    html! {
        <MainLayout current_page="仪表板">
            <div class="space-y-6">
                <div class="flex flex-col md:flex-row justify-between items-start md:items-center">
                    <div>
                        <h2 class="text-2xl font-bold tracking-tight text-slate-900">{"概览"}</h2>
                        <p class="text-sm text-slate-500">{"欢迎回来，这里是您的二批面料贸易管家实时数据。"}</p>
                    </div>
                    <div class="flex gap-2 mt-4 md:mt-0">
                        <button class="btn-outline text-sm text-slate-600">{"📅 本月"}</button>
                        <button class="btn-primary text-sm">{"下载报表"}</button>
                    </div>
                </div>

                <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
                    <div class="card p-6 bg-white shadow-sm border-slate-200">
                        <div class="flex flex-row items-center justify-between space-y-0 pb-2">
                            <h3 class="tracking-tight text-sm font-medium text-slate-500">{"今日销售总额"}</h3>
                            <svg class="h-4 w-4 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
                        </div>
                        <div class="text-2xl font-bold text-slate-900">{"¥45,231.89"}</div>
                        <p class="text-xs text-green-600 font-medium mt-1">{"+20.1% 较昨日"}</p>
                    </div>
                    <div class="card p-6 bg-white shadow-sm border-slate-200">
                        <div class="flex flex-row items-center justify-between space-y-0 pb-2">
                            <h3 class="tracking-tight text-sm font-medium text-slate-500">{"待收账款 (AR)"}</h3>
                            <svg class="h-4 w-4 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 9V7a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2m2 4h10a2 2 0 002-2v-6a2 2 0 00-2-2H9a2 2 0 00-2 2v6a2 2 0 002 2zm7-5a2 2 0 11-4 0 2 2 0 014 0z"></path></svg>
                        </div>
                        <div class="text-2xl font-bold text-slate-900">{"¥124,500.00"}</div>
                        <p class="text-xs text-red-500 font-medium mt-1">{"有 3 笔账款已逾期"}</p>
                    </div>
                    <div class="card p-6 bg-white shadow-sm border-slate-200">
                        <div class="flex flex-row items-center justify-between space-y-0 pb-2">
                            <h3 class="tracking-tight text-sm font-medium text-slate-500">{"库存预警"}</h3>
                            <svg class="h-4 w-4 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"></path></svg>
                        </div>
                        <div class="text-2xl font-bold text-slate-900">{"12 款"}</div>
                        <p class="text-xs text-slate-500 mt-1">{"低于安全库存水位"}</p>
                    </div>
                    <div class="card p-6 bg-white shadow-sm border-slate-200">
                        <div class="flex flex-row items-center justify-between space-y-0 pb-2">
                            <h3 class="tracking-tight text-sm font-medium text-slate-500">{"待发货订单"}</h3>
                            <svg class="h-4 w-4 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path></svg>
                        </div>
                        <div class="text-2xl font-bold text-slate-900">{"8 笔"}</div>
                        <p class="text-xs text-blue-600 font-medium mt-1">{"需今日内处理"}</p>
                    </div>
                </div>

                <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-7">
                    <div class="card col-span-4 p-6 bg-white shadow-sm border-slate-200">
                        <div class="flex flex-col space-y-1.5 pb-4 border-b border-slate-100">
                            <h3 class="font-semibold leading-none tracking-tight text-slate-900">{"销售趋势"}</h3>
                        </div>
                        <div class="pt-4 h-64 flex items-center justify-center bg-slate-50 rounded-md border border-slate-100 mt-2">
                            <span class="text-slate-400 text-sm">{"[折线图表占位区] 此处接入 ECharts/Chart.js"}</span>
                        </div>
                    </div>
                    <div class="card col-span-3 p-6 bg-white shadow-sm border-slate-200">
                        <div class="flex flex-col space-y-1.5 pb-4 border-b border-slate-100">
                            <h3 class="font-semibold leading-none tracking-tight text-slate-900">{"近期低库存面料 (Top 5)"}</h3>
                        </div>
                        <div class="pt-4 space-y-4">
                            <div class="flex items-center">
                                <div class="w-2 h-2 bg-red-500 rounded-full mr-2"></div>
                                <div class="ml-2 space-y-1">
                                    <p class="text-sm font-medium leading-none text-slate-900">{"SJ-100C 全棉汗布 (32S)"}</p>
                                    <p class="text-sm text-slate-500">{"剩余: 120 kg | 安全线: 500 kg"}</p>
                                </div>
                                <div class="ml-auto font-medium text-sm text-blue-600 cursor-pointer">{"立即采购"}</div>
                            </div>
                            <div class="flex items-center">
                                <div class="w-2 h-2 bg-orange-400 rounded-full mr-2"></div>
                                <div class="ml-2 space-y-1">
                                    <p class="text-sm font-medium leading-none text-slate-900">{"PK-6535 CVC珠地网眼"}</p>
                                    <p class="text-sm text-slate-500">{"剩余: 230 kg | 安全线: 300 kg"}</p>
                                </div>
                                <div class="ml-auto font-medium text-sm text-blue-600 cursor-pointer">{"立即采购"}</div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </MainLayout>
    }
}
