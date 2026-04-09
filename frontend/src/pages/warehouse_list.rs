//! 仓库管理页面

use crate::models::warehouse::Warehouse;
use crate::services::warehouse_service::WarehouseService;
use yew::prelude::*;

#[function_component(WarehouseListPage)]
pub fn warehouse_list_page() -> Html {
    let warehouses = use_state(|| Vec::<Warehouse>::new());

    {
        let warehouses = warehouses.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(res) = WarehouseService::list_warehouses().await {
                    warehouses.set(res.warehouses);
                }
            });
            || ()
        });
    }

    html! {
        <div class="p-4">
            <h1 class="text-2xl font-bold mb-4">{ "仓库管理" }</h1>
            <table class="min-w-full bg-white border border-gray-200">
                <thead>
                    <tr>
                        <th class="py-2 px-4 border-b">{ "编号" }</th>
                        <th class="py-2 px-4 border-b">{ "名称" }</th>
                        <th class="py-2 px-4 border-b">{ "状态" }</th>
                    </tr>
                </thead>
                <tbody>
                    {
                        if warehouses.is_empty() {
                            html! { <tr><td colspan="3" class="text-center py-4">{ "暂无数据" }</td></tr> }
                        } else {
                            html! {
                                for warehouses.iter().map(|warehouse| html! {
                                    <tr key={warehouse.id}>
                                        <td class="py-2 px-4 border-b text-center">{ &warehouse.code }</td>
                                        <td class="py-2 px-4 border-b text-center">{ &warehouse.name }</td>
                                        <td class="py-2 px-4 border-b text-center">{ &warehouse.status }</td>
                                    </tr>
                                })
                            }
                        }
                    }
                </tbody>
            </table>
        </div>
    }
}
