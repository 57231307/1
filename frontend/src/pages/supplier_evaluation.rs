//! 供应商评估页面

use crate::components::main_layout::MainLayout;
use crate::models::supplier_evaluation::SupplierEvaluation;
use crate::services::supplier_evaluation_service::SupplierEvaluationService;
use yew::prelude::*;

#[function_component(SupplierEvaluationPage)]
pub fn supplier_evaluation_page() -> Html {
    let evaluations = use_state(|| Vec::<SupplierEvaluation>::new());

    {
        let evaluations = evaluations.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(res) = SupplierEvaluationService::list_evaluations(None, None, None, None, 1, 10).await {
                    evaluations.set(res);
                }
            });
            || ()
        });
    }

    html! {
        <MainLayout current_page={"供应商评估"}>
<div class="p-4">
            <h1 class="text-2xl font-bold mb-4">{ "供应商评估" }</h1>
            <table class="data-table w-full">
                <thead>
                    <tr>
                        <th class="py-2 px-4 border-b numeric-cell text-right">{ "供应商ID" }</th>
                        <th class="py-2 px-4 border-b">{ "评估周期" }</th>
                        <th class="py-2 px-4 border-b numeric-cell text-right">{ "总分" }</th>
                        <th class="py-2 px-4 border-b">{ "等级" }</th>
                        <th class="py-2 px-4 border-b">{ "评估日期" }</th>
                        <th class="py-2 px-4 border-b">{ "状态" }</th>
                    </tr>
                </thead>
                <tbody>
                    {
                        if evaluations.is_empty() {
                            html! { <tr><td colspan="6" class="text-center py-4">{ "暂无数据" }</td></tr> }
                        } else {
                            html! {
                                for evaluations.iter().map(|evaluation| html! {
                                    <tr key={evaluation.id}>
                                        <td class="py-2 px-4 border-b numeric-cell text-right">{ evaluation.supplier_id }</td>
                                        <td class="py-2 px-4 border-b text-center">{ &evaluation.evaluation_period }</td>
                                        <td class="py-2 px-4 border-b numeric-cell text-right">{ &evaluation.overall_score }</td>
                                        <td class="py-2 px-4 border-b text-center">{ &evaluation.grade }</td>
                                        <td class="py-2 px-4 border-b text-center">{ &evaluation.evaluation_date }</td>
                                        <td class="py-2 px-4 border-b text-center">
                                            <span class="status-badge">{ &evaluation.status }</span>
                                        </td>
                                    </tr>
                                })
                            }
                        }
                    }
                </tbody>
            </table>
        </div>
    
</MainLayout>}
}
