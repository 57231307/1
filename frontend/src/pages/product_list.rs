// 产品列表管理页面

use yew::prelude::*;

#[function_component(ProductListPage)]
pub fn product_list_page() -> Html {
    html! {
        <div class="product-list-page">
            <div class="header">
                <h1>{"产品管理"}</h1>
            </div>
            <div class="content">
                <p>{"产品管理功能开发中..."}</p>
            </div>
        </div>
    }
}
