use crate::utils::permissions;
use yew::prelude::*;
use crate::services::fixed_asset_service::FixedAssetService;
use crate::services::crud_service::CrudService;
use crate::models::fixed_asset::{AssetQueryParams};

#[function_component(FixedAssetPage)]
pub fn fixed_asset_page() -> Html {
    let assets = use_state(Vec::new);
    let total = use_state(|| 0u64);
    let loading = use_state(|| false);
    let error = use_state(|| String::new());
    
    {
        let assets = assets.clone();
        let total = total.clone();
        let loading = loading.clone();
        let error = error.clone();
        
        use_effect_with((), move |_| {
            loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                let params = AssetQueryParams {
                    keyword: None,
                    status: None,
                    asset_category: None,
                    page: Some(1),
                    page_size: Some(20),
                };
                
                match FixedAssetService::list_assets(params).await {
                    Ok(resp) => {
                        assets.set(resp.items);
                        total.set(resp.total);
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(format!("加载失败: {}", e));
                        loading.set(false);
                    }
                }
            });
            || ()
        });
    }

    html! {
        <div class="fixed-asset-page">
            <div class="header" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                <h1>{"固定资产管理"}</h1>
                <button class="btn btn-primary">{"新增资产"}</button>
            </div>
            
            if *loading {
                <div class="loading">{"加载中..."}</div>
            } else if !(*error).is_empty() {
                <div class="error" style="color: red; margin-bottom: 10px;">{ (*error).clone() }</div>
            } else {
                <div class="summary" style="margin-bottom: 10px;">
                    {format!("共找到 {} 项资产", *total)}
                </div>
                <table class="table" style="width: 100%; border-collapse: collapse;">
                    <thead>
                        <tr style="background-color: #f5f5f5; text-align: left;">
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"资产编号"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"资产名称"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"分类"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"原值"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"当前价值"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"状态"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"购入日期"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        { for assets.iter().map(|a| {
                            html! {
                                <tr style="border-bottom: 1px solid #eee;">
                                    <td style="padding: 10px;">{ &a.asset_no }</td>
                                    <td style="padding: 10px;">{ &a.asset_name }</td>
                                    <td style="padding: 10px;">{ a.asset_category.clone().unwrap_or_default() }</td>
                                    <td style="padding: 10px;">{ &a.original_value }</td>
                                    <td style="padding: 10px;">{ a.current_value.clone().unwrap_or_default() }</td>
                                    <td style="padding: 10px;">{ a.status.clone().unwrap_or_default() }</td>
                                    <td style="padding: 10px;">{ &a.purchase_date }</td>
                                    <td style="padding: 10px;">
                                        <button class="btn btn-sm" style="margin-right: 5px;">{"查看"}</button>
                                        if a.status.as_deref() == Some("IN_USE") {
                                            <button class="btn btn-sm btn-warning" style="margin-right: 5px;">{"计提折旧"}</button>
                                            <button class="btn btn-sm btn-danger">{"处置"}</button>
                                        }
                                    </td>
                                </tr>
                            }
                        }) }
                    </tbody>
                </table>
            }
        </div>
    }
}
