//! 仓库管理页面

use crate::components::main_layout::MainLayout;
use yew::prelude::*;
use web_sys::window;

#[derive(Clone, PartialEq)]
pub struct WarehouseItem {
    pub id: usize,
    pub name: String,
    pub warehouse_type: String,
    pub capacity: usize,
    pub status: String,
}

#[function_component(WarehouseListPage)]
pub fn warehouse_list_page() -> Html {
    let warehouses = use_state(|| Vec::<WarehouseItem>::new());
    let on_print = Callback::from(|_: yew::MouseEvent| {
        if let Some(win) = window() {
            let _ = win.print();
        }
    });

    let show_form = use_state(|| false);
    
    let new_name = use_state(|| String::new());
    let new_type = use_state(|| String::new());
    let new_capacity = use_state(|| String::new());

    {
        let warehouses = warehouses.clone();
        use_effect_with((), move |_| {
            let initial_data = vec![
                WarehouseItem {
                    id: 1,
                    name: "一号坯布库".to_string(),
                    warehouse_type: "坯布".to_string(),
                    capacity: 50000,
                    status: "正常".to_string(),
                },
                WarehouseItem {
                    id: 2,
                    name: "二号成品库".to_string(),
                    warehouse_type: "成品".to_string(),
                    capacity: 30000,
                    status: "爆满".to_string(),
                },
                WarehouseItem {
                    id: 3,
                    name: "染化料仓".to_string(),
                    warehouse_type: "原料".to_string(),
                    capacity: 10000,
                    status: "正常".to_string(),
                },
            ];
            warehouses.set(initial_data);
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
        let warehouses = warehouses.clone();
        let show_form = show_form.clone();
        let new_name = new_name.clone();
        let new_type = new_type.clone();
        let new_capacity = new_capacity.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let mut current = (*warehouses).clone();
            let id = current.len() + 1;
            let capacity = new_capacity.parse::<usize>().unwrap_or(0);
            
            current.push(WarehouseItem {
                id,
                name: (*new_name).clone(),
                warehouse_type: (*new_type).clone(),
                capacity,
                status: "正常".to_string(),
            });
            
            warehouses.set(current);
            show_form.set(false);
            new_name.set(String::new());
            new_type.set(String::new());
            new_capacity.set(String::new());
        })
    };

    let on_name_change = {
        let new_name = new_name.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            new_name.set(input.value());
        })
    };

    let on_type_change = {
        let new_type = new_type.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            new_type.set(input.value());
        })
    };

    let on_capacity_change = {
        let new_capacity = new_capacity.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            new_capacity.set(input.value());
        })
    };

    html! {
        <MainLayout current_page={"warehouse_list"}>
            <div class="warehouse-list-page p-4">
                <div class="header mb-4 flex justify-between items-center">
                    <h1 class="text-2xl font-bold">{"仓库管理"}</h1>
                    <button 
                        onclick={on_add_click}
                        class="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded shadow">
                        {"+ 新增"}
                    </button>
                                <button onclick={on_print.clone()} class="btn-outline ml-2 text-slate-600 border-slate-300">{"🖨️ 打印"}</button>
                </div>

                if *show_form {
                    <div class="mb-4 p-4 border rounded bg-gray-50">
                        <form onsubmit={on_submit} class="flex gap-4 items-end">
                            <div>
                                <label class="block text-sm font-medium text-gray-700">{"仓库名称"}</label>
                                <input type="text" value={(*new_name).clone()} onchange={on_name_change} required=true class="mt-1 block w-full rounded-md border-gray-300 shadow-sm p-2 border" />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700">{"仓库类型"}</label>
                                <input type="text" value={(*new_type).clone()} onchange={on_type_change} required=true class="mt-1 block w-full rounded-md border-gray-300 shadow-sm p-2 border" />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700">{"容量 (米/公斤)"}</label>
                                <input type="number" value={(*new_capacity).clone()} onchange={on_capacity_change} required=true class="mt-1 block w-full rounded-md border-gray-300 shadow-sm p-2 border" />
                            </div>
                            <button type="submit" class="bg-green-500 hover:bg-green-600 text-white px-4 py-2 rounded shadow">
                                {"保存"}
                            </button>
                        </form>
                    </div>
                }

                <div class="content overflow-x-auto">
                    <div class="overflow-x-auto w-full pb-4">
<table class="data-table w-full border-collapse">
                        <thead>
                            <tr class="bg-gray-100">
                                <th class="py-2 px-4 border-b numeric-cell text-right">{"ID"}</th>
                                <th class="py-2 px-4 border-b text-left">{"仓库名称"}</th>
                                <th class="py-2 px-4 border-b text-left">{"仓库类型"}</th>
                                <th class="py-2 px-4 border-b numeric-cell text-right">{"容量"}</th>
                                <th class="py-2 px-4 border-b text-center">{"状态"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {
                                if warehouses.is_empty() {
                                    html! {
                                        <tr><td colspan="5" class="text-center py-4">{"暂无数据"}</td></tr>
                                    }
                                } else {
                                    html! {
                                        for warehouses.iter().map(|warehouse| html! {
                                            <tr key={warehouse.id} class="hover:bg-gray-50">
                                                <td class="py-2 px-4 border-b numeric-cell text-right">{ warehouse.id }</td>
                                                <td class="py-2 px-4 border-b text-left">{ &warehouse.name }</td>
                                                <td class="py-2 px-4 border-b text-left">{ &warehouse.warehouse_type }</td>
                                                <td class="py-2 px-4 border-b numeric-cell text-right">{ warehouse.capacity }</td>
                                                <td class="py-2 px-4 border-b text-center">
                                                    <span class={
                                                        if warehouse.status == "爆满" {
                                                            "status-badge bg-red-100 text-red-800 px-2 py-1 rounded text-xs"
                                                        } else {
                                                            "status-badge bg-green-100 text-green-800 px-2 py-1 rounded text-xs"
                                                        }
                                                    }>{ &warehouse.status }</span>
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
            </div>
        </MainLayout>
    }
}
