//! 供应商评估页面

use crate::components::main_layout::MainLayout;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub struct SupplierEvalItem {
    pub id: i32,
    pub supplier_name: String,
    pub evaluation_period: String,
    pub overall_score: f64,
    pub delivery_score: f64,
    pub quality_score: f64,
    pub price_score: f64,
    pub grade: String,
    pub evaluation_date: String,
    pub status: String,
}

#[function_component(SupplierEvaluationPage)]
pub fn supplier_evaluation_page() -> Html {
    let evaluations = use_state(|| Vec::<SupplierEvalItem>::new());

    {
        let evaluations = evaluations.clone();
        use_effect_with((), move |_| {
            let initial_data = vec![
                SupplierEvalItem {
                    id: 1,
                    supplier_name: "浙江某印染厂".to_string(),
                    evaluation_period: "2023 Q3".to_string(),
                    overall_score: 92.5,
                    delivery_score: 90.0,
                    quality_score: 95.0,
                    price_score: 92.5,
                    grade: "A".to_string(),
                    evaluation_date: "2023-10-15".to_string(),
                    status: "已归档".to_string(),
                },
                SupplierEvalItem {
                    id: 2,
                    supplier_name: "福建某织造厂".to_string(),
                    evaluation_period: "2023 Q3".to_string(),
                    overall_score: 78.0,
                    delivery_score: 70.0,
                    quality_score: 80.0,
                    price_score: 84.0,
                    grade: "C".to_string(),
                    evaluation_date: "2023-10-18".to_string(),
                    status: "整改中".to_string(),
                },
            ];
            evaluations.set(initial_data);
            || ()
        });
    }

    html! {
        <MainLayout current_page={"供应商评估"}>
            <div class="p-4">
                <div class="header mb-4 flex justify-between items-center">
                    <h1 class="text-2xl font-bold">{ "纺织供应商评估" }</h1>
                    <button class="bg-blue-500 hover:bg-blue-600 text-white font-bold py-2 px-4 rounded">
                        {"+ 新增评估"}
                    </button>
                </div>

                <div class="bg-white p-4 rounded shadow mb-4">
                    <h2 class="text-lg font-semibold mb-2">{"新建评级"}</h2>
                    <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700">{"供应商"}</label>
                            <input type="text" class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500" placeholder="例如: 浙江某印染厂" />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700">{"评估周期"}</label>
                            <input type="text" class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500" placeholder="例如: 2023 Q4" />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700">{"总分"}</label>
                            <input type="number" class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500" placeholder="0 - 100" />
                        </div>
                        <div class="flex items-end">
                            <button class="w-full bg-green-500 hover:bg-green-600 text-white font-bold py-2 px-4 rounded">
                                {"提交"}
                            </button>
                        </div>
                    </div>
                </div>

                <div class="table-responsive">
                    <table class="data-table w-full text-left border-collapse">
                        <thead>
                            <tr class="bg-gray-100 border-b">
                                <th class="py-2 px-4 numeric-cell text-right">{"ID"}</th>
                                <th class="py-2 px-4">{"供应商名称"}</th>
                                <th class="py-2 px-4">{"评估周期"}</th>
                                <th class="py-2 px-4 numeric-cell text-right">{"总分"}</th>
                                <th class="py-2 px-4 numeric-cell text-right">{"交期得分"}</th>
                                <th class="py-2 px-4 numeric-cell text-right">{"质量得分"}</th>
                                <th class="py-2 px-4 numeric-cell text-right">{"价格得分"}</th>
                                <th class="py-2 px-4 text-center">{"等级"}</th>
                                <th class="py-2 px-4">{"评估日期"}</th>
                                <th class="py-2 px-4">{"状态"}</th>
                                <th class="py-2 px-4">{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {
                                if evaluations.is_empty() {
                                    html! { <tr><td colspan="11" class="text-center py-4">{ "暂无数据" }</td></tr> }
                                } else {
                                    html! {
                                        for evaluations.iter().map(|evaluation| html! {
                                            <tr key={evaluation.id} class="border-b hover:bg-gray-50">
                                                <td class="py-2 px-4 numeric-cell text-right">{ evaluation.id }</td>
                                                <td class="py-2 px-4">{ &evaluation.supplier_name }</td>
                                                <td class="py-2 px-4">{ &evaluation.evaluation_period }</td>
                                                <td class="py-2 px-4 numeric-cell text-right font-medium">{ format!("{:.1}", evaluation.overall_score) }</td>
                                                <td class="py-2 px-4 numeric-cell text-right">{ format!("{:.1}", evaluation.delivery_score) }</td>
                                                <td class="py-2 px-4 numeric-cell text-right">{ format!("{:.1}", evaluation.quality_score) }</td>
                                                <td class="py-2 px-4 numeric-cell text-right">{ format!("{:.1}", evaluation.price_score) }</td>
                                                <td class="py-2 px-4 text-center">
                                                    <span class={format!("status-badge font-bold {}", match evaluation.grade.as_str() {
                                                        "A" => "text-green-600",
                                                        "B" => "text-blue-600",
                                                        "C" => "text-yellow-600",
                                                        _ => "text-red-600",
                                                    })}>{ &evaluation.grade }</span>
                                                </td>
                                                <td class="py-2 px-4">{ &evaluation.evaluation_date }</td>
                                                <td class="py-2 px-4">
                                                    <span class="status-badge bg-gray-100 text-gray-800 px-2 py-1 rounded text-sm">{ &evaluation.status }</span>
                                                </td>
                                                <td class="py-2 px-4">
                                                    <button class="text-blue-500 hover:text-blue-700 mr-2">{"详情"}</button>
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
