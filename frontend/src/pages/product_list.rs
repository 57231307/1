use crate::components::main_layout::MainLayout;
use crate::services::product_service::ProductService;
use crate::models::product::{Product, CreateProductRequest};
use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[function_component(ProductListPage)]
pub fn product_list_page() -> Html {
    let products = use_state(|| Vec::<Product>::new());
    let show_form = use_state(|| false);

    let form_code = use_state(String::new);
    let form_name = use_state(String::new);
    let form_comp = use_state(String::new);
    let form_yarn_count = use_state(String::new);
    let form_density = use_state(String::new);
    let form_color_code = use_state(String::new);
    let form_price = use_state(|| String::new());

    {
        let products = products.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(res) = ProductService::list_products().await {
                    products.set(res.products);
                }
            });
            || ()
        });
    }

    let on_add_click = {
        let show_form = show_form.clone();
        Callback::from(move |_| {
            show_form.set(!*show_form);
        })
    };

    let on_submit = {
        let products = products.clone();
        let show_form = show_form.clone();
        let form_code = form_code.clone();
        let form_name = form_name.clone();
        let form_comp = form_comp.clone();
        let form_price = form_price.clone();
        
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            
            let req = CreateProductRequest {
                name: (*form_name).clone(),
                code: (*form_code).clone(),
                category_id: Some(1),
                unit: "m".to_string(),
                price: Some((*form_price).clone()),
                description: Some((*form_comp).clone()),
            };

            let products_clone = products.clone();
            let show_form_clone = show_form.clone();
            
            spawn_local(async move {
                if let Ok(_) = ProductService::create_product(req).await {
                    if let Ok(res) = ProductService::list_products().await {
                        products_clone.set(res.products);
                    }
                    show_form_clone.set(false);
                }
            });
        })
    };

    html! {
        <MainLayout current_page={"product_list"}>
            <div class="product-list-page p-4">
                <div class="header mb-4 flex justify-between items-center">
                    <h1 class="text-2xl font-bold">{"产品管理"}</h1>
                    <button onclick={on_add_click} class="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded">
                        {"+ 新增"}
                    </button>
                </div>

                if *show_form {
                    <div class="mb-4 p-4 border rounded bg-gray-50">
                        <form onsubmit={on_submit} class="grid grid-cols-4 gap-4 items-end">
                            <div class="col-span-1">
                                <label class="block text-sm text-gray-700 mb-1">{"代码"}</label>
                                <input required=true type="text" class="w-full border rounded px-2 py-1" 
                                    value={(*form_code).clone()}
                                    oninput={Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        form_code.set(input.value());
                                    })} />
                            </div>
                            <div class="col-span-1">
                                <label class="block text-sm text-gray-700 mb-1">{"名称"}</label>
                                <input required=true type="text" class="w-full border rounded px-2 py-1" 
                                    value={(*form_name).clone()}
                                    oninput={Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        form_name.set(input.value());
                                    })} />
                            </div>
                            <div class="col-span-1">
                                <label class="block text-sm text-gray-700 mb-1">{"成分"}</label>
                                <input type="text" class="w-full border rounded px-2 py-1" 
                                    value={(*form_comp).clone()}
                                    oninput={Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        form_comp.set(input.value());
                                    })} />
                            </div>
                            <div class="col-span-1">
                                <label class="block text-sm text-gray-700 mb-1">{"纱支"}</label>
                                <input type="text" class="w-full border rounded px-2 py-1" 
                                    value={(*form_yarn_count).clone()}
                                    oninput={Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        form_yarn_count.set(input.value());
                                    })} />
                            </div>
                            <div class="col-span-1">
                                <label class="block text-sm text-gray-700 mb-1">{"经纬密"}</label>
                                <input type="text" class="w-full border rounded px-2 py-1" 
                                    value={(*form_density).clone()}
                                    oninput={Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        form_density.set(input.value());
                                    })} />
                            </div>
                            <div class="col-span-1">
                                <label class="block text-sm text-gray-700 mb-1">{"色号"}</label>
                                <input type="text" class="w-full border rounded px-2 py-1" 
                                    value={(*form_color_code).clone()}
                                    oninput={Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        form_color_code.set(input.value());
                                    })} />
                            </div>
                            <div class="col-span-1">
                                <label class="block text-sm text-gray-700 mb-1">{"价格 (¥)"}</label>
                                <input required=true type="number" step="0.01" class="w-full border rounded px-2 py-1" 
                                    value={(*form_price).clone()}
                                    oninput={Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        form_price.set(input.value());
                                    })} />
                            </div>
                            <div class="col-span-1">
                                <button type="submit" class="bg-green-500 hover:bg-green-600 text-white px-4 py-1 rounded w-full">{"保存"}</button>
                            </div>
                        </form>
                    </div>
                }

                <div class="content">
                    <table class="data-table w-full">
                        <thead>
                            <tr>
                                <th class="numeric-cell text-right">{"ID"}</th>
                                <th>{"代码"}</th>
                                <th>{"名称"}</th>
                                <th>{"成分"}</th>
                                <th>{"纱支"}</th>
                                <th>{"经纬密"}</th>
                                <th>{"色号"}</th>
                                <th class="numeric-cell text-right">{"价格"}</th>
                                <th>{"状态"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {
                                if products.is_empty() {
                                    html! {
                                        <tr><td colspan="9" class="text-center py-4 text-gray-500">{"暂无数据"}</td></tr>
                                    }
                                } else {
                                    html! {
                                        for products.iter().map(|product| {
                                            html! {
                                                <tr key={product.id} class="hover:bg-gray-50">
                                                    <td class="numeric-cell text-right">{product.id}</td>
                                                    <td>{&product.code}</td>
                                                    <td>{&product.name}</td>
                                                    <td>{product.description.as_deref().unwrap_or("-")}</td>
                                                    <td>{"-"}</td>
                                                    <td>{"-"}</td>
                                                    <td>{"-"}</td>
                                                    <td class="numeric-cell text-right">{product.price.as_deref().unwrap_or("0.00")}</td>
                                                    <td><span class="status-badge bg-green-100 text-green-800">{"在售"}</span></td>
                                                </tr>
                                            }
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
