//! 库存调整单页面

use yew::prelude::*;

/// 库存调整单页面组件
#[function_component(InventoryAdjustmentPage)]
pub fn inventory_adjustment_page() -> Html {
    html! {
        <div class="inventory-adjustment-page">
            <div class="header">
                <h1>{"库存调整单"}</h1>
            </div>
            <div class="content">
                <table class="table"><thead><tr><th>{"ID"}</th><th>{"名称"}</th><th>{"操作"}</th></tr></thead><tbody><tr><td colspan="3" class="text-center">{"暂无数据"}</td></tr></tbody></table>
            </div>
        </div>
    }
}
