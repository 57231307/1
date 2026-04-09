//! 预算管理页面

use yew::prelude::*;
use web_sys::window;
use crate::components::main_layout::MainLayout;

#[derive(Clone, PartialEq)]
pub struct BudgetItem {
    pub id: u32,
    pub name: String,
    pub department: String,
    pub amount: f64,
    pub status: String,
}

#[function_component(BudgetManagementPage)]
pub fn budget_management_page() -> Html {
    let budget_items = use_state(|| Vec::<BudgetItem>::new());
    let on_print = Callback::from(|_: yew::MouseEvent| {
        if let Some(win) = window() {
            let _ = win.print();
        }
    });


    {
        let budget_items = budget_items.clone();
        use_effect_with((), move |_| {
            budget_items.set(vec![
                BudgetItem {
                    id: 1,
                    name: "二季度采购预算".to_string(),
                    department: "采购部".to_string(),
                    amount: 500000.00,
                    status: "已审批".to_string(),
                },
                BudgetItem {
                    id: 2,
                    name: "纺纱车间设备维护预算".to_string(),
                    department: "生产部".to_string(),
                    amount: 150000.00,
                    status: "待审批".to_string(),
                },
                BudgetItem {
                    id: 3,
                    name: "年度研发预算".to_string(),
                    department: "研发部".to_string(),
                    amount: 300000.00,
                    status: "执行中".to_string(),
                },
            ]);
            || ()
        });
    }

    html! {
        <MainLayout current_page={"budget_management"}>
            <div class="budget-management-page p-4">
                <div class="header mb-4 flex justify-between items-center">
                    <h1 class="text-2xl font-bold">{"预算管理"}</h1>
                    <button class="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded">
                        {"新增预算"}
                    </button>
                </div>
                
                <div class="grid-form mb-6 grid grid-cols-1 md:grid-cols-4 gap-4 bg-white p-4 rounded shadow">
                    <div class="form-group">
                        <label class="block text-sm font-medium text-gray-700 mb-1">{"预算名称"}</label>
                        <input type="text" class="w-full border rounded px-3 py-2" placeholder="请输入预算名称" />
                    </div>
                    <div class="form-group">
                        <label class="block text-sm font-medium text-gray-700 mb-1">{"申请部门"}</label>
                        <input type="text" class="w-full border rounded px-3 py-2" placeholder="请输入部门" />
                    </div>
                    <div class="form-group">
                        <label class="block text-sm font-medium text-gray-700 mb-1">{"状态"}</label>
                        <select class="w-full border rounded px-3 py-2">
                            <option>{"全部"}</option>
                            <option>{"已审批"}</option>
                            <option>{"待审批"}</option>
                            <option>{"执行中"}</option>
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
                        <div class="overflow-x-auto w-full pb-4">
<table class="data-table w-full">
                            <thead class="bg-gray-50 border-b">
                                <tr>
                                    <th class="px-4 py-3 text-left">{"ID"}</th>
                                    <th class="px-4 py-3 text-left">{"预算名称"}</th>
                                    <th class="px-4 py-3 text-left">{"部门"}</th>
                                    <th class="px-4 py-3 text-right">{"预算金额"}</th>
                                    <th class="px-4 py-3 text-center">{"状态"}</th>
                                    <th class="px-4 py-3 text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {
                                    if budget_items.is_empty() {
                                        html! {
                                            <tr><td colspan="6" class="text-center py-4">{"暂无数据"}</td></tr>
                                        }
                                    } else {
                                        html! {
                                            for budget_items.iter().map(|item| html! {
                                                <tr key={item.id} class="border-b hover:bg-gray-50">
                                                    <td class="px-4 py-3">{ item.id }</td>
                                                    <td class="px-4 py-3">{ &item.name }</td>
                                                    <td class="px-4 py-3">{ &item.department }</td>
                                                    <td class="numeric-cell text-right px-4 py-3">{ format!("{:.2}", item.amount) }</td>
                                                    <td class="px-4 py-3 text-center">
                                                        <span class="status-badge px-2 py-1 rounded text-xs bg-blue-100 text-blue-800">{ &item.status }</span>
                                                    </td>
                                                    <td class="px-4 py-3 text-center">
                                                        <button class="text-blue-500 hover:text-blue-700 mr-2">{"编辑"}</button>
                                                        <button class="text-red-500 hover:text-red-700">{"删除"}</button>
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
            </div>
        </MainLayout>
    }
}
