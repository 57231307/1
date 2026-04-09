use crate::services::product_category_service::{ProductCategory, ProductCategoryService};
use yew::prelude::*;

#[function_component(ProductCategoryPage)]
pub fn product_category_page() -> Html {
    let categories = use_state(|| Vec::<ProductCategory>::new());

    {
        let categories = categories.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(res) = ProductCategoryService::list().await {
                    categories.set(res);
                }
            });
            || ()
        });
    }

    html! {
        <div class="product-category-page p-4">
            <div class="header mb-4">
                <h1 class="text-2xl font-bold">{"产品类别管理"}</h1>
            </div>
            <div class="content">
                <table class="min-w-full bg-white border border-gray-200">
                    <thead>
                        <tr>
                            <th class="py-2 px-4 border-b text-left">{"ID"}</th>
                            <th class="py-2 px-4 border-b text-left">{"编码"}</th>
                            <th class="py-2 px-4 border-b text-left">{"类别名称"}</th>
                            <th class="py-2 px-4 border-b text-left">{"层级"}</th>
                            <th class="py-2 px-4 border-b text-center">{"状态"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {
                            if categories.is_empty() {
                                html! {
                                    <tr>
                                        <td colspan="5" class="text-center py-4">{"暂无数据"}</td>
                                    </tr>
                                }
                            } else {
                                html! {
                                    {for categories.iter().map(|cat| {
                                        html! {
                                            <tr key={cat.id}>
                                                <td class="py-2 px-4 border-b">{cat.id}</td>
                                                <td class="py-2 px-4 border-b">{&cat.code}</td>
                                                <td class="py-2 px-4 border-b">{&cat.name}</td>
                                                <td class="py-2 px-4 border-b">{cat.level}</td>
                                                <td class="py-2 px-4 border-b text-center">
                                                    if cat.is_active {
                                                        <span class="text-green-600">{"启用"}</span>
                                                    } else {
                                                        <span class="text-red-600">{"禁用"}</span>
                                                    }
                                                </td>
                                            </tr>
                                        }
                                    })}
                                }
                            }
                        }
                    </tbody>
                </table>
            </div>
        </div>
    }
}
