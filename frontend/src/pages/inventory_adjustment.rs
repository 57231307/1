//! 库存调整单页面

use crate::components::main_layout::MainLayout;
use yew::prelude::*;
use web_sys::window;

#[derive(Clone, PartialEq)]
pub struct AdjustmentItem {
    pub id: u32,
    pub adjust_no: String,
    pub barcode: String,
    pub roll_no: String,
    pub dye_lot_no: String,
    pub adjust_type: String, // 调整类型：盘点差异调整、报损等
    pub adjust_length: f64, // 调整长度(米)
    pub adjust_weight: f64, // 调整重量(kg)
    pub reason: String,
    pub status: String,
}

#[function_component(InventoryAdjustmentPage)]
pub fn inventory_adjustment_page() -> Html {
    let adjustments = use_state(|| Vec::<AdjustmentItem>::new());
    let on_print = Callback::from(|_: yew::MouseEvent| {
        if let Some(win) = window() {
            let _ = win.print();
        }
    });


    {
        let adjustments = adjustments.clone();
        use_effect_with((), move |_| {
            let initial_data = vec![
                AdjustmentItem {
                    id: 1,
                    adjust_no: "ADJ-20231001-001".to_string(),
                    barcode: "BC-2023-001".to_string(),
                    roll_no: "匹号A101".to_string(),
                    dye_lot_no: "DYE-101".to_string(),
                    adjust_type: "盘点差异调整".to_string(),
                    adjust_length: -2.5,
                    adjust_weight: -0.5,
                    reason: "月末盘点少米数".to_string(),
                    status: "已审核".to_string(),
                },
                AdjustmentItem {
                    id: 2,
                    adjust_no: "ADJ-20231002-002".to_string(),
                    barcode: "BC-2023-002".to_string(),
                    roll_no: "匹号B205".to_string(),
                    dye_lot_no: "DYE-102".to_string(),
                    adjust_type: "质量报损".to_string(),
                    adjust_length: -15.0,
                    adjust_weight: -3.2,
                    reason: "大面积油污报废".to_string(),
                    status: "待审核".to_string(),
                },
                AdjustmentItem {
                    id: 3,
                    adjust_no: "ADJ-20231005-003".to_string(),
                    barcode: "BC-2023-003".to_string(),
                    roll_no: "匹号C302".to_string(),
                    dye_lot_no: "DYE-103".to_string(),
                    adjust_type: "盘点差异调整".to_string(),
                    adjust_length: 1.0,
                    adjust_weight: 0.2,
                    reason: "盘盈入库".to_string(),
                    status: "已审核".to_string(),
                },
            ];
            adjustments.set(initial_data);
            || ()
        });
    }

    html! {
        <MainLayout current_page={"inventory_adjustment"}>
            <div class="inventory-adjustment-page p-4">
                <div class="header mb-4 flex justify-between items-center">
                    <h1 class="text-2xl font-bold">{"库存调整单"}</h1>
                    <button class="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded shadow-sm">{"新增调整单"}</button>
                </div>

                <div class="filter-form bg-white p-4 rounded mb-4 shadow-sm border border-gray-100">
                    <div class="grid grid-cols-1 md:grid-cols-4 gap-4 items-end">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">{"调整单号"}</label>
                            <input type="text" class="block w-full rounded-md border-gray-300 shadow-sm p-2 border focus:border-blue-500 focus:ring-blue-500" placeholder="如: ADJ-..." />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">{"匹号"}</label>
                            <input type="text" class="block w-full rounded-md border-gray-300 shadow-sm p-2 border focus:border-blue-500 focus:ring-blue-500" placeholder="如: 匹号A102" />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">{"调整类型"}</label>
                            <select class="block w-full rounded-md border-gray-300 shadow-sm p-2 border focus:border-blue-500 focus:ring-blue-500">
                                <option value="">{"全部"}</option>
                                <option value="盘点差异调整">{"盘点差异调整"}</option>
                                <option value="质量报损">{"质量报损"}</option>
                            </select>
                        </div>
                        <div>
                            <button class="w-full bg-gray-100 hover:bg-gray-200 text-gray-800 px-4 py-2 rounded border border-gray-300 shadow-sm font-medium">{"查询"}</button>
                        </div>
                    </div>
                </div>

                <div class="content bg-white rounded shadow-sm border border-gray-100 overflow-hidden">
                    <div class="table-responsive overflow-x-auto w-full pb-4 shadow-sm sm:rounded-lg">
<table class="data-table w-full text-left">
                        <thead class="bg-gray-50 border-b border-gray-200">
                            <tr>
                                <th class="py-3 px-4 font-semibold text-gray-600">{"ID"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600">{"调整单号"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600">{"条码编号"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600">{"匹号"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600">{"入库缸号"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600">{"调整类型"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600 numeric-cell text-right">{"调整长度(m)"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600 numeric-cell text-right">{"调整重量(kg)"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600">{"调整原因"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600 text-center">{"状态"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600 text-center">{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-gray-100">
                            {
                                if adjustments.is_empty() {
                                    html! {
                                        <tr><td colspan="11" class="text-center py-8 text-gray-500">{"暂无数据"}</td></tr>
                                    }
                                } else {
                                    html! {
                                        for adjustments.iter().map(|adj| {
                                            let status_class = match adj.status.as_str() {
                                                "已审核" => "bg-green-100 text-green-800",
                                                "待审核" => "bg-yellow-100 text-yellow-800",
                                                _ => "bg-gray-100 text-gray-800"
                                            };
                                            let length_class = if adj.adjust_length < 0.0 { "text-red-600" } else { "text-green-600" };
                                            let weight_class = if adj.adjust_weight < 0.0 { "text-red-600" } else { "text-green-600" };
                                            html! {
                                                <tr key={adj.id} class="hover:bg-gray-50 transition-colors">
                                                    <td class="py-3 px-4">{ adj.id }</td>
                                                    <td class="py-3 px-4 font-medium">{ &adj.adjust_no }</td>
                                                    <td class="py-3 px-4 font-mono text-sm">{ &adj.barcode }</td>
                                                    <td class="py-3 px-4">{ &adj.roll_no }</td>
                                                    <td class="py-3 px-4 font-mono text-sm">{ &adj.dye_lot_no }</td>
                                                    <td class="py-3 px-4">{ &adj.adjust_type }</td>
                                                    <td class={format!("py-3 px-4 numeric-cell text-right font-mono {}", length_class)}>
                                                        { if adj.adjust_length > 0.0 { format!("+{:.2}", adj.adjust_length) } else { format!("{:.2}", adj.adjust_length) } }
                                                    </td>
                                                    <td class={format!("py-3 px-4 numeric-cell text-right font-mono {}", weight_class)}>
                                                        { if adj.adjust_weight > 0.0 { format!("+{:.2}", adj.adjust_weight) } else { format!("{:.2}", adj.adjust_weight) } }
                                                    </td>
                                                    <td class="py-3 px-4 text-sm text-gray-600">{ &adj.reason }</td>
                                                    <td class="py-3 px-4 text-center">
                                                        <span class={format!("status-badge px-2.5 py-1 rounded-full text-xs font-medium {}", status_class)}>
                                                            { &adj.status }
                                                        </span>
                                                    </td>
                                                    <td class="py-3 px-4 text-center">
                                                        <button class="text-blue-600 hover:text-blue-800 font-medium">{"查看"}</button>
                                                    </td>
                                                </tr>
                                            }
                                        })
                                    }
                                }
                            }
                        </tbody>
                    </table>
</div>
                </div>
            </div>
        </MainLayout>
    }
}
