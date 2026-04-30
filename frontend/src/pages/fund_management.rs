// 资金管理页面

use yew::prelude::*;
use crate::services::fund_management_service::FundManagementService;

#[function_component(FundManagementPage)]
pub fn fund_management_page() -> Html {
    let transfer_result = use_state(|| String::new());
    
    let on_transfer = {
        let transfer_result = transfer_result.clone();
        Callback::from(move |_| {
            let transfer_result = transfer_result.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match FundManagementService::transfer_fund(1, 2, "100.0".to_string(), None, Some("test transfer".to_string())).await {
                    Ok(_) => transfer_result.set("转账成功!".to_string()),
                    Err(e) => transfer_result.set(format!("转账失败: {}", e)),
                }
            });
        })
    };

    html! {
        <div class="fund-management-page">
            <div class="header">
                <h1>{"资金管理"}</h1>
            </div>
            <div class="content">
                <button onclick={on_transfer}>{"测试资金调拨 (从账户1到账户2, 100元)"}</button>
                <p>{ (*transfer_result).clone() }</p>
            </div>
        </div>
    }
}