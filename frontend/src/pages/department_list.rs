//! 部门管理页面

use crate::models::department::Department;
use crate::services::department_service::DepartmentService;
use yew::prelude::*;

#[function_component(DepartmentListPage)]
pub fn department_list_page() -> Html {
    let departments = use_state(|| Vec::<Department>::new());

    {
        let departments = departments.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(res) = DepartmentService::list_departments().await {
                    departments.set(res.departments);
                }
            });
            || ()
        });
    }

    html! {
        <div class="department-list-page p-4">
            <div class="header mb-4">
                <h1 class="text-2xl font-bold">{"部门管理"}</h1>
            </div>
            <div class="content">
                <table class="min-w-full bg-white border border-gray-200">
                    <thead>
                        <tr>
                            <th class="py-2 px-4 border-b">{"ID"}</th>
                            <th class="py-2 px-4 border-b">{"代码"}</th>
                            <th class="py-2 px-4 border-b">{"名称"}</th>
                            <th class="py-2 px-4 border-b">{"负责人"}</th>
                            <th class="py-2 px-4 border-b">{"电话"}</th>
                            <th class="py-2 px-4 border-b">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {
                            if departments.is_empty() {
                                html! {
                                    <tr><td colspan="6" class="text-center py-4">{"暂无数据"}</td></tr>
                                }
                            } else {
                                html! {
                                    for departments.iter().map(|dept| html! {
                                        <tr key={dept.id}>
                                            <td class="py-2 px-4 border-b text-center">{ dept.id }</td>
                                            <td class="py-2 px-4 border-b text-center">{ &dept.code }</td>
                                            <td class="py-2 px-4 border-b text-center">{ &dept.name }</td>
                                            <td class="py-2 px-4 border-b text-center">{ dept.manager.clone().unwrap_or_default() }</td>
                                            <td class="py-2 px-4 border-b text-center">{ dept.phone.clone().unwrap_or_default() }</td>
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
