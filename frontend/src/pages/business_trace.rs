//! 业务追溯页面

use yew::prelude::*;

/// 业务追溯页面组件
#[function_component(BusinessTracePage)]
pub fn business_trace_page() -> Html {
    html! {
        <div class="business-trace-page">
            <div class="header">
                <h1>{"业务追溯"}</h1>
            </div>
            <div class="content">
                <p>{"业务追溯功能开发中..."}</p>
            </div>
        </div>
    }
}