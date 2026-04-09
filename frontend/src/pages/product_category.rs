use crate::components::main_layout::MainLayout;
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
        <MainLayout current_page={"product_category"}>
            <div class="product-category-page p-4">
                <div class="header mb-4">
                    <h1 class="text-2xl font-bold">{"产品类别管理"}</h1>
                </div>
                <div class="content">
                    <table class="data-table w-full">
                        <thead>
                            <tr>
                                <th class="py-2 px-4 border-b numeric-cell text-right">{"ID"}</th>
                                <th class="py-2 px-4 border-b text-left">{"编码"}</th>
                                <th class="py-2 px-4 border-b text-left">{"类别名称"}</th>
                                <th class="py-2 px-4 border-b numeric-cell text-right">{"层级"}</th>
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
                                                    <td class="py-2 px-4 border-b numeric-cell text-right">{cat.id}</td>
                                                    <td class="py-2 px-4 border-b text-left">{&cat.code}</td>
                                                    <td class="py-2 px-4 border-b text-left">{&cat.name}</td>
                                                    <td class="py-2 px-4 border-b numeric-cell text-right">{cat.level}</td>
                                                    <td class="py-2 px-4 border-b text-center">
                                                        if cat.is_active {
                                                            <span class="status-badge">{"启用"}</span>
                                                        } else {
                                                            <span class="status-badge">{"禁用"}</span>
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
        </MainLayout>
    }
}
