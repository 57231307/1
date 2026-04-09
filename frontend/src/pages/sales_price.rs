//! 销售价格管理页面

use crate::components::main_layout::MainLayout;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub struct SalePriceItem {
    pub id: i32,
    pub product_name: String,
    pub price: f64,
    pub currency: String,
    pub effective_date: String,
    pub status: String,
}

#[function_component(SalesPricePage)]
pub fn sales_price_page() -> Html {
    let prices = use_state(|| Vec::<SalePriceItem>::new());

    {
        let prices = prices.clone();
        use_effect_with((), move |_| {
            prices.set(vec![
                SalePriceItem {
                    id: 1,
                    product_name: "特价全棉布报价".to_string(),
                    price: 15.5,
                    currency: "CNY".to_string(),
                    effective_date: "2024-05-01".to_string(),
                    status: "已生效".to_string(),
                },
                SalePriceItem {
                    id: 2,
                    product_name: "高织真丝面料".to_string(),
                    price: 120.0,
                    currency: "CNY".to_string(),
                    effective_date: "2024-06-01".to_string(),
                    status: "待审核".to_string(),
                },
                SalePriceItem {
                    id: 3,
                    product_name: "混纺针织布大宗报价".to_string(),
                    price: 28.8,
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
            <div class="p-4">
                <h1 class="text-2xl font-bold mb-4">{ "销售价格管理" }</h1>
                
                <div class="mb-4 flex justify-between items-center bg-white p-4 rounded shadow-sm border border-gray-200">
                    <div class="flex space-x-4">
                        <input type="text" placeholder="搜索产品名称..." class="border border-gray-300 p-2 rounded w-64 focus:outline-none focus:ring-2 focus:ring-blue-500" />
                        <select class="border border-gray-300 p-2 rounded focus:outline-none focus:ring-2 focus:ring-blue-500">
                            <option value="">{ "所有状态" }</option>
                            <option value="active">{ "已生效" }</option>
                            <option value="pending">{ "待审核" }</option>
                        </select>
                        <button class="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded transition-colors">{ "筛选" }</button>
                    </div>
                    <button class="bg-green-500 hover:bg-green-600 text-white px-4 py-2 rounded transition-colors flex items-center">
                        <span class="mr-1">{ "+" }</span> { "新增报价" }
                    </button>
                </div>

                <div class="overflow-x-auto bg-white rounded shadow-sm border border-gray-200">
                    <table class="data-table w-full text-left border-collapse">
                        <thead class="bg-gray-50 border-b border-gray-200">
                            <tr>
                                <th class="py-3 px-4 text-gray-700 font-semibold">{ "产品名称" }</th>
                                <th class="py-3 px-4 text-gray-700 font-semibold numeric-cell text-right">{ "价格" }</th>
                                <th class="py-3 px-4 text-gray-700 font-semibold">{ "货币" }</th>
                                <th class="py-3 px-4 text-gray-700 font-semibold">{ "生效日期" }</th>
                                <th class="py-3 px-4 text-gray-700 font-semibold">{ "状态" }</th>
                                <th class="py-3 px-4 text-gray-700 font-semibold text-center">{ "操作" }</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-gray-200">
                            {
                                if prices.is_empty() {
                                    html! { <tr><td colspan="6" class="text-center py-8 text-gray-500">{ "暂无数据" }</td></tr> }
                                } else {
                                    html! {
                                        for prices.iter().map(|price| html! {
                                            <tr key={price.id} class="hover:bg-gray-50 transition-colors">
                                                <td class="py-3 px-4">{ &price.product_name }</td>
                                                <td class="py-3 px-4 numeric-cell text-right font-medium">{ format!("{:.2}", price.price) }</td>
                                                <td class="py-3 px-4 text-gray-600">{ &price.currency }</td>
                                                <td class="py-3 px-4 text-gray-600">{ &price.effective_date }</td>
                                                <td class="py-3 px-4"><span class="status-badge px-2 py-1 rounded text-sm bg-blue-100 text-blue-800">{ &price.status }</span></td>
                                                <td class="py-3 px-4 text-center">
                                                    <button class="text-blue-600 hover:text-blue-800 mr-3">{ "编辑" }</button>
                                                    <button class="text-red-600 hover:text-red-800">{ "删除" }</button>
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
        </MainLayout>
    }
}
