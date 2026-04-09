use crate::components::main_layout::MainLayout;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
struct CategoryItem {
    id: u32,
    code: String,
    name: String,
    description: String,
    is_active: bool,
}

#[function_component(ProductCategoryPage)]
pub fn product_category_page() -> Html {
    let categories = use_state(|| Vec::<CategoryItem>::new());
    let show_form = use_state(|| false);

    let form_code = use_state(String::new);
    let form_name = use_state(String::new);
    let form_desc = use_state(String::new);

    {
        let categories = categories.clone();
        use_effect_with((), move |_| {
            categories.set(vec![
                CategoryItem { id: 1, code: "KNIT".to_string(), name: "针织布".to_string(), description: "全棉汗布、卫衣布等".to_string(), is_active: true },
                CategoryItem { id: 2, code: "WOVEN".to_string(), name: "梭织布".to_string(), description: "牛津纺、府绸等".to_string(), is_active: true },
                CategoryItem { id: 3, code: "CHEM".to_string(), name: "化纤".to_string(), description: "涤纶、锦纶等".to_string(), is_active: true },
                CategoryItem { id: 4, code: "ACC".to_string(), name: "辅料".to_string(), description: "拉链、纽扣、织带等".to_string(), is_active: false },
            ]);
            || ()
        });
    }

    let on_add_click = {
        let show_form = show_form.clone();
        Callback::from(move |_| show_form.set(!*show_form))
    };

    let on_submit = {
        let categories = categories.clone();
        let show_form = show_form.clone();
        let form_code = form_code.clone();
        let form_name = form_name.clone();
        let form_desc = form_desc.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let mut list = (*categories).clone();
            let new_id = list.iter().map(|c| c.id).max().unwrap_or(0) + 1;
            list.push(CategoryItem {
                id: new_id,
                code: (*form_code).clone(),
                name: (*form_name).clone(),
                description: (*form_desc).clone(),
                is_active: true,
            });
            categories.set(list);
            show_form.set(false);
            form_code.set(String::new());
            form_name.set(String::new());
            form_desc.set(String::new());
        })
    };

    html! {
        <MainLayout current_page={"product_category"}>
            <div class="product-category-page p-4">
                <div class="header mb-4 flex justify-between items-center">
                    <h1 class="text-2xl font-bold">{"产品类别管理"}</h1>
                    <button onclick={on_add_click} class="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded">
                        {"+ 新增"}
                    </button>
                </div>
                
                if *show_form {
                    <div class="mb-4 p-4 border rounded bg-gray-50">
                        <form onsubmit={on_submit} class="flex gap-4 items-end">
                            <div class="flex-1">
                                <label class="block text-sm text-gray-700 mb-1">{"编码"}</label>
                                <input required=true type="text" class="w-full border rounded px-2 py-1" 
                                    value={(*form_code).clone()}
                                    oninput={Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        form_code.set(input.value());
                                    })} />
                            </div>
                            <div class="flex-1">
                                <label class="block text-sm text-gray-700 mb-1">{"类别名称"}</label>
                                <input required=true type="text" class="w-full border rounded px-2 py-1" 
                                    value={(*form_name).clone()}
                                    oninput={Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        form_name.set(input.value());
                                    })} />
                            </div>
                            <div class="flex-1">
                                <label class="block text-sm text-gray-700 mb-1">{"描述"}</label>
                                <input type="text" class="w-full border rounded px-2 py-1" 
                                    value={(*form_desc).clone()}
                                    oninput={Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        form_desc.set(input.value());
                                    })} />
                            </div>
                            <div>
                                <button type="submit" class="bg-green-500 hover:bg-green-600 text-white px-4 py-1 rounded">{"保存"}</button>
                            </div>
                        </form>
                    </div>
                }

                <div class="content">
                    <table class="data-table w-full">
                        <thead>
                            <tr>
                                <th class="numeric-cell text-right">{"ID"}</th>
                                <th>{"编码"}</th>
                                <th>{"类别名称"}</th>
                                <th>{"描述"}</th>
                                <th>{"状态"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {
                                if categories.is_empty() {
                                    html! {
                                        <tr>
                                            <td colspan="5" class="text-center py-4 text-gray-500">{"暂无数据"}</td>
                                        </tr>
                                    }
                                } else {
                                    html! {
                                        {for categories.iter().map(|cat| {
                                            html! {
                                                <tr key={cat.id} class="hover:bg-gray-50">
                                                    <td class="numeric-cell text-right">{cat.id}</td>
                                                    <td>{&cat.code}</td>
                                                    <td>{&cat.name}</td>
                                                    <td>{&cat.description}</td>
                                                    <td>
                                                        if cat.is_active {
                                                            <span class="status-badge bg-green-500 text-white">{"启用"}</span>
                                                        } else {
                                                            <span class="status-badge bg-red-500 text-white">{"禁用"}</span>
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
