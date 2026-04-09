//! 库存查询页面

use crate::models::inventory::StockResponse;
use crate::services::inventory_service::InventoryService;
use yew::prelude::*;

#[function_component(InventoryStockPage)]
pub fn inventory_stock_page() -> Html {
    let stocks = use_state(|| Vec::<StockResponse>::new());

    {
        let stocks = stocks.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(res) = InventoryService::list_stock(1, 10).await {
                    stocks.set(res.stock);
                }
            });
            || ()
        });
    }

    html! {
        <div class="inventory-stock-page p-4">
            <div class="header mb-4">
                <h1 class="text-2xl font-bold">{"库存查询"}</h1>
            </div>
            <div class="content">
                <table class="min-w-full bg-white border border-gray-200">
                    <thead>
                        <tr>
                            <th class="py-2 px-4 border-b">{"ID"}</th>
                            <th class="py-2 px-4 border-b">{"仓库ID"}</th>
                            <th class="py-2 px-4 border-b">{"产品ID"}</th>
                            <th class="py-2 px-4 border-b">{"现有库存"}</th>
                            <th class="py-2 px-4 border-b">{"可用库存"}</th>
                            <th class="py-2 px-4 border-b">{"保留库存"}</th>
                            <th class="py-2 px-4 border-b">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {
                            if stocks.is_empty() {
                                html! {
                                    <tr><td colspan="7" class="text-center py-4">{"暂无数据"}</td></tr>
                                }
                            } else {
                                html! {
                                    for stocks.iter().map(|stock| html! {
                                        <tr key={stock.id}>
                                            <td class="py-2 px-4 border-b text-center">{ stock.id }</td>
                                            <td class="py-2 px-4 border-b text-center">{ stock.warehouse_id }</td>
                                            <td class="py-2 px-4 border-b text-center">{ stock.product_id }</td>
                                            <td class="py-2 px-4 border-b text-center">{ &stock.quantity_on_hand }</td>
                                            <td class="py-2 px-4 border-b text-center">{ &stock.quantity_available }</td>
                                            <td class="py-2 px-4 border-b text-center">{ &stock.quantity_reserved }</td>
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
