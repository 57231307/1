//! 部门管理页面

use crate::components::main_layout::MainLayout;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub struct DepartmentItem {
    pub id: usize,
    pub name: String,
    pub manager: String,
    pub headcount: usize,
    pub status: String,
}

#[function_component(DepartmentListPage)]
pub fn department_list_page() -> Html {
    let departments = use_state(|| Vec::<DepartmentItem>::new());
    let show_form = use_state(|| false);
    
    let new_name = use_state(|| String::new());
    let new_manager = use_state(|| String::new());
    let new_headcount = use_state(|| String::new());

    {
        let departments = departments.clone();
        use_effect_with((), move |_| {
            let initial_data = vec![
                DepartmentItem {
                    id: 1,
                    name: "织造车间".to_string(),
                    manager: "张三".to_string(),
                    headcount: 120,
                    status: "正常".to_string(),
                },
                DepartmentItem {
                    id: 2,
                    name: "染整车间".to_string(),
                    manager: "李四".to_string(),
                    headcount: 85,
                    status: "正常".to_string(),
                },
                DepartmentItem {
                    id: 3,
                    name: "质检部".to_string(),
                    manager: "王五".to_string(),
                    headcount: 30,
                    status: "正常".to_string(),
                },
            ];
            departments.set(initial_data);
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
        let departments = departments.clone();
        let show_form = show_form.clone();
        let new_name = new_name.clone();
        let new_manager = new_manager.clone();
        let new_headcount = new_headcount.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let mut current = (*departments).clone();
            let id = current.len() + 1;
            let headcount = new_headcount.parse::<usize>().unwrap_or(0);
            
            current.push(DepartmentItem {
                id,
                name: (*new_name).clone(),
                manager: (*new_manager).clone(),
                headcount,
                status: "正常".to_string(),
            });
            
            departments.set(current);
            show_form.set(false);
            new_name.set(String::new());
            new_manager.set(String::new());
            new_headcount.set(String::new());
        })
    };

    let on_name_change = {
        let new_name = new_name.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            new_name.set(input.value());
        })
    };

    let on_manager_change = {
        let new_manager = new_manager.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            new_manager.set(input.value());
        })
    };

    let on_headcount_change = {
        let new_headcount = new_headcount.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            new_headcount.set(input.value());
        })
    };

    html! {
        <MainLayout current_page={"department_list"}>
            <div class="department-list-page p-4">
                <div class="header mb-4 flex justify-between items-center">
                    <h1 class="text-2xl font-bold">{"部门管理"}</h1>
                    <button 
                        onclick={on_add_click}
                        class="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded shadow">
                        {"+ 新增"}
                    </button>
                </div>

                if *show_form {
                    <div class="mb-4 p-4 border rounded bg-gray-50">
                        <form onsubmit={on_submit} class="flex gap-4 items-end">
                            <div>
                                <label class="block text-sm font-medium text-gray-700">{"部门名称"}</label>
                                <input type="text" value={(*new_name).clone()} onchange={on_name_change} required=true class="mt-1 block w-full rounded-md border-gray-300 shadow-sm p-2 border" />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700">{"负责人"}</label>
                                <input type="text" value={(*new_manager).clone()} onchange={on_manager_change} required=true class="mt-1 block w-full rounded-md border-gray-300 shadow-sm p-2 border" />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700">{"人数"}</label>
                                <input type="number" value={(*new_headcount).clone()} onchange={on_headcount_change} required=true class="mt-1 block w-full rounded-md border-gray-300 shadow-sm p-2 border" />
                            </div>
                            <button type="submit" class="bg-green-500 hover:bg-green-600 text-white px-4 py-2 rounded shadow">
                                {"保存"}
                            </button>
                        </form>
                    </div>
                }

                <div class="content overflow-x-auto">
                    <table class="data-table w-full border-collapse">
                        <thead>
                            <tr class="bg-gray-100">
                                <th class="py-2 px-4 border-b numeric-cell text-right">{"ID"}</th>
                                <th class="py-2 px-4 border-b text-left">{"部门名称"}</th>
                                <th class="py-2 px-4 border-b text-left">{"负责人"}</th>
                                <th class="py-2 px-4 border-b numeric-cell text-right">{"人数"}</th>
                                <th class="py-2 px-4 border-b text-center">{"状态"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {
                                if departments.is_empty() {
                                    html! {
                                        <tr><td colspan="5" class="text-center py-4">{"暂无数据"}</td></tr>
                                    }
                                } else {
                                    html! {
                                        for departments.iter().map(|dept| html! {
                                            <tr key={dept.id} class="hover:bg-gray-50">
                                                <td class="py-2 px-4 border-b numeric-cell text-right">{ dept.id }</td>
                                                <td class="py-2 px-4 border-b text-left">{ &dept.name }</td>
                                                <td class="py-2 px-4 border-b text-left">{ &dept.manager }</td>
                                                <td class="py-2 px-4 border-b numeric-cell text-right">{ dept.headcount }</td>
                                                <td class="py-2 px-4 border-b text-center">
                                                    <span class="status-badge bg-green-100 text-green-800 px-2 py-1 rounded text-xs">{ &dept.status }</span>
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
