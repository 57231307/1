import os

dashboard_content = """use crate::components::main_layout::MainLayout;
use yew::prelude::*;

#[function_component(DashboardPage)]
pub fn dashboard_page() -> Html {
    html! {
        <MainLayout current_page="仪表板">
            <div class="space-y-4 md:space-y-6">
                
                {/* Top Data Cards */}
                <div class="grid grid-cols-2 md:grid-cols-4 gap-3 md:gap-4">
                    {/* 今日销售额 */}
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
                    {/* 今日销售单数 */}
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
                    {/* 当前库存总额 */}
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
                    {/* 逾期应收金额 */}
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

                {/* Middle Todos */}
                <div class="grid grid-cols-1 md:grid-cols-2 gap-3 md:gap-4">
                    {/* 待处理事项 */}
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
                    {/* 常用操作 */}
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

                {/* Bottom Charts */}
                <div class="grid grid-cols-1 md:grid-cols-3 gap-3 md:gap-4">
                    {/* 近7日销售趋势 */}
                    <div class="card bg-white p-0">
                        <div class="px-4 py-3 border-b border-[#E5E6EB] font-bold text-[16px] text-[#1D2129] flex justify-between items-center">
                            {"近7日销售趋势"}
                            <select class="w-24 h-7 text-xs py-0 border-[#E5E6EB] rounded">
                                <option>{"销售额"}</option>
                                <option>{"订单数"}</option>
                            </select>
                        </div>
                        <div class="h-48 p-4 flex items-center justify-center bg-[#F5F7FA] m-4 rounded text-[#86909C] text-sm">
                            {"[折线图表占位]"}
                        </div>
                    </div>
                    {/* 针织/梭织销量占比 */}
                    <div class="card bg-white p-0">
                        <div class="px-4 py-3 border-b border-[#E5E6EB] font-bold text-[16px] text-[#1D2129]">{"针织/梭织销量占比"}</div>
                        <div class="h-48 p-4 flex items-center justify-center bg-[#F5F7FA] m-4 rounded text-[#86909C] text-sm">
                            {"[饼图占位]"}
                        </div>
                    </div>
                    {/* 热销面料TOP10 */}
                    <div class="card bg-white p-0">
                        <div class="px-4 py-3 border-b border-[#E5E6EB] font-bold text-[16px] text-[#1D2129]">{"热销面料 TOP 10"}</div>
                        <div class="h-48 p-4 flex items-center justify-center bg-[#F5F7FA] m-4 rounded text-[#86909C] text-sm">
                            {"[横向柱状图占位]"}
                        </div>
                    </div>
                </div>

            </div>
        </MainLayout>
    }
}
"""

with open('frontend/src/pages/dashboard.rs', 'w', encoding='utf-8') as f:
    f.write(dashboard_content)

print("Dashboard updated.")
