//! 销售分析页面

use yew::prelude::*;

#[function_component(SalesAnalysisPage)]
pub fn sales_analysis_page() -> Html {
    html! {
        <div class="sales-analysis-page">
            <div class="header">
                <h1>{"销售分析"}</h1>
            </div>
            <div class="content">
                <p>{"销售分析功能开发中..."}</p>
            </div>
        </div>
    }
}
