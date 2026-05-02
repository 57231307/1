use yew::prelude::*;
use crate::services::sales_analysis_service::SalesAnalysisService;
use crate::models::sales_analysis::SalesTrendAnalysis;

#[function_component(SalesAnalysisPage)]
pub fn sales_analysis_page() -> Html {
    let trend = use_state(|| None::<SalesTrendAnalysis>);
    let loading = use_state(|| false);
    let error = use_state(|| String::new());
    
    {
        let trend = trend.clone();
        let loading = loading.clone();
        let error = error.clone();
        
        use_effect_with((), move |_| {
            loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                // Default to last 30 days or current month
                match SalesAnalysisService::get_trend_analysis("MONTH", "2026-04-01", "2026-04-30", None, None).await {
                    Ok(data) => {
                        trend.set(Some(data));
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(format!("加载销售分析数据失败: {}", e));
                        loading.set(false);
                    }
                }
            });
            || ()
        });
    }

    html! {
        <div class="sales-analysis-page">
            <div class="header" style="margin-bottom: 20px;">
                <h1>{"销售分析看板"}</h1>
                <p style="color: #666;">{"总览与趋势分析"}</p>
            </div>
            
            if *loading {
                <div class="loading">{"加载数据中..."}</div>
            } else if !(*error).is_empty() {
                <div class="error" style="color: red;">{ (*error).clone() }</div>
            } else if let Some(data) = (*trend).clone() {
                <div class="dashboard-grid" style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 20px; margin-bottom: 20px;">
                    <div class="card" style="padding: 20px; border: 1px solid #ddd; border-radius: 8px; background: #fff;">
                        <h3>{"总销售额"}</h3>
                        <div style="font-size: 24px; font-weight: bold; color: #2c3e50;">
                            {format!("¥{}", data.total_sales_amount)}
                        </div>
                        <div style="color: #27ae60; margin-top: 10px;">
                            {format!("增长率: {} {}", data.growth_rate, if data.trend_direction == "UP" { "↑" } else { "↓" })}
                        </div>
                    </div>
                    
                    <div class="card" style="padding: 20px; border: 1px solid #ddd; border-radius: 8px; background: #fff;">
                        <h3>{"总销量"}</h3>
                        <div style="font-size: 24px; font-weight: bold; color: #2c3e50;">
                            {format!("{}", data.total_sales_quantity)}
                        </div>
                        <div style="color: #7f8c8d; margin-top: 10px;">
                            {format!("日均销量: {}", data.average_daily_sales)}
                        </div>
                    </div>

                    <div class="card" style="padding: 20px; border: 1px solid #ddd; border-radius: 8px; background: #fff;">
                        <h3>{"分析周期"}</h3>
                        <div style="font-size: 16px; color: #2c3e50;">
                            {format!("{} 至 {}", data.start_date, data.end_date)}
                        </div>
                        <div style="color: #7f8c8d; margin-top: 10px;">
                            {format!("周期类型: {}", data.period)}
                        </div>
                    </div>
                </div>
                
                <div class="charts-section" style="padding: 20px; border: 1px solid #ddd; border-radius: 8px; background: #fff;">
                    <h3>{"近期销售极值记录"}</h3>
                    <ul style="list-style: none; padding: 0;">
                        <li style="padding: 10px 0; border-bottom: 1px solid #eee;">
                            {"最高峰日期: "} 
                            <strong style="color: #e74c3c;">{data.peak_date.unwrap_or_else(|| "暂无数据".to_string())}</strong>
                        </li>
                        <li style="padding: 10px 0;">
                            {"最低谷日期: "} 
                            <strong style="color: #3498db;">{data.lowest_date.unwrap_or_else(|| "暂无数据".to_string())}</strong>
                        </li>
                    </ul>
                </div>
            }
        </div>
    }
}
