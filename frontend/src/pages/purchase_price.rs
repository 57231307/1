//! 采购价格管理页面

use crate::components::main_layout::MainLayout;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub struct PurchasePriceItem {
    pub id: i32,
    pub product_name: String,
    pub supplier_name: String,
    pub price: f64,
    pub currency: String,
    pub effective_date: String,
    pub status: String,
}

#[function_component(PurchasePricePage)]
pub fn purchase_price_page() -> Html {
    let prices = use_state(|| Vec::<PurchasePriceItem>::new());

    {
        let prices = prices.clone();
        use_effect_with((), move |_| {
            let initial_data = vec![
                PurchasePriceItem {
                    id: 1,
                    product_name: "32S纯棉精梳纱".to_string(),
                    supplier_name: "山东某纺织集团".to_string(),
                    price: 24500.0,
                    currency: "CNY".to_string(),
                    effective_date: "2023-10-01".to_string(),
                    status: "生效中".to_string(),
                },
                PurchasePriceItem {
                    id: 2,
                    product_name: "75D涤纶DTY".to_string(),
                    supplier_name: "江苏某化纤厂".to_string(),
                    price: 8500.0,
                    currency: "CNY".to_string(),
                    effective_date: "2023-10-05".to_string(),
                    status: "待审核".to_string(),
                },
            ];
            prices.set(initial_data);
            || ()
        });
    }

    html! {
        <MainLayout current_page={"/purchase-prices"}>
            <div class="purchase-price-page p-4">
                <div class="header mb-4 flex justify-between items-center">
                    <h1 class="text-2xl font-bold">{"纱线采购报价管理"}</h1>
                    <button class="bg-blue-500 hover:bg-blue-600 text-white font-bold py-2 px-4 rounded">
                        {"+ 新增报价"}
                    </button>
                </div>
                
                <div class="bg-white p-4 rounded shadow mb-4">
                    <h2 class="text-lg font-semibold mb-2">{"快速新增"}</h2>
                    <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700">{"产品名称"}</label>
                            <input type="text" class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500" placeholder="例如: 32S纯棉精梳纱" />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700">{"供应商"}</label>
                            <input type="text" class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500" placeholder="例如: 山东某纺织集团" />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700">{"价格"}</label>
                            <input type="number" class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500" placeholder="例如: 24500.0" />
                        </div>
                        <div class="flex items-end">
                            <button class="w-full bg-green-500 hover:bg-green-600 text-white font-bold py-2 px-4 rounded">
                                {"保存"}
                            </button>
                        </div>
                    </div>
                </div>

                <div class="content table-responsive">
                    <table class="data-table w-full text-left border-collapse">
                        <thead>
                            <tr class="bg-gray-100 border-b">
                                <th class="p-2">{"ID"}</th>
                                <th class="p-2">{"产品名称"}</th>
                                <th class="p-2">{"供应商名称"}</th>
                                <th class="p-2 numeric-cell text-right">{"价格"}</th>
                                <th class="p-2">{"币种"}</th>
                                <th class="p-2">{"生效日期"}</th>
                                <th class="p-2">{"状态"}</th>
                                <th class="p-2">{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {
                                if prices.is_empty() {
                                    html! {
                                        <tr><td colspan="8" class="text-center py-4">{"暂无数据"}</td></tr>
                                    }
                                } else {
                                    html! {
                                        for prices.iter().map(|price| html! {
                                            <tr key={price.id} class="border-b hover:bg-gray-50">
                                                <td class="p-2">{ price.id }</td>
                                                <td class="p-2">{ &price.product_name }</td>
                                                <td class="p-2">{ &price.supplier_name }</td>
                                                <td class="p-2 numeric-cell text-right">{ format!("{:.2}", price.price) }</td>
                                                <td class="p-2">{ &price.currency }</td>
                                                <td class="p-2">{ &price.effective_date }</td>
                                                <td class="p-2"><span class="status-badge bg-blue-100 text-blue-800 px-2 py-1 rounded text-sm">{ &price.status }</span></td>
                                                <td class="p-2">
                                                    <button class="text-blue-500 hover:text-blue-700 mr-2">{"查看"}</button>
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
