//! 销售价格管理页面

use crate::components::main_layout::MainLayout;
use yew::prelude::*;
use std::collections::HashMap;
use wasm_bindgen::JsCast;

#[derive(Clone, PartialEq)]
pub struct SalePriceItem {
    pub id: i32,
    pub product_name: String,
    pub moq: f64,
    pub moq_unit: String,
    pub price: f64,
    pub tiered_price: f64,
    pub currency: String,
    pub effective_date: String,
    pub status: String,
}

#[function_component(SalesPricePage)]
pub fn sales_price_page() -> Html {
    let prices = use_state(|| Vec::<SalePriceItem>::new());
    let quantities = use_state(|| HashMap::<i32, f64>::new());

    {
        let prices = prices.clone();
        use_effect_with((), move |_| {
            prices.set(vec![
                SalePriceItem {
                    id: 1,
                    product_name: "特价全棉布报价".to_string(),
                    moq: 1000.0,
                    moq_unit: "米".to_string(),
                    price: 16.5,
                    tiered_price: 15.5,
                    currency: "CNY".to_string(),
                    effective_date: "2024-05-01".to_string(),
                    status: "已生效".to_string(),
                },
                SalePriceItem {
                    id: 2,
                    product_name: "高织真丝面料".to_string(),
                    moq: 100.0,
                    moq_unit: "米".to_string(),
                    price: 130.0,
                    tiered_price: 120.0,
                    currency: "CNY".to_string(),
                    effective_date: "2024-06-01".to_string(),
                    status: "待审核".to_string(),
                },
                SalePriceItem {
                    id: 3,
                    product_name: "混纺针织布大宗报价".to_string(),
                    moq: 2000.0,
                    moq_unit: "公斤".to_string(),
                    price: 30.0,
                    tiered_price: 28.8,
                    currency: "CNY".to_string(),
                    effective_date: "2024-05-15".to_string(),
                    status: "已生效".to_string(),
                },
            ]);
            || ()
        });
    }

    html! {
        <MainLayout current_page={"sales_price"}>
            <div class="p-2 text-sm">
                <h1 class="text-xl font-bold mb-2">{ "销售价格管理" }</h1>
                
                <div class="mb-2 flex justify-between items-center bg-white p-2 rounded shadow-sm border border-gray-200">
                    <div class="flex space-x-2">
                        <input type="text" placeholder="搜索产品名称..." class="border border-gray-300 p-1 text-sm rounded w-64 focus:outline-none focus:ring-1 focus:ring-blue-500" />
                        <select class="border border-gray-300 p-1 text-sm rounded focus:outline-none focus:ring-1 focus:ring-blue-500">
                            <option value="">{ "所有状态" }</option>
                            <option value="active">{ "已生效" }</option>
                            <option value="pending">{ "待审核" }</option>
                        </select>
                        <button class="bg-blue-500 hover:bg-blue-600 text-white px-3 py-1 text-sm rounded transition-colors">{ "筛选" }</button>
                    </div>
                    <button class="bg-green-500 hover:bg-green-600 text-white px-3 py-1 text-sm rounded transition-colors flex items-center">
                        <span class="mr-1">{ "+" }</span> { "新增报价" }
                    </button>
                </div>

                <div class="overflow-x-auto bg-white rounded shadow-sm border border-gray-200">
                    <table class="data-table w-full text-left border-collapse">
                        <thead class="bg-gray-50 border-b border-gray-200">
                            <tr>
                                <th class="py-2 px-3 text-gray-700 font-semibold">{ "产品名称" }</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold">{ "起订量(MOQ)" }</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold numeric-cell text-right">{ "基础单价" }</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold numeric-cell text-right">{ "阶梯单价" }</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold">{ "试算数量" }</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold numeric-cell text-right">{ "总价估算" }</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold">{ "状态" }</th>
                                <th class="py-2 px-3 text-gray-700 font-semibold text-center">{ "操作" }</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-gray-200">
                            {
                                if prices.is_empty() {
                                    html! { <tr><td colspan="8" class="text-center py-4 text-gray-500">{ "暂无数据" }</td></tr> }
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
                                                    <td class="py-2 px-3">{ format!("{:.0}{}", price.moq, price.moq_unit) }</td>
                                                    <td class="py-2 px-3 numeric-cell text-right font-medium">{ format!("{:.2}", price.price) }</td>
                                                    <td class="py-2 px-3 numeric-cell text-right font-medium text-green-600">{ format!("{:.2}", price.tiered_price) }</td>
                                                    <td class="py-2 px-3">
                                                        <div class="flex items-center">
                                                            <input type="number" min="0" step="1" {oninput} class="border border-gray-300 p-1 text-sm rounded w-20 focus:outline-none focus:ring-1 focus:ring-blue-500" placeholder="数量" />
                                                        </div>
                                                    </td>
                                                    <td class="py-2 px-3 numeric-cell text-right font-bold text-blue-600">{ format!("{:.2}", total) }</td>
                                                    <td class="py-2 px-3"><span class="status-badge px-2 py-0.5 rounded text-xs bg-blue-100 text-blue-800">{ &price.status }</span></td>
                                                    <td class="py-2 px-3 text-center">
                                                        <button class="text-blue-600 hover:text-blue-800 mr-2">{ "编辑" }</button>
                                                        <button class="text-red-600 hover:text-red-800">{ "删除" }</button>
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
