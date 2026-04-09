//! CRM 商机管理页面

use crate::components::main_layout::MainLayout;
use yew::prelude::*;
use web_sys::window;

#[derive(Clone, PartialEq)]
pub struct CrmOpportunityItem {
    pub id: i32,
    pub opportunity_no: String,
    pub name: String,
    pub intended_fabric: String,
    pub estimated_quantity: String,
    pub amount: f64,
    pub probability: f64,
    pub stage: String,
}

#[function_component(CrmOpportunityPage)]
pub fn crm_opportunity_page() -> Html {
    let opps = use_state(|| Vec::<CrmOpportunityItem>::new());
    let on_print = Callback::from(|_: yew::MouseEvent| {
        if let Some(win) = window() {
            let _ = win.print();
        }
    });


    {
        let opps = opps.clone();
        use_effect_with((), move |_| {
            opps.set(vec![
                CrmOpportunityItem {
                    id: 1,
                    opportunity_no: "OPP-2024001".to_string(),
                    name: "广东夏装全棉面料大单".to_string(),
                    intended_fabric: "100%全棉".to_string(),
                    estimated_quantity: "50000米".to_string(),
                    amount: 1200000.0,
                    probability: 80.0,
                    stage: "报价/谈判".to_string(),
                },
                CrmOpportunityItem {
                    id: 2,
                    opportunity_no: "OPP-2024002".to_string(),
                    name: "福建运动服化纤面料采购".to_string(),
                    intended_fabric: "90%涤纶 10%氨纶".to_string(),
                    estimated_quantity: "30000米".to_string(),
                    amount: 800000.0,
                    probability: 50.0,
                    stage: "需求分析".to_string(),
                },
                CrmOpportunityItem {
                    id: 3,
                    opportunity_no: "OPP-2024003".to_string(),
                    name: "海外品牌莫代尔面料代工".to_string(),
                    intended_fabric: "莫代尔混纺".to_string(),
                    estimated_quantity: "10万米".to_string(),
                    amount: 2500000.0,
                    probability: 30.0,
                    stage: "初步接触".to_string(),
                },
            ]);
            || ()
        });
    }

    html! {
        <MainLayout current_page={"crm_opportunity"}>
            <div class="p-2 text-sm">
                <h1 class="text-xl font-bold mb-2">{ "CRM 商机管理" }</h1>
                
                <div class="mb-2 flex justify-between items-center bg-white p-2 rounded shadow-sm border border-gray-200">
                    <div class="flex space-x-2">
                        <input type="text" placeholder="搜索商机名称/编号..." class="border border-gray-300 p-1 text-sm rounded w-64 focus:outline-none focus:ring-1 focus:ring-blue-500" />
                        <select class="border border-gray-300 p-1 text-sm rounded focus:outline-none focus:ring-1 focus:ring-blue-500">
                            <option value="">{ "所有阶段" }</option>
                            <option value="analysis">{ "需求分析" }</option>
                            <option value="negotiation">{ "报价/谈判" }</option>
                            <option value="closed_won">{ "赢单" }</option>
                        </select>
                        <button class="bg-blue-500 hover:bg-blue-600 text-white px-3 py-1 text-sm rounded transition-colors">{ "筛选" }</button>
                    </div>
                    <button class="bg-green-500 hover:bg-green-600 text-white px-3 py-1 text-sm rounded transition-colors flex items-center">
                        <span class="mr-1">{ "+" }</span> { "新建商机" }
                    </button>
                </div>

                <div class="overflow-x-auto bg-white rounded shadow-sm border border-gray-200">
                    <table class="data-table w-full text-left border-collapse">
                        <thead class="bg-gray-50 border-b border-gray-200">
                            <tr>
                                <th class="py-2 px-3 text-gray-700 font-semibold">{ "商机编号" }</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold">{ "商机名称" }</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold">{ "意向面料成分" }</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold">{ "预估米数/匹数" }</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold numeric-cell text-right">{ "预计金额(元)" }</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold numeric-cell text-right">{ "赢单概率(%)" }</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold">{ "当前阶段" }</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold text-center">{ "操作" }</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-gray-200">
                            {
                                if opps.is_empty() {
                                    html! { <tr><td colspan="8" class="text-center py-4 text-gray-500">{ "暂无数据" }</td></tr> }
                                } else {
                                    html! {
                                        for opps.iter().map(|opp| html! {
                                            <tr key={opp.id} class="hover:bg-gray-50 transition-colors">
                                                <td class="py-2 px-3 text-gray-600">{ &opp.opportunity_no }</td>
                                                <td class="py-2 px-3 font-medium text-gray-800">{ &opp.name }</td>
                                                <td class="py-2 px-3 text-gray-600">{ &opp.intended_fabric }</td>
                                                <td class="py-2 px-3 text-gray-600">{ &opp.estimated_quantity }</td>
                                                <td class="py-2 px-3 numeric-cell text-right text-green-600 font-medium">{ format!("{:.2}", opp.amount) }</td>
                                                <td class="py-2 px-3 numeric-cell text-right font-medium">{ format!("{:.1}", opp.probability) }</td>
                                                <td class="py-2 px-3"><span class="status-badge px-2 py-0.5 rounded text-xs bg-blue-100 text-blue-800">{ &opp.stage }</span></td>
                                                <td class="py-2 px-3 text-center">
                                                    <button class="text-blue-600 hover:text-blue-800 mr-2">{ "查看" }</button>
                                                    <button class="text-indigo-600 hover:text-indigo-800">{ "推进阶段" }</button>
                                                </td>
                                            </tr>
                                        })
                                    }
                                }
                            }
                        </tbody>
                    </table>
                </div>
            </div>
        </MainLayout>
    }
}
