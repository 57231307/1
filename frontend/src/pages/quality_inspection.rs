//! 质量检验页面

use yew::prelude::*;

#[function_component(QualityInspectionPage)]
pub fn quality_inspection_page() -> Html {
    html! {
        <div class="quality-inspection-page">
            <div class="header">
                <h1>{"质量检验"}</h1>
            </div>
            <div class="content">
                <table class="table"><thead><tr><th>{"ID"}</th><th>{"名称"}</th><th>{"操作"}</th></tr></thead><tbody><tr><td colspan="3" class="text-center">{"暂无数据"}</td></tr></tbody></table>
            </div>
        </div>
    }
}
