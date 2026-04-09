//! CRM 线索管理页面

use crate::components::main_layout::MainLayout;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub struct CrmLeadItem {
    pub id: i32,
    pub lead_no: String,
    pub name: String,
    pub source: String,
    pub status: String,
}

#[function_component(CrmLeadPage)]
pub fn crm_lead_page() -> Html {
    let leads = use_state(|| Vec::<CrmLeadItem>::new());

    {
        let leads = leads.clone();
        use_effect_with((), move |_| {
            leads.set(vec![
                CrmLeadItem {
                    id: 1,
                    lead_no: "LD-2024001".to_string(),
                    name: "江苏某服装厂采购意向".to_string(),
                    source: "上海纺织展会".to_string(),
                    status: "初步接触".to_string(),
                },
                CrmLeadItem {
                    id: 2,
                    lead_no: "LD-2024002".to_string(),
                    name: "浙江某家纺企业询价".to_string(),
                    source: "线上推广".to_string(),
                    status: "跟进中".to_string(),
                },
                CrmLeadItem {
                    id: 3,
                    lead_no: "LD-2024003".to_string(),
                    name: "福建运动服品牌面料需求".to_string(),
                    source: "客户介绍".to_string(),
                    status: "已转化为商机".to_string(),
                },
            ]);
            || ()
        });
    }

    html! {
        <MainLayout current_page={"crm_lead"}>
            <div class="p-4">
                <h1 class="text-2xl font-bold mb-4">{ "CRM 线索管理" }</h1>
                
                <div class="mb-4 flex justify-between items-center bg-white p-4 rounded shadow-sm border border-gray-200">
                    <div class="flex space-x-4">
                        <input type="text" placeholder="搜索线索名称/编号..." class="border border-gray-300 p-2 rounded w-64 focus:outline-none focus:ring-2 focus:ring-blue-500" />
                        <select class="border border-gray-300 p-2 rounded focus:outline-none focus:ring-2 focus:ring-blue-500">
                            <option value="">{ "全部来源" }</option>
                            <option value="exhibition">{ "展会" }</option>
                            <option value="online">{ "线上" }</option>
                        </select>
                        <button class="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded transition-colors">{ "筛选" }</button>
                    </div>
                    <button class="bg-green-500 hover:bg-green-600 text-white px-4 py-2 rounded transition-colors flex items-center">
                        <span class="mr-1">{ "+" }</span> { "新建线索" }
                    </button>
                </div>

                <div class="overflow-x-auto bg-white rounded shadow-sm border border-gray-200">
                    <table class="data-table w-full text-left border-collapse">
                        <thead class="bg-gray-50 border-b border-gray-200">
                            <tr>
                                <th class="py-3 px-4 text-gray-700 font-semibold">{ "线索编号" }</th>
                                <th class="py-3 px-4 text-gray-700 font-semibold">{ "线索名称" }</th>
                                <th class="py-3 px-4 text-gray-700 font-semibold">{ "来源" }</th>
                                <th class="py-3 px-4 text-gray-700 font-semibold">{ "状态" }</th>
                                <th class="py-3 px-4 text-gray-700 font-semibold text-center">{ "操作" }</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-gray-200">
                            {
                                if leads.is_empty() {
                                    html! { <tr><td colspan="5" class="text-center py-8 text-gray-500">{ "暂无数据" }</td></tr> }
                                } else {
                                    html! {
                                        for leads.iter().map(|lead| html! {
                                            <tr key={lead.id} class="hover:bg-gray-50 transition-colors">
                                                <td class="py-3 px-4 text-gray-600">{ &lead.lead_no }</td>
                                                <td class="py-3 px-4 font-medium text-gray-800">{ &lead.name }</td>
                                                <td class="py-3 px-4 text-gray-600">{ &lead.source }</td>
                                                <td class="py-3 px-4"><span class="status-badge px-2 py-1 rounded text-sm bg-blue-100 text-blue-800">{ &lead.status }</span></td>
                                                <td class="py-3 px-4 text-center">
                                                    <button class="text-blue-600 hover:text-blue-800 mr-3">{ "查看" }</button>
                                                    <button class="text-green-600 hover:text-green-800">{ "转商机" }</button>
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
