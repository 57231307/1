use yew::prelude::*;
use web_sys::window;
use crate::components::main_layout::MainLayout;

#[derive(Clone, PartialEq, Default)]
pub struct ArReceiptItem {
    pub id: i32,
    pub receipt_no: String,
    pub customer_name: String,
    pub amount: f64,
    pub status: String,
    pub description: String,
}

#[function_component(ArReceiptPage)]
pub fn ar_receipt_page() -> Html {
    let items = use_state(Vec::<ArReceiptItem>::new);
    let on_print = Callback::from(|_: yew::MouseEvent| {
        if let Some(win) = window() {
            let _ = win.print();
        }
    });

    
    {
        let items = items.clone();
        use_effect_with((), move |_| {
            // 初始化数据
            items.set(vec![
                ArReceiptItem {
                    id: 1,
                    receipt_no: "RC-20231001-0001".to_string(),
                    customer_name: "江苏服饰厂".to_string(),
                    amount: 50000.0,
                    status: "已收款".to_string(),
                    description: "收到江苏服饰厂尾款".to_string(),
                }
            ]);
            || ()
        });
    }

    html! {
        <MainLayout current_page={"ar_receipt"}>
            <div class="ar-receipt-page p-4">
                <div class="page-header flex justify-between items-center mb-4">
                    <h1 class="text-2xl font-bold">{"收款单"}</h1>
                    <button class="btn-primary px-4 py-2 bg-blue-500 text-white rounded">{"新增收款"}</button>
                </div>
                
                <div class="grid-form mb-6 p-4 border rounded bg-white shadow-sm">
                    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                        <div class="form-item flex flex-col">
                            <label class="mb-1 text-sm text-gray-600">{"收款单号"}</label>
                            <input type="text" class="input-field border rounded px-3 py-2" placeholder="请输入收款单号" />
                        </div>
                        <div class="form-item flex flex-col">
                            <label class="mb-1 text-sm text-gray-600">{"客户名称"}</label>
                            <input type="text" class="input-field border rounded px-3 py-2" placeholder="请输入客户名称" />
                        </div>
                        <div class="form-item flex items-end">
                            <button class="btn-secondary px-4 py-2 border rounded bg-gray-100 w-full">{"查询"}</button>
                        </div>
                    </div>
                </div>

                <div class="table-container bg-white rounded shadow-sm overflow-x-auto">
                    <div class="table-responsive overflow-x-auto w-full pb-4 shadow-sm sm:rounded-lg">
<table class="data-table w-full text-left border-collapse">
                        <thead class="bg-gray-50 border-b">
                            <tr>
                                <th class="p-3">{"ID"}</th>
                                <th class="p-3">{"收款单号"}</th>
                                <th class="p-3">{"客户名称"}</th>
                                <th class="p-3 text-right">{"收款金额"}</th>
                                <th class="p-3">{"状态"}</th>
                                <th class="p-3">{"备注"}</th>
                                <th class="p-3">{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {for items.iter().map(|item| {
                                html! {
                                    <tr class="border-b hover:bg-gray-50">
                                        <td class="p-3">{item.id}</td>
                                        <td class="p-3">{&item.receipt_no}</td>
                                        <td class="p-3">{&item.customer_name}</td>
                                        <td class="numeric-cell text-right p-3 font-mono">{format!("{:.2}", item.amount)}</td>
                                        <td class="p-3"><span class="status-badge px-2 py-1 rounded text-sm bg-blue-100 text-blue-800">{&item.status}</span></td>
                                        <td class="p-3">{&item.description}</td>
                                        <td class="p-3"><button class="text-blue-500 hover:underline">{"查看"}</button></td>
                                    </tr>
                                }
                            })}
                        </tbody>
                    </table>
</div>
                </div>
            </div>
        </MainLayout>
    }
}
