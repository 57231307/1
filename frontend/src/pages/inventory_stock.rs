use crate::components::main_layout::MainLayout;
use crate::components::tracked_print_button::TrackedPrintButton;
use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use std::collections::HashMap;

#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct StockResponse {
    pub id: i32,
    pub product_id: i32,
    pub warehouse_id: i32,
    pub quantity: f64,
    pub locked_quantity: f64,
    pub status: String,
    // Emulated tree view children: Rolls
    #[serde(default)]
    pub rolls: Vec<RollResponse>,
}

#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct RollResponse {
    pub roll_no: String,
    pub batch_no: String,
    pub length: f64,
    pub defect_points: f64,
}

#[function_component(InventoryStockPage)]
pub fn inventory_stock_page() -> Html {
    let stocks = use_state(|| Vec::<StockResponse>::new());
    let is_loading = use_state(|| true);
    let expanded_rows = use_state(|| HashMap::<i32, bool>::new());

    {
        let stocks = stocks.clone();
        let is_loading = is_loading.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(mut res) = ApiService::get::<Vec<StockResponse>>("/api/v1/erp/inventory-stocks").await {
                    // Inject mock rolls for demonstration of the tree view since the backend doesn't have the roll table yet
                    for (i, stock) in res.iter_mut().enumerate() {
                        if i % 2 == 0 {
                            stock.rolls = vec![
                                RollResponse { roll_no: format!("R{}-01", stock.id), batch_no: format!("B{}", stock.id), length: 120.5, defect_points: 0.0 },
                                RollResponse { roll_no: format!("R{}-02", stock.id), batch_no: format!("B{}", stock.id), length: 118.0, defect_points: 12.5 },
                            ];
                        }
                    }
                    stocks.set(res);
                }
                is_loading.set(false);
            });
            || ()
        });
    }

    let toggle_row = {
        let expanded = expanded_rows.clone();
        Callback::from(move |id: i32| {
            let mut current = (*expanded).clone();
            let is_open = current.get(&id).copied().unwrap_or(false);
            current.insert(id, !is_open);
            expanded.set(current);
        })
    };

    html! {
        <MainLayout current_page="库存查询">
            <div class="card p-4 md:p-6">
                <div class="flex flex-col md:flex-row justify-between items-start md:items-center mb-6 gap-4">
                    <h2 class="text-xl font-bold text-slate-800">{"条码级库存看板"}</h2>
                    <div class="flex gap-2 w-full md:w-auto">
                        <TrackedPrintButton document_type="InventoryStockList" document_id="ALL" class="w-full md:w-auto" />
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
                                    <th class="px-4 py-3 font-semibold w-10"></th>
                                    <th class="px-4 py-3 font-semibold">{"库存ID"}</th>
                                    <th class="px-4 py-3 font-semibold">{"产品ID"}</th>
                                    <th class="px-4 py-3 font-semibold">{"仓库ID"}</th>
                                    <th class="px-4 py-3 font-semibold text-right">{"总数量"}</th>
                                    <th class="px-4 py-3 font-semibold text-right">{"锁库量"}</th>
                                    <th class="px-4 py-3 font-semibold">{"状态"}</th>
                                </tr>
                            </thead>
                            <tbody class="divide-y divide-slate-200">
                                {for stocks.iter().map(|s| {
                                    let is_expanded = expanded_rows.get(&s.id).copied().unwrap_or(false);
                                    let toggle = toggle_row.clone();
                                    let id = s.id;
                                    let has_children = !s.rolls.is_empty();
                                    
                                    html! {
                                        <>
                                            <tr class="hover:bg-slate-50 transition-colors cursor-pointer" onclick={move |_| { if has_children { toggle.emit(id) } }}>
                                                <td class="px-4 py-3 text-slate-400">
                                                    if has_children {
                                                        <svg class={format!("w-4 h-4 transform transition-transform {}", if is_expanded { "rotate-90" } else { "" })} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"></path></svg>
                                                    }
                                                </td>
                                                <td class="px-4 py-3 font-medium text-slate-900">{s.id}</td>
                                                <td class="px-4 py-3">{s.product_id}</td>
                                                <td class="px-4 py-3">{s.warehouse_id}</td>
                                                <td class="px-4 py-3 text-right numeric-cell font-mono">{format!("{:.2}", s.quantity)}</td>
                                                <td class="px-4 py-3 text-right numeric-cell font-mono text-orange-500">{format!("{:.2}", s.locked_quantity)}</td>
                                                <td class="px-4 py-3">
                                                    <span class="status-badge bg-green-100 text-green-800">{&s.status}</span>
                                                </td>
                                            </tr>
                                            if is_expanded {
                                                <tr class="bg-slate-50/50">
                                                    <td colspan="7" class="p-0">
                                                        <div class="pl-12 pr-4 py-3 border-l-4 border-indigo-400 bg-indigo-50/30">
                                                            <div class="text-xs font-semibold text-indigo-800 mb-2">{"条码级匹号明细 (Roll Level)"}</div>
                                                            <table class="w-full text-xs text-slate-600 mb-2">
                                                                <thead>
                                                                    <tr class="border-b border-indigo-100 text-indigo-700">
                                                                        <th class="py-1 text-left">{"卷号/条码"}</th>
                                                                        <th class="py-1 text-left">{"关联缸号/批次"}</th>
                                                                        <th class="py-1 text-right">{"卷长 (米/码)"}</th>
                                                                        <th class="py-1 text-right">{"四分制扣分"}</th>
                                                                        <th class="py-1 text-right">{"操作"}</th>
                                                                    </tr>
                                                                </thead>
                                                                <tbody class="divide-y divide-indigo-50">
                                                                    {for s.rolls.iter().map(|r| html! {
                                                                        <tr>
                                                                            <td class="py-1.5 font-mono">{&r.roll_no}</td>
                                                                            <td class="py-1.5">{&r.batch_no}</td>
                                                                            <td class="py-1.5 text-right font-mono">{format!("{:.2}", r.length)}</td>
                                                                            <td class="py-1.5 text-right font-mono text-red-500">{format!("{:.1}", r.defect_points)}</td>
                                                                            <td class="py-1.5 text-right">
                                                                                <button class="text-indigo-600 hover:text-indigo-800 mr-2">{"条码打印"}</button>
                                                                                <button class="text-blue-600 hover:text-blue-800">{"分卷"}</button>
                                                                            </td>
                                                                        </tr>
                                                                    })}
                                                                </tbody>
                                                            </table>
                                                        </div>
                                                    </td>
                                                </tr>
                                            }
                                        </>
                                    }
                                })}
                                if stocks.is_empty() {
                                    <tr>
                                        <td colspan="7" class="px-4 py-8 text-center text-slate-500">{"库存为空"}</td>
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
