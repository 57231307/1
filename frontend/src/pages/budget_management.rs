//! 预算管理页面

use yew::prelude::*;

#[function_component(BudgetManagementPage)]
pub fn budget_management_page() -> Html {
    html! {
        <div class="budget-management-page">
            <div class="header">
                <h1>{"预算管理"}</h1>
            </div>
            <div class="content">
                <p>{"预算管理功能开发中..."}</p>
            </div>
        </div>
    }
}