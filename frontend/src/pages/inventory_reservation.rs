use crate::components::main_layout::MainLayout;
use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct ReservationResponse {
    pub id: i32,
    pub product_id: i32,
    pub order_id: i32,
    pub quantity: f64,
    pub status: String,
}

#[function_component(InventoryReservationPage)]
pub fn inventory_reservation_page() -> Html {
    let reservations = use_state(|| Vec::<ReservationResponse>::new());
    let is_loading = use_state(|| true);

    {
        let reservations = reservations.clone();
        let is_loading = is_loading.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(res) = ApiService::get::<Vec<ReservationResponse>>("/api/v1/erp/inventory-reservations").await {
                    reservations.set(res);
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
        <MainLayout current_page="库存预留">
            <div class="card p-4 md:p-6">
                <div class="flex flex-col md:flex-row justify-between items-start md:items-center mb-6 gap-4">
                    <h2 class="text-xl font-bold text-slate-800">{"库存预留 (锁库)"}</h2>
                    <div class="flex gap-2 w-full md:w-auto">
                        <button class="btn-primary w-full md:w-auto">{"+ 手动锁库"}</button>
                        <button onclick={on_print} class="btn-outline w-full md:w-auto text-slate-600 border-slate-300">{"🖨️ 打印"}</button>
                    </div>
                </div>

                if *is_loading {
                    <div class="skeleton skeleton-row"></div>
                    <div class="skeleton skeleton-row mt-2"></div>
                } else {
                    <div class="table-responsive overflow-x-auto w-full pb-4 shadow-sm sm:rounded-lg">
                        <table class="data-table w-full text-left text-sm text-slate-600">
                            <thead class="bg-slate-50 text-slate-700">
                                <tr>
                                    <th class="px-4 py-3 font-semibold">{"记录ID"}</th>
                                    <th class="px-4 py-3 font-semibold">{"产品ID"}</th>
                                    <th class="px-4 py-3 font-semibold">{"关联订单"}</th>
                                    <th class="px-4 py-3 font-semibold text-right">{"锁定数量"}</th>
                                    <th class="px-4 py-3 font-semibold">{"状态"}</th>
                                    <th class="px-4 py-3 font-semibold text-right">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody class="divide-y divide-slate-200">
                                {for reservations.iter().map(|r| html! {
                                    <tr class="hover:bg-slate-50 transition-colors">
                                        <td class="px-4 py-3 font-medium text-slate-900">{r.id}</td>
                                        <td class="px-4 py-3">{r.product_id}</td>
                                        <td class="px-4 py-3"><span class="text-xs text-blue-500 underline cursor-pointer">{format!("#{}", r.order_id)}</span></td>
                                        <td class="px-4 py-3 text-right numeric-cell font-mono">{r.quantity}</td>
                                        <td class="px-4 py-3">
                                            <span class="status-badge bg-yellow-100 text-yellow-800">{&r.status}</span>
                                        </td>
                                        <td class="px-4 py-3 text-right">
                                            <button class="text-red-600 hover:text-red-800 font-medium text-sm">{"解除锁定"}</button>
                                        </td>
                                    </tr>
                                })}
                                if reservations.is_empty() {
                                    <tr>
                                        <td colspan="6" class="px-4 py-8 text-center text-slate-500">{"暂无库存预留记录"}</td>
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
