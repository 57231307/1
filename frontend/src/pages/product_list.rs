//! 产品列表管理页面

use yew::prelude::*;

#[function_component(ProductListPage)]
pub fn product_list_page() -> Html {
    html! {
        <div class="product-list-page">
            <div class="header">
                <h1>{"产品管理"}</h1>
            </div>
            <div class="content">
                <table class="table"><thead><tr><th>{"ID"}</th><th>{"名称"}</th><th>{"操作"}</th></tr></thead><tbody><tr><td colspan="3" class="text-center">{"暂无数据"}</td></tr></tbody></table>
            </div>
        </div>
    }
}
