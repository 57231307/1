use yew::prelude::*;
use web_sys::window;
use crate::components::main_layout::MainLayout;

#[derive(Clone, PartialEq)]
pub struct AssetItem {
    pub id: u32,
    pub asset_no: String,
    pub asset_name: String,
    pub asset_category: String,
    pub original_value: f64,
    pub accumulated_depreciation: f64,
    pub net_value: f64,
    pub depreciation_method: String,
    pub status: String,
}

#[function_component(FixedAssetPage)]
pub fn fixed_asset_page() -> Html {
    let assets = use_state(|| Vec::<AssetItem>::new());
    let on_print = Callback::from(|_: yew::MouseEvent| {
        if let Some(win) = window() {
            let _ = win.print();
        }
    });


    {
        let assets = assets.clone();
        use_effect_with((), move |_| {
            // 初始化纺织行业固定资产模拟数据
            let mock_data = vec![
                AssetItem {
                    id: 1,
                    asset_no: "FA-TX-001".to_string(),
                    asset_name: "高温高压定型机".to_string(),
                    asset_category: "生产设备".to_string(),
                    original_value: 1_200_000.00,
                    accumulated_depreciation: 200_000.00,
                    net_value: 1_000_000.00,
                    depreciation_method: "直线法".to_string(),
                    status: "使用中".to_string(),
                },
                AssetItem {
                    id: 2,
                    asset_no: "FA-TX-002".to_string(),
                    asset_name: "喷气织机".to_string(),
                    asset_category: "生产设备".to_string(),
                    original_value: 850_000.00,
                    accumulated_depreciation: 150_000.00,
                    net_value: 700_000.00,
                    depreciation_method: "工作量法".to_string(),
                    status: "使用中".to_string(),
                },
                AssetItem {
                    id: 3,
                    asset_no: "FA-TX-003".to_string(),
                    asset_name: "自动穿综机".to_string(),
                    asset_category: "辅助设备".to_string(),
                    original_value: 320_000.00,
                    accumulated_depreciation: 320_000.00,
                    net_value: 0.00,
                    depreciation_method: "直线法".to_string(),
                    status: "已报废".to_string(),
                },
            ];
            assets.set(mock_data);
            || ()
        });
    }

    html! {
        <MainLayout current_page={"fixed_asset"}>
            <div class="p-4">
                <div class="flex justify-between items-center mb-4">
                    <h1 class="text-2xl font-bold">{"固定资产管理"}</h1>
                    <button class="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700">
                        {"新增资产"}
                    </button>
                </div>

                // 紧凑网格表单
                <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-4 bg-white p-4 shadow rounded">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">{"资产编号"}</label>
                        <input type="text" class="w-full border rounded px-2 py-1 text-sm" placeholder="请输入编号" />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">{"资产名称"}</label>
                        <input type="text" class="w-full border rounded px-2 py-1 text-sm" placeholder="如: 定型机" />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">{"状态"}</label>
                        <select class="w-full border rounded px-2 py-1 text-sm">
                            <option>{"全部"}</option>
                            <option>{"使用中"}</option>
                            <option>{"闲置"}</option>
                            <option>{"已报废"}</option>
                        </select>
                    </div>
                    <div class="flex items-end">
                        <button class="bg-gray-100 text-gray-700 px-4 py-1 rounded border hover:bg-gray-200 text-sm">
                            {"查询"}
                        </button>
                    </div>
                </div>

                <div class="bg-white shadow rounded overflow-hidden">
                    <div class="overflow-x-auto w-full pb-4">
<table class="data-table w-full text-sm">
                        <thead class="bg-gray-50 border-b">
                            <tr>
                                <th class="px-4 py-2 text-left">{"ID"}</th>
                                <th class="px-4 py-2 text-left">{"资产编号"}</th>
                                <th class="px-4 py-2 text-left">{"资产名称"}</th>
                                <th class="px-4 py-2 text-left">{"资产类别"}</th>
                                <th class="numeric-cell text-right px-4 py-2">{"原值"}</th>
                                <th class="numeric-cell text-right px-4 py-2">{"累计折旧"}</th>
                                <th class="numeric-cell text-right px-4 py-2">{"净值"}</th>
                                <th class="px-4 py-2 text-center">{"折旧方法"}</th>
                                <th class="px-4 py-2 text-center">{"状态"}</th>
                                <th class="px-4 py-2 text-center">{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {
                                for assets.iter().map(|asset| html! {
                                    <tr class="border-b hover:bg-gray-50" key={asset.id}>
                                        <td class="px-4 py-2">{ asset.id }</td>
                                        <td class="px-4 py-2">{ &asset.asset_no }</td>
                                        <td class="px-4 py-2">{ &asset.asset_name }</td>
                                        <td class="px-4 py-2">{ &asset.asset_category }</td>
                                        <td class="numeric-cell text-right px-4 py-2">{ format!("{:.2}", asset.original_value) }</td>
                                        <td class="numeric-cell text-right px-4 py-2">{ format!("{:.2}", asset.accumulated_depreciation) }</td>
                                        <td class="numeric-cell text-right px-4 py-2">{ format!("{:.2}", asset.net_value) }</td>
                                        <td class="px-4 py-2 text-center">
                                            <span class="status-badge bg-blue-100 text-blue-800 px-2 py-1 rounded text-xs">{ &asset.depreciation_method }</span>
                                        </td>
                                        <td class="px-4 py-2 text-center">
                                            <span class="status-badge bg-green-100 text-green-800 px-2 py-1 rounded text-xs">{ &asset.status }</span>
                                        </td>
                                        <td class="px-4 py-2 text-center">
                                            <button class="text-blue-500 hover:text-blue-700 mr-2">{"编辑"}</button>
                                            <button class="text-red-500 hover:text-red-700">{"折旧"}</button>
                                        </td>
                                    </tr>
                                })
                            }
                        </tbody>
                    </table>
</div>
                </div>
            </div>
        </MainLayout>
    }
}
