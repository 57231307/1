use gloo_dialogs;
use yew::prelude::*;
use crate::services::crm_service::{CrmService, CrmLead};

#[function_component(CrmLeadPage)]
pub fn crm_lead_page() -> Html {
    let leads = use_state(|| Vec::<CrmLead>::new());
    let viewing_item = use_state(|| None::<CrmLead>);

    
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
                                    <button class="text-blue-500 hover:text-blue-700" onclick={
                                        let viewing_item = viewing_item.clone();
                                        let item = lead.clone();
                                        Callback::from(move |_| viewing_item.set(Some(item.clone())))
                                    }>{ "查看" }</button>
                                </td>
                            </tr>
                        })
                    }
                </tbody>
            </table>

            {
                if let Some(item) = (*viewing_item).clone() {
                    let close = {
                        let viewing_item = viewing_item.clone();
                        Callback::from(move |_| viewing_item.set(None))
                    };
                    html! {
                        <div class="modal-overlay" style="position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.5); display: flex; justify-content: center; align-items: center; z-index: 1000;">
                            <div class="modal-content" style="background: white; padding: 20px; border-radius: 8px; min-width: 400px;">
                                <h2 class="text-xl font-bold mb-4">{"详情"}</h2>
                                <div class="space-y-2">
                                    <p><strong>{"ID: "}</strong>{item.id}</p>
                                    <p><strong>{"名称: "}</strong>{&item.name}</p>
                                    <p><strong>{"状态: "}</strong>{&item.status}</p>
                                </div>
                                <div class="mt-4 flex justify-end">
                                    <button class="bg-gray-300 hover:bg-gray-400 px-4 py-2 rounded" onclick={close}>{"关闭"}</button>
                                </div>
                            </div>
                        </div>
                    }
                } else {
                    html! {}
                }
            }
            </div>
    }
}
