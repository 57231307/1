// 部门管理页面

use yew::prelude::*;

#[function_component(DepartmentListPage)]
pub fn department_list_page() -> Html {
    html! {
        <div class="department-list-page">
            <div class="header">
                <h1>{"部门管理"}</h1>
            </div>
            <div class="content">
                <p>{"部门管理功能开发中..."}</p>
            </div>
        </div>
    }
}
