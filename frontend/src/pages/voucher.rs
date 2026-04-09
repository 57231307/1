use yew::prelude::*;
use crate::components::main_layout::MainLayout;

#[derive(Clone, PartialEq)]
pub struct VoucherItem {
    pub id: u32,
    pub date: String,
    pub voucher_word: String,
    pub voucher_number: u32,
    pub summary: String,
    pub total_amount: f64,
    pub status: String,
    pub creator: String,
    pub related_batch: Option<String>,
    pub related_color: Option<String>,
}

#[function_component(VoucherPage)]
pub fn voucher_page() -> Html {
    let vouchers = use_state(|| Vec::<VoucherItem>::new());

    {
        let vouchers = vouchers.clone();
        use_effect_with((), move |_| {
            // 初始化纺织行业凭证模拟数据
            let mock_data = vec![
                VoucherItem {
                    id: 1,
                    date: "2023-10-15".to_string(),
                    voucher_word: "记".to_string(),
                    voucher_number: 101,
                    summary: "发放车间挡车工工资".to_string(),
                    total_amount: 156_000.00,
                    status: "已记账".to_string(),
                    creator: "张会计".to_string(),
                    related_batch: None,
                    related_color: None,
                },
                VoucherItem {
                    id: 2,
                    date: "2023-10-16".to_string(),
                    voucher_word: "记".to_string(),
                    voucher_number: 102,
                    summary: "采购32S纯棉纱线".to_string(),
                    total_amount: 450_000.00,
                    status: "已审核".to_string(),
                    creator: "李财务".to_string(),
                    related_batch: Some("B2023101601".to_string()),
                    related_color: Some("漂白".to_string()),
                },
                VoucherItem {
                    id: 3,
                    date: "2023-10-17".to_string(),
                    voucher_word: "转".to_string(),
                    voucher_number: 103,
                    summary: "计提定型机本月折旧".to_string(),
                    total_amount: 12_500.00,
                    status: "草稿".to_string(),
                    creator: "王出纳".to_string(),
                    related_batch: None,
                    related_color: None,
                },
            ];
            vouchers.set(mock_data);
            || ()
        });
    }

    html! {
        <MainLayout current_page={"voucher"}>
            <div class="p-4">
                <div class="flex justify-between items-center mb-4">
                    <h1 class="text-2xl font-bold">{"凭证管理"}</h1>
                    <button class="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700">
                        {"新增凭证"}
                    </button>
                </div>

                // 紧凑网格表单
                <div class="grid grid-cols-1 md:grid-cols-5 gap-4 mb-4 bg-white p-4 shadow rounded">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">{"凭证日期"}</label>
                        <input type="date" class="w-full border rounded px-2 py-1 text-sm" />
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">{"凭证字"}</label>
                        <select class="w-full border rounded px-2 py-1 text-sm">
                            <option>{"全部"}</option>
                            <option>{"记"}</option>
                            <option>{"收"}</option>
                            <option>{"付"}</option>
                            <option>{"转"}</option>
                        </select>
                    </div>
                    <div class="md:col-span-2">
                        <label class="block text-sm font-medium text-gray-700 mb-1">{"摘要词"}</label>
                        <input type="text" class="w-full border rounded px-2 py-1 text-sm" placeholder="如: 采购棉纱, 工资发放..." />
                    </div>
                    <div class="flex items-end">
                        <button class="bg-gray-100 text-gray-700 px-4 py-1 rounded border hover:bg-gray-200 text-sm w-full">
                            {"查询"}
                        </button>
                    </div>
                </div>

                <div class="bg-white shadow rounded overflow-hidden">
                    <table class="data-table w-full text-sm">
                        <thead class="bg-gray-50 border-b">
                            <tr>
                                <th class="px-4 py-2 text-left">{"ID"}</th>
                                <th class="px-4 py-2 text-left">{"日期"}</th>
                                <th class="px-4 py-2 text-left">{"凭证字号"}</th>
                                <th class="px-4 py-2 text-left">{"摘要"}</th>
                                <th class="px-4 py-2 text-left">{"相关批次"}</th>
                                <th class="px-4 py-2 text-left">{"相关色号"}</th>
                                <th class="numeric-cell text-right px-4 py-2">{"总金额"}</th>
                                <th class="px-4 py-2 text-center">{"制单人"}</th>
                                <th class="px-4 py-2 text-center">{"状态"}</th>
                                <th class="px-4 py-2 text-center">{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {
                                for vouchers.iter().map(|voucher| html! {
                                    <tr class="border-b hover:bg-gray-50" key={voucher.id}>
                                        <td class="px-4 py-2">{ voucher.id }</td>
                                        <td class="px-4 py-2">{ &voucher.date }</td>
                                        <td class="px-4 py-2">{ format!("{}-{:03}", voucher.voucher_word, voucher.voucher_number) }</td>
                                        <td class="px-4 py-2">{ &voucher.summary }</td>
                                        <td class="px-4 py-2">{ voucher.related_batch.clone().unwrap_or_else(|| "-".to_string()) }</td>
                                        <td class="px-4 py-2">{ voucher.related_color.clone().unwrap_or_else(|| "-".to_string()) }</td>
                                        <td class="numeric-cell text-right px-4 py-2">{ format!("{:.2}", voucher.total_amount) }</td>
                                        <td class="px-4 py-2 text-center">{ &voucher.creator }</td>
                                        <td class="px-4 py-2 text-center">
                                            <span class="status-badge bg-purple-100 text-purple-800 px-2 py-1 rounded text-xs">{ &voucher.status }</span>
                                        </td>
                                        <td class="px-4 py-2 text-center">
                                            <button class="text-blue-500 hover:text-blue-700 mr-2">{"查看"}</button>
                                            <button class="text-green-500 hover:text-green-700">{"打印"}</button>
                                        </td>
                                    </tr>
                                })
                            }
                        </tbody>
                    </table>
                </div>
            </div>
        </MainLayout>
    }
}
