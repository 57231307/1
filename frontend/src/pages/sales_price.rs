//! 销售价格管理页面

use crate::models::sales_price::SalesPrice;
use crate::services::sales_price_service::SalesPriceService;
use yew::prelude::*;

#[function_component(SalesPricePage)]
pub fn sales_price_page() -> Html {
    let prices = use_state(|| Vec::<SalesPrice>::new());

    {
        let prices = prices.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(res) = SalesPriceService::list(None, None, None, None, None, 1, 10).await {
                    prices.set(res);
                }
            });
            || ()
        });
    }

    html! {
        <div class="p-4">
            <h1 class="text-2xl font-bold mb-4">{ "销售价格管理" }</h1>
            <table class="min-w-full bg-white border border-gray-200">
                <thead>
                    <tr>
                        <th class="py-2 px-4 border-b">{ "产品ID" }</th>
                        <th class="py-2 px-4 border-b">{ "价格类型" }</th>
                        <th class="py-2 px-4 border-b">{ "价格" }</th>
                        <th class="py-2 px-4 border-b">{ "货币" }</th>
                        <th class="py-2 px-4 border-b">{ "生效日期" }</th>
                        <th class="py-2 px-4 border-b">{ "状态" }</th>
                    </tr>
                </thead>
                <tbody>
                    {
                        if prices.is_empty() {
                            html! { <tr><td colspan="6" class="text-center py-4">{ "暂无数据" }</td></tr> }
                        } else {
                            html! {
                                for prices.iter().map(|price| html! {
                                    <tr key={price.id}>
                                        <td class="py-2 px-4 border-b text-center">{ price.product_id }</td>
                                        <td class="py-2 px-4 border-b text-center">{ &price.price_type }</td>
                                        <td class="py-2 px-4 border-b text-center">{ &price.price }</td>
                                        <td class="py-2 px-4 border-b text-center">{ &price.currency }</td>
                                        <td class="py-2 px-4 border-b text-center">{ &price.effective_date }</td>
                                        <td class="py-2 px-4 border-b text-center">{ &price.status }</td>
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
