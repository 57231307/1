//! 库存查询页面

use crate::components::main_layout::MainLayout;
use yew::prelude::*;
use web_sys::window;

#[derive(Clone, PartialEq)]
pub struct StockItem {
    pub id: u32,
    pub barcode: String,
    pub roll_no: String,
    pub product_name: String,
    pub color: String,
    pub roll_length: f64,
    pub weight: f64,
    pub dye_lot_no: String,
    pub status: String,
}

#[function_component(InventoryStockPage)]
pub fn inventory_stock_page() -> Html {
    let stocks = use_state(|| Vec::<StockItem>::new());
    let on_print = Callback::from(|_: yew::MouseEvent| {
        if let Some(win) = window() {
            let _ = win.print();
        }
    });


    {
        let stocks = stocks.clone();
        use_effect_with((), move |_| {
            let initial_data = vec![
                StockItem {
                    id: 1,
                    barcode: "BC-2023-001".to_string(),
                    roll_no: "匹号A101".to_string(),
                    product_name: "纯棉平布".to_string(),
                    color: "漂白".to_string(),
                    roll_length: 120.5,
                    weight: 25.4,
                    dye_lot_no: "DYE-101".to_string(),
                    status: "可用".to_string(),
                },
                StockItem {
                    id: 2,
                    barcode: "BC-2023-002".to_string(),
                    roll_no: "匹号A102".to_string(),
                    product_name: "涤纶汗布".to_string(),
                    color: "藏青".to_string(),
                    roll_length: 85.0,
                    weight: 18.2,
                    dye_lot_no: "DYE-102".to_string(),
                    status: "预留".to_string(),
                },
                StockItem {
                    id: 3,
                    barcode: "BC-2023-003".to_string(),
                    roll_no: "匹号A103".to_string(),
                    product_name: "全棉斜纹".to_string(),
                    color: "大红".to_string(),
                    roll_length: 105.0,
                    weight: 22.1,
                    dye_lot_no: "DYE-103".to_string(),
                    status: "待检".to_string(),
                },
            ];
            stocks.set(initial_data);
            || ()
        });
    }

    html! {
        <MainLayout current_page={"inventory_stock"}>
            <div class="inventory-stock-page p-4">
                <div class="header mb-4 flex justify-between items-center">
                    <h1 class="text-2xl font-bold">{"库存查询"}</h1>
                    <button class="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded shadow-sm">{"新增库存"}</button>
                </div>
                
                <div class="filter-form bg-white p-4 rounded mb-4 shadow-sm border border-gray-100">
                    <div class="grid grid-cols-1 md:grid-cols-4 gap-4 items-end">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">{"匹号"}</label>
                            <input type="text" class="block w-full rounded-md border-gray-300 shadow-sm p-2 border focus:border-blue-500 focus:ring-blue-500" placeholder="如: 匹号A102" />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">{"品名"}</label>
                            <input type="text" class="block w-full rounded-md border-gray-300 shadow-sm p-2 border focus:border-blue-500 focus:ring-blue-500" placeholder="输入品名" />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">{"状态"}</label>
                            <select class="block w-full rounded-md border-gray-300 shadow-sm p-2 border focus:border-blue-500 focus:ring-blue-500">
                                <option value="">{"全部"}</option>
                                <option value="可用">{"可用"}</option>
                                <option value="预留">{"预留"}</option>
                                <option value="待检">{"待检"}</option>
                            </select>
                        </div>
                        <div>
                            <button class="w-full bg-gray-100 hover:bg-gray-200 text-gray-800 px-4 py-2 rounded border border-gray-300 shadow-sm font-medium">{"查询"}</button>
                        </div>
                    </div>
                </div>

                <div class="content bg-white rounded shadow-sm border border-gray-100 overflow-hidden">
                    <div class="overflow-x-auto w-full pb-4">
<table class="data-table w-full text-left">
                        <thead class="bg-gray-50 border-b border-gray-200">
                            <tr>
                                <th class="py-3 px-4 font-semibold text-gray-600">{"ID"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600">{"条码编号"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600">{"匹号"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600">{"品名"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600">{"颜色"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600 numeric-cell text-right">{"卷长(m)"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600 numeric-cell text-right">{"重量(kg)"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600">{"入库缸号"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600 text-center">{"状态"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600 text-center">{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-gray-100">
                            {
                                if stocks.is_empty() {
                                    html! {
                                        <tr><td colspan="10" class="text-center py-8 text-gray-500">{"暂无数据"}</td></tr>
                                    }
                                } else {
                                    html! {
                                        for stocks.iter().map(|stock| {
                                            let status_class = match stock.status.as_str() {
                                                "可用" => "bg-green-100 text-green-800",
                                                "预留" => "bg-blue-100 text-blue-800",
                                                "待检" => "bg-yellow-100 text-yellow-800",
                                                _ => "bg-gray-100 text-gray-800"
                                            };
                                            html! {
                                                <tr key={stock.id} class="hover:bg-gray-50 transition-colors">
                                                    <td class="py-3 px-4">{ stock.id }</td>
                                                    <td class="py-3 px-4 font-mono text-sm">{ &stock.barcode }</td>
                                                    <td class="py-3 px-4 font-medium">{ &stock.roll_no }</td>
                                                    <td class="py-3 px-4">{ &stock.product_name }</td>
                                                    <td class="py-3 px-4">{ &stock.color }</td>
                                                    <td class="py-3 px-4 numeric-cell text-right font-mono">{ format!("{:.1}", stock.roll_length) }</td>
                                                    <td class="py-3 px-4 numeric-cell text-right font-mono">{ format!("{:.1}", stock.weight) }</td>
                                                    <td class="py-3 px-4 font-mono text-sm">{ &stock.dye_lot_no }</td>
                                                    <td class="py-3 px-4 text-center">
                                                        <span class={format!("status-badge px-2.5 py-1 rounded-full text-xs font-medium {}", status_class)}>
                                                            { &stock.status }
                                                        </span>
                                                    </td>
                                                    <td class="py-3 px-4 text-center">
                                                        <button class="text-blue-600 hover:text-blue-800 font-medium">{"查看明细"}</button>
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
