//! 采购价格管理页面

use crate::components::main_layout::MainLayout;
use crate::models::purchase_price::PurchasePrice;
use crate::services::purchase_price_service::PurchasePriceService;
use yew::prelude::*;

#[function_component(PurchasePricePage)]
pub fn purchase_price_page() -> Html {
    let prices = use_state(|| Vec::<PurchasePrice>::new());

    {
        let prices = prices.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(res) = PurchasePriceService::list(None, None, None, None, 1, 10).await {
                    prices.set(res);
                }
            });
            || ()
        });
    }

    html! {
        <MainLayout current_page={""}>
<div class="purchase-price-page p-4">
            <div class="header mb-4">
                <h1 class="text-2xl font-bold">{"采购价格管理"}</h1>
            </div>
            <div class="content">
                <table class="min-w-full bg-white border border-gray-200">
                    <thead>
                        <tr>
                            <th class="py-2 px-4 border-b">{"ID"}</th>
                            <th class="py-2 px-4 border-b">{"产品ID"}</th>
                            <th class="py-2 px-4 border-b">{"供应商ID"}</th>
                            <th class="py-2 px-4 border-b">{"价格"}</th>
                            <th class="py-2 px-4 border-b">{"币种"}</th>
                            <th class="py-2 px-4 border-b">{"生效日期"}</th>
                            <th class="py-2 px-4 border-b">{"状态"}</th>
                            <th class="py-2 px-4 border-b">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {
                            if prices.is_empty() {
                                html! {
                                    <tr><td colspan="8" class="text-center py-4">{"暂无数据"}</td></tr>
                                }
                            } else {
                                html! {
                                    for prices.iter().map(|price| html! {
                                        <tr key={price.id}>
                                            <td class="py-2 px-4 border-b text-center">{ price.id }</td>
                                            <td class="py-2 px-4 border-b text-center">{ price.product_id }</td>
                                            <td class="py-2 px-4 border-b text-center">{ price.supplier_id }</td>
                                            <td class="py-2 px-4 border-b text-center">{ &price.price }</td>
                                            <td class="py-2 px-4 border-b text-center">{ &price.currency }</td>
                                            <td class="py-2 px-4 border-b text-center">{ &price.effective_date }</td>
                                            <td class="py-2 px-4 border-b text-center">{ price.expiry_date.as_deref().unwrap_or("-") }</td>
                                            <td class="py-2 px-4 border-b text-center">
                                                <button class="text-blue-500 hover:text-blue-700 mr-2">{"查看"}</button>
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
    
</MainLayout>}
}
