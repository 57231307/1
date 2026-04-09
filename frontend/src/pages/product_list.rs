//! 产品列表管理页面

use crate::models::product::Product;
use crate::services::product_service::ProductService;
use yew::prelude::*;

#[function_component(ProductListPage)]
pub fn product_list_page() -> Html {
    let products = use_state(|| Vec::<Product>::new());

    {
        let products = products.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(res) = ProductService::list_products().await {
                    products.set(res.products);
                }
            });
            || ()
        });
    }

    html! {
        <div class="product-list-page p-4">
            <div class="header mb-4">
                <h1 class="text-2xl font-bold">{"产品管理"}</h1>
            </div>
            <div class="content">
                <table class="min-w-full bg-white border border-gray-200">
                    <thead>
                        <tr>
                            <th class="py-2 px-4 border-b">{"ID"}</th>
                            <th class="py-2 px-4 border-b">{"代码"}</th>
                            <th class="py-2 px-4 border-b">{"名称"}</th>
                            <th class="py-2 px-4 border-b">{"分类ID"}</th>
                            <th class="py-2 px-4 border-b">{"单位"}</th>
                            <th class="py-2 px-4 border-b">{"价格"}</th>
                            <th class="py-2 px-4 border-b">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {
                            if products.is_empty() {
                                html! {
                                    <tr><td colspan="7" class="text-center py-4">{"暂无数据"}</td></tr>
                                }
                            } else {
                                html! {
                                    for products.iter().map(|product| html! {
                                        <tr key={product.id}>
                                            <td class="py-2 px-4 border-b text-center">{ product.id }</td>
                                            <td class="py-2 px-4 border-b text-center">{ &product.code }</td>
                                            <td class="py-2 px-4 border-b text-center">{ &product.name }</td>
                                            <td class="py-2 px-4 border-b text-center">{ product.category_id.unwrap_or_default() }</td>
                                            <td class="py-2 px-4 border-b text-center">{ &product.unit }</td>
                                            <td class="py-2 px-4 border-b text-center">{ product.price.clone().unwrap_or_default() }</td>
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
