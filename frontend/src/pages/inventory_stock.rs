//! 库存查询页面

use yew::prelude::*;

#[function_component(InventoryStockPage)]
pub fn inventory_stock_page() -> Html {
    html! {
        <div class="inventory-stock-page">
            <div class="header">
                <h1>{"库存查询"}</h1>
            </div>
            <div class="content">
                <table class="table"><thead><tr><th>{"ID"}</th><th>{"名称"}</th><th>{"操作"}</th></tr></thead><tbody><tr><td colspan="3" class="text-center">{"暂无数据"}</td></tr></tbody></table>
            </div>
        </div>
    }
}
