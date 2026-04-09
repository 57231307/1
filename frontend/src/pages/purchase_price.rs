//! 采购价格管理页面

use crate::components::main_layout::MainLayout;
use yew::prelude::*;
use std::collections::HashMap;
use wasm_bindgen::JsCast;

#[derive(Clone, PartialEq)]
pub struct PurchasePriceItem {
    pub id: i32,
    pub product_name: String,
    pub supplier_name: String,
    pub moq: f64,
    pub moq_unit: String,
    pub price: f64,
    pub tiered_price: f64,
    pub currency: String,
    pub effective_date: String,
    pub status: String,
}

#[function_component(PurchasePricePage)]
pub fn purchase_price_page() -> Html {
    let prices = use_state(|| Vec::<PurchasePriceItem>::new());
    let quantities = use_state(|| HashMap::<i32, f64>::new());

    {
        let prices = prices.clone();
        use_effect_with((), move |_| {
            let initial_data = vec![
                PurchasePriceItem {
                    id: 1,
                    product_name: "32S纯棉精梳纱".to_string(),
                    supplier_name: "山东某纺织集团".to_string(),
                    moq: 5000.0,
                    moq_unit: "公斤".to_string(),
                    price: 24500.0,
                    tiered_price: 24000.0,
                    currency: "CNY".to_string(),
                    effective_date: "2023-10-01".to_string(),
                    status: "生效中".to_string(),
                },
                PurchasePriceItem {
                    id: 2,
                    product_name: "75D涤纶DTY".to_string(),
                    supplier_name: "江苏某化纤厂".to_string(),
                    moq: 10000.0,
                    moq_unit: "公斤".to_string(),
                    price: 8500.0,
                    tiered_price: 8200.0,
                    currency: "CNY".to_string(),
                    effective_date: "2023-10-05".to_string(),
                    status: "待审核".to_string(),
                },
                PurchasePriceItem {
                    id: 3,
                    product_name: "高弹牛仔布坯布".to_string(),
                    supplier_name: "浙江某织造厂".to_string(),
                    moq: 50000.0,
                    moq_unit: "米".to_string(),
                    price: 12.5,
                    tiered_price: 11.8,
                    currency: "CNY".to_string(),
                    effective_date: "2024-03-10".to_string(),
                    status: "生效中".to_string(),
                },
            ];
            prices.set(initial_data);
            || ()
        });
    }

    html! {
        <MainLayout current_page={"/purchase-prices"}>
            <div class="purchase-price-page p-2 text-sm">
                <div class="header mb-2 flex justify-between items-center">
                    <h1 class="text-xl font-bold">{"纱线采购报价管理"}</h1>
                    <button class="bg-blue-500 hover:bg-blue-600 text-white font-bold py-1 px-3 text-sm rounded transition-colors">
                        {"+ 新增报价"}
                    </button>
                </div>
                
                <div class="bg-white p-3 rounded shadow-sm border border-gray-200 mb-2">
                    <h2 class="text-base font-semibold mb-2">{"快速新增"}</h2>
                    <div class="grid grid-cols-1 md:grid-cols-5 gap-3">
                        <div>
                            <label class="block text-xs font-medium text-gray-700">{"产品名称"}</label>
                            <input type="text" class="mt-1 block w-full text-sm rounded border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 p-1 border" placeholder="32S纯棉精梳纱" />
                        </div>
                        <div>
                            <label class="block text-xs font-medium text-gray-700">{"供应商"}</label>
                            <input type="text" class="mt-1 block w-full text-sm rounded border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 p-1 border" placeholder="某纺织集团" />
                        </div>
                        <div>
                            <label class="block text-xs font-medium text-gray-700">{"起订量及单位"}</label>
                            <input type="text" class="mt-1 block w-full text-sm rounded border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 p-1 border" placeholder="5000公斤" />
                        </div>
                        <div>
                            <label class="block text-xs font-medium text-gray-700">{"基础/阶梯价格"}</label>
                            <input type="text" class="mt-1 block w-full text-sm rounded border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 p-1 border" placeholder="24500 / 24000" />
                        </div>
                        <div class="flex items-end">
                            <button class="w-full bg-green-500 hover:bg-green-600 text-white font-bold py-1 px-3 text-sm rounded transition-colors h-[28px]">
                                {"保存"}
                            </button>
                        </div>
                    </div>
                </div>

                <div class="overflow-x-auto bg-white rounded shadow-sm border border-gray-200">
                    <table class="data-table w-full text-left border-collapse">
                        <thead class="bg-gray-50 border-b border-gray-200">
                            <tr>
                                <th class="py-2 px-3 text-gray-700 font-semibold">{"产品名称"}</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold">{"供应商名称"}</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold">{"起订量(MOQ)"}</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold numeric-cell text-right">{"基础单价"}</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold numeric-cell text-right">{"阶梯单价"}</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold">{"试算数量"}</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold numeric-cell text-right">{"总价估算"}</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold">{"状态"}</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold text-center">{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-gray-200">
                            {
                                if prices.is_empty() {
                                    html! {
                                        <tr><td colspan="9" class="text-center py-4 text-gray-500">{"暂无数据"}</td></tr>
                                    }
                                } else {
                                    html! {
                                        for prices.iter().map(|price| {
                                            let qty = quantities.get(&price.id).copied().unwrap_or(0.0);
                                            let total = if qty >= price.moq { qty * price.tiered_price } else { qty * price.price };
                                            
                                            let oninput = {
                                                let quantities = quantities.clone();
                                                let id = price.id;
                                                Callback::from(move |e: InputEvent| {
                                                    let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                                    let val = input.value().parse::<f64>().unwrap_or(0.0);
                                                    let mut map = (*quantities).clone();
                                                    map.insert(id, val);
                                                    quantities.set(map);
                                                })
                                            };

                                            html! {
                                                <tr key={price.id} class="hover:bg-gray-50 transition-colors">
                                                    <td class="py-2 px-3">{ &price.product_name }</td>
                                                    <td class="py-2 px-3">{ &price.supplier_name }</td>
                                                    <td class="py-2 px-3">{ format!("{:.0}{}", price.moq, price.moq_unit) }</td>
                                                    <td class="py-2 px-3 numeric-cell text-right font-medium">{ format!("{:.2}", price.price) }</td>
                                                    <td class="py-2 px-3 numeric-cell text-right font-medium text-green-600">{ format!("{:.2}", price.tiered_price) }</td>
                                                    <td class="py-2 px-3">
                                                        <div class="flex items-center">
                                                            <input type="number" min="0" step="1" {oninput} class="border border-gray-300 p-1 text-sm rounded w-20 focus:outline-none focus:ring-1 focus:ring-blue-500" placeholder="数量" />
                                                        </div>
                                                    </td>
                                                    <td class="py-2 px-3 numeric-cell text-right font-bold text-blue-600">{ format!("{:.2}", total) }</td>
                                                    <td class="py-2 px-3"><span class="status-badge bg-blue-100 text-blue-800 px-2 py-0.5 rounded text-xs">{ &price.status }</span></td>
                                                    <td class="py-2 px-3 text-center">
                                                        <button class="text-blue-500 hover:text-blue-700 mr-2">{"查看"}</button>
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
        </MainLayout>
    }
}
