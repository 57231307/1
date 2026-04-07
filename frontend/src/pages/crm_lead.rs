use crate::services::crm_service::{CrmLead, CrmService};
use yew::prelude::*;

#[function_component(CrmLeadPage)]
pub fn crm_lead_page() -> Html {
    let leads = use_state(|| Vec::<CrmLead>::new());

    {
        let leads = leads.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(res) = CrmService::list_leads(1, 10).await {
                    leads.set(res.data);
                }
            });
            || ()
        });
    }

    let on_create_click = Callback::from(|_| {
        // Stub for create
    });

    html! {
        <div class="p-4">
            <h1 class="text-2xl font-bold mb-4">{ "CRM 线索管理" }</h1>
            <div class="mb-4">
                <button onclick={on_create_click} class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded">
                    { "新建线索" }
                </button>
            </div>

            <table class="min-w-full bg-white border border-gray-200">
                <thead>
                    <tr>
                        <th class="py-2 px-4 border-b">{ "编号" }</th>
                        <th class="py-2 px-4 border-b">{ "名称" }</th>
                        <th class="py-2 px-4 border-b">{ "来源" }</th>
                        <th class="py-2 px-4 border-b">{ "状态" }</th>
                        <th class="py-2 px-4 border-b">{ "操作" }</th>
                    </tr>
                </thead>
                <tbody>
                    {
                        for leads.iter().map(|lead| html! {
                            <tr key={lead.id}>
                                <td class="py-2 px-4 border-b text-center">{ &lead.lead_no }</td>
                                <td class="py-2 px-4 border-b text-center">{ &lead.name }</td>
                                <td class="py-2 px-4 border-b text-center">{ &lead.source }</td>
                                <td class="py-2 px-4 border-b text-center">{ &lead.status }</td>
                                <td class="py-2 px-4 border-b text-center">
                                    <button class="text-blue-500 hover:text-blue-700">{ "查看" }</button>
                                </td>
                            </tr>
                        })
                    }
                </tbody>
            </table>
        </div>
    }
}
