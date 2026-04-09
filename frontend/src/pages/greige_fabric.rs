use crate::components::main_layout::MainLayout;
use crate::models::greige_fabric::GreigeFabric;
use crate::services::greige_fabric::GreigeFabricService;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use web_sys::window;

#[function_component(GreigeFabricPage)]
pub fn greige_fabric_page() -> Html {
    let fabrics = use_state(Vec::<GreigeFabric>::new);
    let on_print = Callback::from(|_: yew::MouseEvent| {
        if let Some(win) = window() {
            let _ = win.print();
        }
    });

    let loading = use_state(|| true);
    let show_form = use_state(|| false);

    // 表单状态
    let form_no = use_state(String::new);
    let form_name = use_state(String::new);
    let form_type = use_state(String::new);
    let form_width = use_state(String::new);
    let form_weight = use_state(String::new);

    let refresh_trigger = use_state(|| 0);

    // 加载数据
    {
        let fabrics = fabrics.clone();
        let loading = loading.clone();
        use_effect_with(refresh_trigger.clone(), move |_| {
            spawn_local(async move {
                if let Ok(res) = GreigeFabricService::get_list().await {
                    fabrics.set(res);
                }
                loading.set(false);
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

    // 换算比计算公式：1000 / (门幅(m) * 克重)
    let calculate_ratio = |w_cm: &str, w_kg: &str| -> Option<f64> {
        let width: f64 = w_cm.parse().ok()?;
        let weight: f64 = w_kg.parse().ok()?;
        if width <= 0.0 || weight <= 0.0 {
            return None;
        }
        let width_m = width / 100.0;
        Some(1000.0 / (width_m * weight))
    };

    let on_submit = {
        let form_no = form_no.clone();
        let form_name = form_name.clone();
        let form_type = form_type.clone();
        let form_width = form_width.clone();
        let form_weight = form_weight.clone();
        let show_form = show_form.clone();
        let refresh_trigger = refresh_trigger.clone();
        let loading = loading.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            
            let req = crate::models::greige_fabric::GreigeFabric {
                id: 0,
                code: (*form_no).clone(),
                name: (*form_name).clone(),
                width_cm: (*form_width).clone().parse::<f64>().unwrap_or(0.0),
                weight_gsm: (*form_weight).clone().parse::<f64>().unwrap_or(0.0),
                composition: (*form_type).clone(),
                meters_per_kg: 0.0,
            };

            let show_form = show_form.clone();
            let refresh_trigger = refresh_trigger.clone();
            let loading = loading.clone();

            let form_no = form_no.clone();
            let form_name = form_name.clone();
            let form_type = form_type.clone();
            let form_width = form_width.clone();
            let form_weight = form_weight.clone();

            spawn_local(async move {
                loading.set(true);
                if let Ok(_) = GreigeFabricService::create(&req).await {
                    show_form.set(false);
                    form_no.set(String::new());
                    form_name.set(String::new());
                    form_type.set(String::new());
                    form_width.set(String::new());
                    form_weight.set(String::new());
                    refresh_trigger.set(*refresh_trigger + 1);
                } else {
                    loading.set(false);
                }
            });
        })
    };

    let current_ratio = calculate_ratio(&form_width, &form_weight);

    let on_input_no = { let form_no = form_no.clone(); Callback::from(move |e: InputEvent| { if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() { form_no.set(input.value()); } }) };
    let on_input_name = { let form_name = form_name.clone(); Callback::from(move |e: InputEvent| { if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() { form_name.set(input.value()); } }) };
    let on_input_type = { let form_type = form_type.clone(); Callback::from(move |e: InputEvent| { if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() { form_type.set(input.value()); } }) };
    let on_input_width = { let form_width = form_width.clone(); Callback::from(move |e: InputEvent| { if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() { form_width.set(input.value()); } }) };
    let on_input_weight = { let form_weight = form_weight.clone(); Callback::from(move |e: InputEvent| { if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() { form_weight.set(input.value()); } }) };

    html! {
        <MainLayout current_page={"坯布管理"}>
            <div class="p-4">
                <div class="page-header flex justify-between items-center mb-4">
                    <h2 class="text-xl font-bold">{"坯布管理"}</h2>
                    <button class="btn-primary px-4 py-2 bg-blue-600 text-white rounded shadow" onclick={on_add_click}>
                        {if *show_form { "取消新增" } else { "+ 新增坯布" }}
                    </button>
                </div>

                if *show_form {
                    <div class="bg-white p-4 shadow-sm rounded border mb-6">
                        <h3 class="text-lg font-bold mb-4 border-b pb-2">{"新增坯布录入"}</h3>
                        <form onsubmit={on_submit} class="grid grid-cols-1 md:grid-cols-6 gap-4 items-end">
                            <div>
                                <label class="block text-sm mb-1 font-medium text-gray-700">{"编号"}</label>
                                <input type="text" class="form-input w-full border rounded px-3 py-2" value={(*form_no).clone()} oninput={on_input_no} required=true placeholder="输入编号" />
                            </div>
                            <div>
                                <label class="block text-sm mb-1 font-medium text-gray-700">{"名称"}</label>
                                <input type="text" class="form-input w-full border rounded px-3 py-2" value={(*form_name).clone()} oninput={on_input_name} required=true placeholder="输入名称" />
                            </div>
                            <div>
                                <label class="block text-sm mb-1 font-medium text-gray-700">{"成分"}</label>
                                <input type="text" class="form-input w-full border rounded px-3 py-2" value={(*form_type).clone()} oninput={on_input_type} required=true placeholder="输入成分" />
                            </div>
                            <div>
                                <label class="block text-sm mb-1 font-medium text-gray-700">{"门幅 (cm)"}</label>
                                <input type="number" step="0.1" class="form-input w-full border rounded px-3 py-2" value={(*form_width).clone()} oninput={on_input_width} required=true placeholder="如: 150" />
                            </div>
                            <div>
                                <label class="block text-sm mb-1 font-medium text-gray-700">{"克重 (g/m²)"}</label>
                                <input type="number" step="0.1" class="form-input w-full border rounded px-3 py-2" value={(*form_weight).clone()} oninput={on_input_weight} required=true placeholder="如: 200" />
                            </div>
                            <div>
                                <label class="block text-sm mb-1 font-medium text-gray-700">{"米/公斤 换算比"}</label>
                                <div class="w-full bg-gray-50 font-bold text-blue-600 flex items-center px-3 border rounded h-[42px]">
                                    {if let Some(r) = current_ratio { format!("{:.4}", r) } else { "-".to_string() }}
                                </div>
                            </div>
                            <div class="col-span-1 md:col-span-6 flex justify-end mt-2">
                                <button type="submit" class="btn-primary bg-blue-600 hover:bg-blue-700 text-white px-6 py-2 rounded shadow transition-colors">
                                    {"保存坯布"}
                                </button>
                            </div>
                        </form>
                    </div>
                }

                <div class="table-responsive bg-white shadow-sm rounded border">
                    <div class="overflow-x-auto w-full pb-4">
<table class="data-table w-full border-collapse">
                        <thead>
                            <tr class="bg-gray-50 border-b">
                                <th class="p-3 text-left font-semibold text-gray-700">{"编号"}</th>
                                <th class="p-3 text-left font-semibold text-gray-700">{"名称"}</th>
                                <th class="p-3 text-left font-semibold text-gray-700">{"成分"}</th>
                                <th class="p-3 text-right font-semibold text-gray-700">{"门幅 (cm)"}</th>
                                <th class="p-3 text-right font-semibold text-gray-700">{"克重 (g/m²)"}</th>
                                <th class="p-3 text-right font-semibold text-gray-700">{"米/公斤换算比"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {
                                if *loading {
                                    html! { <tr><td colspan="6" class="text-center py-8 text-gray-500">{"加载中..."}</td></tr> }
                                } else if fabrics.is_empty() {
                                    html! { <tr><td colspan="6" class="text-center py-8 text-gray-500">{"暂无数据"}</td></tr> }
                                } else {
                                    html! {
                                        {for fabrics.iter().map(|fabric| {
                                            let w_m = fabric.width_cm / 100.0;
                                            let calculated_ratio = if w_m > 0.0 && fabric.weight_gsm > 0.0 {
                                                1000.0 / (w_m * fabric.weight_gsm)
                                            } else {
                                                0.0
                                            };
                                            let display_ratio = if fabric.meters_per_kg > 0.0 { fabric.meters_per_kg } else { calculated_ratio };
                                            
                                            html! {
                                                <tr class="border-b hover:bg-gray-50 transition-colors">
                                                    <td class="p-3">{&fabric.code}</td>
                                                    <td class="p-3">{&fabric.name}</td>
                                                    <td class="p-3">{&fabric.composition}</td>
                                                    <td class="p-3 numeric-cell text-right">
                                                        {format!("{:.1}", fabric.width_cm)}
                                                    </td>
                                                    <td class="p-3 numeric-cell text-right">
                                                        {format!("{:.1}", fabric.weight_gsm)}
                                                    </td>
                                                    <td class="p-3 numeric-cell text-right font-bold text-blue-600">
                                                        {format!("{:.4}", display_ratio)}
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
            </div>
        </MainLayout>
    }
}
