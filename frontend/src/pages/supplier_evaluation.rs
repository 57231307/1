//! 供应商评估页面

use yew::prelude::*;

#[function_component(SupplierEvaluationPage)]
pub fn supplier_evaluation_page() -> Html {
    html! {
        <div class="supplier-evaluation-page">
            <div class="header">
                <h1>{"供应商评估"}</h1>
            </div>
            <div class="content">
                <table class="table"><thead><tr><th>{"ID"}</th><th>{"名称"}</th><th>{"操作"}</th></tr></thead><tbody><tr><td colspan="3" class="text-center">{"暂无数据"}</td></tr></tbody></table>
            </div>
        </div>
    }
}
