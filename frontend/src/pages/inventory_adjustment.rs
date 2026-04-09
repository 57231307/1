//! 库存调整单页面

use crate::models::inventory_adjustment::AdjustmentSummary;
use crate::services::inventory_adjustment_service::InventoryAdjustmentService;
use yew::prelude::*;

/// 库存调整单页面组件
#[function_component(InventoryAdjustmentPage)]
pub fn inventory_adjustment_page() -> Html {
    let adjustments = use_state(|| Vec::<AdjustmentSummary>::new());

    {
        let adjustments = adjustments.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(res) = InventoryAdjustmentService::list_adjustments(Some(1), Some(10)).await {
                    adjustments.set(res.adjustments);
                }
            });
            || ()
        });
    }

    html! {
        <div class="inventory-adjustment-page p-4">
            <div class="header mb-4">
                <h1 class="text-2xl font-bold">{"库存调整单"}</h1>
            </div>
            <div class="content">
                <table class="min-w-full bg-white border border-gray-200">
                    <thead>
                        <tr>
                            <th class="py-2 px-4 border-b">{"ID"}</th>
                            <th class="py-2 px-4 border-b">{"调整单号"}</th>
                            <th class="py-2 px-4 border-b">{"仓库ID"}</th>
                            <th class="py-2 px-4 border-b">{"调整类型"}</th>
                            <th class="py-2 px-4 border-b">{"原因类型"}</th>
                            <th class="py-2 px-4 border-b">{"总数量"}</th>
                            <th class="py-2 px-4 border-b">{"状态"}</th>
                            <th class="py-2 px-4 border-b">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {
                            if adjustments.is_empty() {
                                html! {
                                    <tr><td colspan="8" class="text-center py-4">{"暂无数据"}</td></tr>
                                }
                            } else {
                                html! {
                                    for adjustments.iter().map(|adj| html! {
                                        <tr key={adj.id}>
                                            <td class="py-2 px-4 border-b text-center">{ adj.id }</td>
                                            <td class="py-2 px-4 border-b text-center">{ &adj.adjustment_no }</td>
                                            <td class="py-2 px-4 border-b text-center">{ adj.warehouse_id }</td>
                                            <td class="py-2 px-4 border-b text-center">{ &adj.adjustment_type }</td>
                                            <td class="py-2 px-4 border-b text-center">{ &adj.reason_type }</td>
                                            <td class="py-2 px-4 border-b text-center">{ &adj.total_quantity }</td>
                                            <td class="py-2 px-4 border-b text-center">{ &adj.status }</td>
                                            <td class="py-2 px-4 border-b text-center">
                                                <button class="text-blue-500 hover:text-blue-700">{"查看"}</button>
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
    }
}
