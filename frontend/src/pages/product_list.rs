//! 产品列表管理页面

use crate::components::main_layout::MainLayout;
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
        <MainLayout current_page={"product_list"}>
            <div class="product-list-page p-4">
                <div class="header mb-4">
                    <h1 class="text-2xl font-bold">{"产品管理"}</h1>
                </div>
                <div class="content">
                    <table class="data-table w-full">
                        <thead>
                            <tr>
                                <th class="py-2 px-4 border-b numeric-cell text-right">{"ID"}</th>
                                <th class="py-2 px-4 border-b text-left">{"代码"}</th>
                                <th class="py-2 px-4 border-b text-left">{"名称"}</th>
                                <th class="py-2 px-4 border-b numeric-cell text-right">{"分类ID"}</th>
                                <th class="py-2 px-4 border-b text-center">{"单位"}</th>
                                <th class="py-2 px-4 border-b numeric-cell text-right">{"价格"}</th>
                                <th class="py-2 px-4 border-b text-center">{"操作"}</th>
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
                                                <td class="py-2 px-4 border-b numeric-cell text-right">{ product.id }</td>
                                                <td class="py-2 px-4 border-b text-left">{ &product.code }</td>
                                                <td class="py-2 px-4 border-b text-left">{ &product.name }</td>
                                                <td class="py-2 px-4 border-b numeric-cell text-right">{ product.category_id.unwrap_or_default() }</td>
                                                <td class="py-2 px-4 border-b text-center">{ &product.unit }</td>
                                                <td class="py-2 px-4 border-b numeric-cell text-right">{ product.price.clone().unwrap_or_default() }</td>
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
        </MainLayout>
    }
}
