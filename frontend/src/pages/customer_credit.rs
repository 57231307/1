//! 客户信用管理页面

use yew::prelude::*;

/// 客户信用管理页面组件
#[function_component(CustomerCreditPage)]
pub fn customer_credit_page() -> Html {
    html! {
        <div class="customer-credit-page">
            <div class="header">
                <h1>{"客户信用管理"}</h1>
            </div>
            <div class="content">
                <p>{"客户信用管理功能开发中..."}</p>
            </div>
        </div>
    }
}