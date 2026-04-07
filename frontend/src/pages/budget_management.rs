//! 预算管理页面

use yew::prelude::*;
use crate::services::budget_management_service::BudgetManagementService;

#[function_component(BudgetManagementPage)]
pub fn budget_management_page() -> Html {
    let adjust_result = use_state(|| String::new());
    
    let on_adjust = {
        let adjust_result = adjust_result.clone();
        Callback::from(move |_| {
            let adjust_result = adjust_result.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match BudgetManagementService::adjust_budget(1, "100.0".to_string(), Some("test adjustment".to_string())).await {
                    Ok(_) => adjust_result.set("预算调整成功!".to_string()),
                    Err(e) => adjust_result.set(format!("预算调整失败: {}", e)),
                }
            });
        })
    };

    html! {
        <div class="budget-management-page">
            <div class="header">
                <h1>{"预算管理"}</h1>
            </div>
            <div class="content">
                <button onclick={on_adjust}>{"测试预算调整 (项目ID: 1, 增加100元)"}</button>
                <p>{ (*adjust_result).clone() }</p>
            </div>
        </div>
    }
}