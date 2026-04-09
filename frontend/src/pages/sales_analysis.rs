//! 销售分析页面

use crate::components::main_layout::MainLayout;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub struct SalesAnalysisItem {
    pub id: i32,
    pub region: String,
    pub target_amount: f64,
    pub actual_amount: f64,
    pub completion_rate: f64,
    pub status: String,
}

#[function_component(SalesAnalysisPage)]
pub fn sales_analysis_page() -> Html {
    let items = use_state(|| Vec::<SalesAnalysisItem>::new());

    {
        let items = items.clone();
        use_effect_with((), move |_| {
            items.set(vec![
                SalesAnalysisItem {
                    id: 1,
                    region: "江苏服装厂大客户组".to_string(),
                    target_amount: 500000.0,
                    actual_amount: 450000.0,
                    completion_rate: 90.0,
                    status: "正常进度".to_string(),
                },
                SalesAnalysisItem {
                    id: 2,
                    region: "浙江印染企业专区".to_string(),
                    target_amount: 300000.0,
                    actual_amount: 350000.0,
                    completion_rate: 116.6,
                    status: "超额完成".to_string(),
                },
                SalesAnalysisItem {
                    id: 3,
                    region: "广东夏装面料市场".to_string(),
                    target_amount: 800000.0,
                    actual_amount: 400000.0,
                    completion_rate: 50.0,
                    status: "需关注".to_string(),
                },
            ]);
            || ()
        });
    }

    html! {
        <MainLayout current_page={"sales_analysis"}>
            <div class="p-4">
                <h1 class="text-2xl font-bold mb-4">{ "销售分析" }</h1>
                
                <div class="mb-4 flex justify-between items-center bg-white p-4 rounded shadow-sm border border-gray-200">
                    <div class="flex space-x-4">
                        <input type="text" placeholder="搜索地区/客户群..." class="border border-gray-300 p-2 rounded w-64 focus:outline-none focus:ring-2 focus:ring-blue-500" />
                        <select class="border border-gray-300 p-2 rounded focus:outline-none focus:ring-2 focus:ring-blue-500">
                            <option value="">{ "全部周期" }</option>
                            <option value="q1">{ "第一季度" }</option>
                            <option value="q2">{ "第二季度" }</option>
                        </select>
                        <button class="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded transition-colors">{ "查询" }</button>
                    </div>
                    <button class="bg-indigo-500 hover:bg-indigo-600 text-white px-4 py-2 rounded transition-colors flex items-center">
                        <span class="mr-1">{ "↓" }</span> { "导出报表" }
                    </button>
                </div>

                <div class="overflow-x-auto bg-white rounded shadow-sm border border-gray-200">
                    <table class="data-table w-full text-left border-collapse">
                        <thead class="bg-gray-50 border-b border-gray-200">
                            <tr>
                                <th class="py-3 px-4 text-gray-700 font-semibold">{ "地区/客户群" }</th>
                                <th class="py-3 px-4 text-gray-700 font-semibold numeric-cell text-right">{ "目标金额(元)" }</th>
                                <th class="py-3 px-4 text-gray-700 font-semibold numeric-cell text-right">{ "实际金额(元)" }</th>
                                <th class="py-3 px-4 text-gray-700 font-semibold numeric-cell text-right">{ "完成率(%)" }</th>
                                <th class="py-3 px-4 text-gray-700 font-semibold">{ "状态" }</th>
                                <th class="py-3 px-4 text-gray-700 font-semibold text-center">{ "操作" }</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-gray-200">
                            {
                                if items.is_empty() {
                                    html! { <tr><td colspan="6" class="text-center py-8 text-gray-500">{ "暂无数据" }</td></tr> }
                                } else {
                                    html! {
                                        for items.iter().map(|item| html! {
                                            <tr key={item.id} class="hover:bg-gray-50 transition-colors">
                                                <td class="py-3 px-4 font-medium text-gray-800">{ &item.region }</td>
                                                <td class="py-3 px-4 numeric-cell text-right">{ format!("{:.2}", item.target_amount) }</td>
                                                <td class="py-3 px-4 numeric-cell text-right text-green-600 font-medium">{ format!("{:.2}", item.actual_amount) }</td>
                                                <td class="py-3 px-4 numeric-cell text-right font-medium">{ format!("{:.1}", item.completion_rate) }</td>
                                                <td class="py-3 px-4"><span class="status-badge px-2 py-1 rounded text-sm bg-blue-100 text-blue-800">{ &item.status }</span></td>
                                                <td class="py-3 px-4 text-center">
                                                    <button class="text-blue-600 hover:text-blue-800">{ "查看详情" }</button>
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
