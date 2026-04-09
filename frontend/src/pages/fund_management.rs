//! 资金管理页面

use yew::prelude::*;
use crate::components::main_layout::MainLayout;

#[derive(Clone, PartialEq)]
pub struct FundItem {
    pub id: u32,
    pub account_name: String,
    pub account_no: String,
    pub balance: f64,
    pub status: String,
}

#[function_component(FundManagementPage)]
pub fn fund_management_page() -> Html {
    let fund_items = use_state(|| Vec::<FundItem>::new());

    {
        let fund_items = fund_items.clone();
        use_effect_with((), move |_| {
            fund_items.set(vec![
                FundItem {
                    id: 1,
                    account_name: "基本户-工行".to_string(),
                    account_no: "6222020000001234".to_string(),
                    balance: 1250000.50,
                    status: "正常".to_string(),
                },
                FundItem {
                    id: 2,
                    account_name: "外汇结算户-中行".to_string(),
                    account_no: "6222020000005678".to_string(),
                    balance: 850000.00,
                    status: "正常".to_string(),
                },
                FundItem {
                    id: 3,
                    account_name: "专用户-建行".to_string(),
                    account_no: "6222020000009012".to_string(),
                    balance: 300000.00,
                    status: "冻结".to_string(),
                },
            ]);
            || ()
        });
    }

    html! {
        <MainLayout current_page={"fund_management"}>
            <div class="fund-management-page p-4">
                <div class="header mb-4 flex justify-between items-center">
                    <h1 class="text-2xl font-bold">{"资金管理"}</h1>
                    <button class="bg-green-500 hover:bg-green-600 text-white px-4 py-2 rounded">
                        {"新增账户"}
                    </button>
                </div>
                
                <div class="grid-form mb-6 grid grid-cols-1 md:grid-cols-4 gap-4 bg-white p-4 rounded shadow">
                    <div class="form-group">
                        <label class="block text-sm font-medium text-gray-700 mb-1">{"账户名称"}</label>
                        <input type="text" class="w-full border rounded px-3 py-2" placeholder="请输入账户名称" />
                    </div>
                    <div class="form-group">
                        <label class="block text-sm font-medium text-gray-700 mb-1">{"账号"}</label>
                        <input type="text" class="w-full border rounded px-3 py-2" placeholder="请输入账号" />
                    </div>
                    <div class="form-group">
                        <label class="block text-sm font-medium text-gray-700 mb-1">{"账户状态"}</label>
                        <select class="w-full border rounded px-3 py-2">
                            <option>{"全部"}</option>
                            <option>{"正常"}</option>
                            <option>{"冻结"}</option>
                        </select>
                    </div>
                    <div class="form-group flex items-end">
                        <button class="bg-gray-100 hover:bg-gray-200 text-gray-800 px-4 py-2 rounded w-full">
                            {"查询"}
                        </button>
                    </div>
                </div>

                <div class="content bg-white rounded shadow overflow-hidden">
                    <div class="table-responsive">
                        <table class="data-table w-full">
                            <thead class="bg-gray-50 border-b">
                                <tr>
                                    <th class="px-4 py-3 text-left">{"ID"}</th>
                                    <th class="px-4 py-3 text-left">{"账户名称"}</th>
                                    <th class="px-4 py-3 text-left">{"账号"}</th>
                                    <th class="px-4 py-3 text-right">{"余额"}</th>
                                    <th class="px-4 py-3 text-center">{"状态"}</th>
                                    <th class="px-4 py-3 text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {
                                    if fund_items.is_empty() {
                                        html! {
                                            <tr><td colspan="6" class="text-center py-4">{"暂无数据"}</td></tr>
                                        }
                                    } else {
                                        html! {
                                            for fund_items.iter().map(|item| html! {
                                                <tr key={item.id} class="border-b hover:bg-gray-50">
                                                    <td class="px-4 py-3">{ item.id }</td>
                                                    <td class="px-4 py-3">{ &item.account_name }</td>
                                                    <td class="px-4 py-3">{ &item.account_no }</td>
                                                    <td class="numeric-cell text-right px-4 py-3">{ format!("{:.2}", item.balance) }</td>
                                                    <td class="px-4 py-3 text-center">
                                                        <span class="status-badge px-2 py-1 rounded text-xs bg-green-100 text-green-800">{ &item.status }</span>
                                                    </td>
                                                    <td class="px-4 py-3 text-center">
                                                        <button class="text-blue-500 hover:text-blue-700 mr-2">{"调拨"}</button>
                                                        <button class="text-blue-500 hover:text-blue-700 mr-2">{"明细"}</button>
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
        </MainLayout>
    }
}
