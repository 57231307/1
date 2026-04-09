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
            <MainLayout current_page={"/purchase-prices"}>
<div class="purchase-price-page p-4">
            <div class="header mb-4">
                <h1 class="text-2xl font-bold">{"采购价格管理"}</h1>
            </div>
            <div class="content table-responsive">
                <table class="data-table w-full">
                    <thead>
                        <tr>
                            <th>{"ID"}</th>
                            <th>{"产品ID"}</th>
                            <th>{"供应商ID"}</th>
                            <th class="numeric-cell text-right">{"价格"}</th>
                            <th>{"币种"}</th>
                            <th>{"生效日期"}</th>
                            <th>{"状态"}</th>
                            <th>{"操作"}</th>
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
                                            <td>{ price.id }</td>
                                            <td>{ price.product_id }</td>
                                            <td>{ price.supplier_id }</td>
                                            <td class="numeric-cell text-right">{ &price.price }</td>
                                            <td>{ &price.currency }</td>
                                            <td>{ &price.effective_date }</td>
                                            <td><span class="status-badge">{ price.expiry_date.as_deref().unwrap_or("-") }</span></td>
                                            <td>
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
