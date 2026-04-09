use crate::components::main_layout::MainLayout;
use crate::services::crm_service::{CrmOpportunity, CrmService};
use yew::prelude::*;

#[function_component(CrmOpportunityPage)]
pub fn crm_opportunity_page() -> Html {
    let opps = use_state(|| Vec::<CrmOpportunity>::new());

    {
        let opps = opps.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(res) = CrmService::list_opportunities(1, 10).await {
                    opps.set(res.data);
                }
            });
            || ()
        });
    }

    let on_create_click = Callback::from(|_| {
        // Stub for create
    });

    html! {
        <MainLayout current_page={"CRM 商机管理"}>
<div class="p-4">
            <h1 class="text-2xl font-bold mb-4">{ "CRM 商机管理" }</h1>
            <div class="mb-4">
                <button onclick={on_create_click} class="bg-green-500 hover:bg-green-700 text-white font-bold py-2 px-4 rounded">
                    { "新建商机" }
                </button>
            </div>

            <table class="data-table w-full">
                <thead>
                    <tr>
                        <th class="py-2 px-4 border-b">{ "编号" }</th>
                        <th class="py-2 px-4 border-b">{ "名称" }</th>
                        <th class="py-2 px-4 border-b numeric-cell text-right">{ "金额" }</th>
                        <th class="py-2 px-4 border-b">{ "阶段" }</th>
                        <th class="py-2 px-4 border-b">{ "操作" }</th>
                    </tr>
                </thead>
                <tbody>
                    {
                        for opps.iter().map(|opp| html! {
                            <tr key={opp.id}>
                                <td class="py-2 px-4 border-b text-center">{ &opp.opportunity_no }</td>
                                <td class="py-2 px-4 border-b text-center">{ &opp.name }</td>
                                <td class="py-2 px-4 border-b numeric-cell text-right">{ opp.amount.to_string() }</td>
                                <td class="py-2 px-4 border-b text-center">
                                    <span class="status-badge">{ &opp.stage }</span>
                                </td>
                                <td class="py-2 px-4 border-b text-center">
                                    <button class="text-blue-500 hover:text-blue-700">{ "查看" }</button>
                                </td>
                            </tr>
                        })
                    }
                </tbody>
            </table>
        </div>
    
</MainLayout>}
}
