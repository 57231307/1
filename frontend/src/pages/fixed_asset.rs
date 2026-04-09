//! 固定资产管理页面

use crate::components::main_layout::MainLayout;
use crate::models::fixed_asset::{AssetQueryParams, FixedAsset};
use crate::services::fixed_asset_service::FixedAssetService;
use yew::prelude::*;

#[function_component(FixedAssetPage)]
pub fn fixed_asset_page() -> Html {
    let assets = use_state(|| Vec::<FixedAsset>::new());

    {
        let assets = assets.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let params = AssetQueryParams {
                    keyword: None,
                    status: None,
                    asset_category: None,
                    page: Some(1),
                    page_size: Some(10),
                };
                if let Ok(res) = FixedAssetService::list_assets(params).await {
                    assets.set(res.items);
                }
            });
            || ()
        });
    }

    html! {
        <MainLayout current_page={"fixed_asset"}>
<div class="fixed-asset-page p-4">
            <div class="header mb-4">
                <h1 class="text-2xl font-bold">{"固定资产管理"}</h1>
            </div>
            <div class="content">
                <div class="table-responsive">
                    <table class="data-table w-full">
                        <thead>
                            <tr>
                                <th>{"ID"}</th>
                                <th>{"资产编号"}</th>
                                <th>{"资产名称"}</th>
                                <th>{"资产类别"}</th>
                                <th class="numeric-cell text-right">{"原值"}</th>
                                <th>{"状态"}</th>
                                <th>{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {
                                if assets.is_empty() {
                                    html! {
                                        <tr><td colspan="7" class="text-center py-4">{"暂无数据"}</td></tr>
                                    }
                                } else {
                                    html! {
                                        for assets.iter().map(|asset| html! {
                                            <tr key={asset.id}>
                                                <td>{ asset.id }</td>
                                                <td>{ &asset.asset_no }</td>
                                                <td>{ &asset.asset_name }</td>
                                                <td>{ asset.asset_category.clone().unwrap_or_default() }</td>
                                                <td class="numeric-cell text-right">{ &asset.original_value }</td>
                                                <td>
                                                    <span class="status-badge">{ asset.status.clone().unwrap_or_default() }</span>
                                                </td>
                                                <td>
                                                    <button class="text-blue-500 hover:text-blue-700">{"查看"}</button>
                                                </td>
                                            </tr>
                                        })
                                    }
                                }
                            }
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    
</MainLayout>}
}
