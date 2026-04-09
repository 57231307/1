use crate::components::main_layout::MainLayout;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
struct ProductItem {
    id: u32,
    code: String,
    name: String,
    composition: String,
    price: f64,
    status: String,
}

#[function_component(ProductListPage)]
pub fn product_list_page() -> Html {
    let products = use_state(|| Vec::<ProductItem>::new());
    let show_form = use_state(|| false);

    let form_code = use_state(String::new);
    let form_name = use_state(String::new);
    let form_comp = use_state(String::new);
    let form_price = use_state(|| String::new());

    {
        let products = products.clone();
        use_effect_with((), move |_| {
            products.set(vec![
                ProductItem { id: 101, code: "SJ-100C".to_string(), name: "全棉汗布".to_string(), composition: "100% 棉".to_string(), price: 35.50, status: "在售".to_string() },
                ProductItem { id: 102, code: "PK-6535".to_string(), name: "CVC珠地网眼".to_string(), composition: "60%棉 40%聚酯纤维".to_string(), price: 28.00, status: "在售".to_string() },
                ProductItem { id: 103, code: "RB-SP".to_string(), name: "氨纶罗纹".to_string(), composition: "95%棉 5%氨纶".to_string(), price: 42.00, status: "缺货".to_string() },
                ProductItem { id: 104, code: "FL-PF".to_string(), name: "摇粒绒".to_string(), composition: "100% 聚酯纤维".to_string(), price: 18.50, status: "下架".to_string() },
            ]);
            || ()
        });
    }

    let on_add_click = {
        let show_form = show_form.clone();
        Callback::from(move |_| show_form.set(!*show_form))
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
            let mut list = (*products).clone();
            let new_id = list.iter().map(|p| p.id).max().unwrap_or(0) + 1;
            let price = form_price.parse::<f64>().unwrap_or(0.0);
            list.push(ProductItem {
                id: new_id,
                code: (*form_code).clone(),
                name: (*form_name).clone(),
                composition: (*form_comp).clone(),
                price,
                status: "在售".to_string(),
            });
            products.set(list);
            show_form.set(false);
            form_code.set(String::new());
            form_name.set(String::new());
            form_comp.set(String::new());
            form_price.set(String::new());
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
                        <form onsubmit={on_submit} class="flex gap-4 items-end">
                            <div class="flex-1">
                                <label class="block text-sm text-gray-700 mb-1">{"代码"}</label>
                                <input required=true type="text" class="w-full border rounded px-2 py-1" 
                                    value={(*form_code).clone()}
                                    oninput={Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        form_code.set(input.value());
                                    })} />
                            </div>
                            <div class="flex-1">
                                <label class="block text-sm text-gray-700 mb-1">{"名称"}</label>
                                <input required=true type="text" class="w-full border rounded px-2 py-1" 
                                    value={(*form_name).clone()}
                                    oninput={Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        form_name.set(input.value());
                                    })} />
                            </div>
                            <div class="flex-1">
                                <label class="block text-sm text-gray-700 mb-1">{"成分"}</label>
                                <input type="text" class="w-full border rounded px-2 py-1" 
                                    value={(*form_comp).clone()}
                                    oninput={Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        form_comp.set(input.value());
                                    })} />
                            </div>
                            <div class="flex-1">
                                <label class="block text-sm text-gray-700 mb-1">{"价格 (¥)"}</label>
                                <input required=true type="number" step="0.01" class="w-full border rounded px-2 py-1" 
                                    value={(*form_price).clone()}
                                    oninput={Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        form_price.set(input.value());
                                    })} />
                            </div>
                            <div>
                                <button type="submit" class="bg-green-500 hover:bg-green-600 text-white px-4 py-1 rounded">{"保存"}</button>
                            </div>
                        </form>
                    </div>
                }

                <div class="content">
                    <table class="data-table w-full border-collapse">
                        <thead>
                            <tr class="bg-gray-100">
                                <th class="py-2 px-4 border-b text-right">{"ID"}</th>
                                <th class="py-2 px-4 border-b text-left">{"代码"}</th>
                                <th class="py-2 px-4 border-b text-left">{"名称"}</th>
                                <th class="py-2 px-4 border-b text-left">{"成分"}</th>
                                <th class="py-2 px-4 border-b text-right">{"价格"}</th>
                                <th class="py-2 px-4 border-b text-center">{"状态"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {
                                if products.is_empty() {
                                    html! {
                                        <tr><td colspan="6" class="text-center py-4 text-gray-500">{"暂无数据"}</td></tr>
                                    }
                                } else {
                                    html! {
                                        for products.iter().map(|product| {
                                            let badge_class = match product.status.as_str() {
                                                "在售" => "bg-green-500",
                                                "缺货" => "bg-yellow-500",
                                                _ => "bg-gray-400",
                                            };
                                            html! {
                                                <tr key={product.id} class="hover:bg-gray-50">
                                                    <td class="py-2 px-4 border-b text-right">{ product.id }</td>
                                                    <td class="py-2 px-4 border-b text-left">{ &product.code }</td>
                                                    <td class="py-2 px-4 border-b text-left">{ &product.name }</td>
                                                    <td class="py-2 px-4 border-b text-left">{ &product.composition }</td>
                                                    <td class="py-2 px-4 border-b text-right">{ format!("¥{:.2}", product.price) }</td>
                                                    <td class="py-2 px-4 border-b text-center">
                                                        <span class={format!("inline-block px-2 py-1 text-xs text-white rounded-full {}", badge_class)}>
                                                            { &product.status }
                                                        </span>
                                                    </td>
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
