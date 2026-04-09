//! 销售分析页面

use crate::components::main_layout::MainLayout;
use crate::models::sales_analysis::SalesTarget;
use crate::services::sales_analysis_service::SalesAnalysisService;
use yew::prelude::*;

#[function_component(SalesAnalysisPage)]
pub fn sales_analysis_page() -> Html {
    let targets = use_state(|| Vec::<SalesTarget>::new());

    {
        let targets = targets.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(res) = SalesAnalysisService::list_targets(None, None, 1, 10).await {
                    targets.set(res);
                }
            });
            || ()
        });
    }

    html! {
        <MainLayout current_page={"sales_analysis"}>
<div class="p-4">
            <h1 class="text-2xl font-bold mb-4">{ "销售目标" }</h1>
            <table class="data-table w-full min-w-full bg-white border border-gray-200">
                <thead>
                    <tr>
                        <th class="py-2 px-4 border-b">{ "类型" }</th>
                        <th class="py-2 px-4 border-b">{ "周期" }</th>
                        <th class="py-2 px-4 border-b numeric-cell text-right">{ "目标金额" }</th>
                        <th class="py-2 px-4 border-b numeric-cell text-right">{ "实际金额" }</th>
                        <th class="py-2 px-4 border-b numeric-cell text-right">{ "完成率" }</th>
                        <th class="py-2 px-4 border-b">{ "状态" }</th>
                    </tr>
                </thead>
                <tbody>
                    {
                        if targets.is_empty() {
                            html! { <tr><td colspan="6" class="text-center py-4">{ "暂无数据" }</td></tr> }
                        } else {
                            html! {
                                for targets.iter().map(|target| html! {
                                    <tr key={target.id}>
                                        <td class="py-2 px-4 border-b text-center">{ &target.target_type }</td>
                                        <td class="py-2 px-4 border-b text-center">{ &target.period }</td>
                                        <td class="py-2 px-4 border-b numeric-cell text-right">{ &target.target_amount }</td>
                                        <td class="py-2 px-4 border-b numeric-cell text-right">{ &target.actual_amount }</td>
                                        <td class="py-2 px-4 border-b numeric-cell text-right">{ &target.completion_rate }</td>
                                        <td class="py-2 px-4 border-b text-center"><span class="status-badge">{ &target.status }</span></td>
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
