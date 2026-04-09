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
        <MainLayout current_page={""}>
<div class="fixed-asset-page p-4">
            <div class="header mb-4">
                <h1 class="text-2xl font-bold">{"固定资产管理"}</h1>
            </div>
            <div class="content">
                <table class="min-w-full bg-white border border-gray-200">
                    <thead>
                        <tr>
                            <th class="py-2 px-4 border-b">{"ID"}</th>
                            <th class="py-2 px-4 border-b">{"资产编号"}</th>
                            <th class="py-2 px-4 border-b">{"资产名称"}</th>
                            <th class="py-2 px-4 border-b">{"资产类别"}</th>
                            <th class="py-2 px-4 border-b">{"原值"}</th>
                            <th class="py-2 px-4 border-b">{"状态"}</th>
                            <th class="py-2 px-4 border-b">{"操作"}</th>
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
                                            <td class="py-2 px-4 border-b text-center">{ asset.id }</td>
                                            <td class="py-2 px-4 border-b text-center">{ &asset.asset_no }</td>
                                            <td class="py-2 px-4 border-b text-center">{ &asset.asset_name }</td>
                                            <td class="py-2 px-4 border-b text-center">{ asset.asset_category.clone().unwrap_or_default() }</td>
                                            <td class="py-2 px-4 border-b text-center">{ &asset.original_value }</td>
                                            <td class="py-2 px-4 border-b text-center">{ asset.status.clone().unwrap_or_default() }</td>
                                            <td class="py-2 px-4 border-b text-center">
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
    
</MainLayout>}
}
