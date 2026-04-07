//! 凭证管理页面

use yew::prelude::*;

#[function_component(VoucherPage)]
pub fn voucher_page() -> Html {
    html! {
        <div class="voucher-page">
            <div class="header">
                <h1>{"凭证管理"}</h1>
            </div>
            <div class="content">
                <table class="table"><thead><tr><th>{"ID"}</th><th>{"名称"}</th><th>{"操作"}</th></tr></thead><tbody><tr><td colspan="3" class="text-center">{"暂无数据"}</td></tr></tbody></table>
            </div>
        </div>
    }
}
