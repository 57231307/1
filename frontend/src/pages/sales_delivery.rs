use crate::components::main_layout::MainLayout;
use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct DeliveryResponse {
    pub id: i32,
    pub delivery_no: String,
    pub order_id: i32,
    pub customer_id: i32,
    pub status: String,
}

#[function_component(SalesDeliveryPage)]
pub fn sales_delivery_page() -> Html {
    let deliveries = use_state(|| Vec::<DeliveryResponse>::new());
    let is_loading = use_state(|| true);

    {
        let deliveries = deliveries.clone();
        let is_loading = is_loading.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(res) = ApiService::get::<Vec<DeliveryResponse>>("/api/v1/erp/deliveries").await {
                    deliveries.set(res);
                }
                is_loading.set(false);
            });
            || ()
        });
    }

    let on_print = Callback::from(|_: yew::MouseEvent| {
        if let Some(win) = web_sys::window() {
            let _ = win.print();
        }
    });

    html! {
        <MainLayout current_page="销售发货">
            <div class="card p-4 md:p-6">
                <div class="flex flex-col md:flex-row justify-between items-start md:items-center mb-6 gap-4">
                    <h2 class="text-xl font-bold text-slate-800">{"销售发货单"}</h2>
                    <div class="flex gap-2 w-full md:w-auto">
                        <button class="btn-primary w-full md:w-auto">{"+ 新建发货"}</button>
                        <button onclick={on_print} class="btn-outline w-full md:w-auto text-slate-600 border-slate-300">{"🖨️ 打印"}</button>
                    </div>
                </div>

                if *is_loading {
                    <div class="skeleton skeleton-row"></div>
                    <div class="skeleton skeleton-row mt-2"></div>
                    <div class="skeleton skeleton-row mt-2"></div>
                } else {
                    <div class="table-responsive overflow-x-auto w-full pb-4 shadow-sm sm:rounded-lg">
                        <table class="data-table w-full text-left text-sm text-slate-600">
                            <thead class="bg-slate-50 text-slate-700">
                                <tr>
                                    <th class="px-4 py-3 font-semibold">{"发货单号"}</th>
                                    <th class="px-4 py-3 font-semibold">{"关联订单"}</th>
                                    <th class="px-4 py-3 font-semibold">{"客户ID"}</th>
                                    <th class="px-4 py-3 font-semibold">{"状态"}</th>
                                    <th class="px-4 py-3 font-semibold text-right">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody class="divide-y divide-slate-200">
                                {for deliveries.iter().map(|d| html! {
                                    <tr class="hover:bg-slate-50 transition-colors">
                                        <td class="px-4 py-3 font-medium text-slate-900">{&d.delivery_no}</td>
                                        <td class="px-4 py-3"><span class="text-xs text-blue-500 underline cursor-pointer">{format!("#{}", d.order_id)}</span></td>
                                        <td class="px-4 py-3">{d.customer_id}</td>
                                        <td class="px-4 py-3">
                                            <span class="status-badge bg-green-100 text-green-800">{&d.status}</span>
                                        </td>
                                        <td class="px-4 py-3 text-right">
                                            <button class="text-blue-600 hover:text-blue-800 font-medium text-sm">{"查看明细"}</button>
                                            <button class="btn-outline text-xs ml-2 text-green-600 border-green-200 hover:bg-green-50">{"触发库存变动"}</button>
                                        </td>
                                    </tr>
                                })}
                                if deliveries.is_empty() {
                                    <tr>
                                        <td colspan="5" class="px-4 py-8 text-center text-slate-500">{"暂无发货单记录"}</td>
                                    </tr>
                                }
                            </tbody>
                        </table>
                    </div>
                }
            </div>
        </MainLayout>
    }
}
