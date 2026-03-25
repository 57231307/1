//! 财务分析页面

use yew::prelude::*;

#[function_component(FinancialAnalysisPage)]
pub fn financial_analysis_page() -> Html {
    html! {
        <div class="financial-analysis-page">
            <div class="header">
                <h1>{"财务分析"}</h1>
            </div>
            <div class="content">
                <p>{"财务分析功能开发中..."}</p>
            </div>
        </div>
    }
}
