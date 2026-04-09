use yew::prelude::*;
use web_sys::window;
use crate::components::main_layout::MainLayout;

#[derive(Clone, PartialEq, Default)]
pub struct ArVerifyItem {
    pub id: i32,
    pub verify_no: String,
    pub batch_no: String,
    pub customer_name: String,
    pub amount: f64,
    pub status: String,
    pub description: String,
}

#[function_component(ArVerificationPage)]
pub fn ar_verification_page() -> Html {
    let items = use_state(Vec::<ArVerifyItem>::new);
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
                ArVerifyItem {
                    id: 1,
                    verify_no: "VR-20231001-0001".to_string(),
                    batch_no: "-".to_string(),
                    customer_name: "江苏服饰厂".to_string(),
                    amount: 50000.0,
                    status: "已核销".to_string(),
                    description: "收到江苏服饰厂尾款".to_string(),
                }
            ]);
            || ()
        });
    }

    html! {
        <MainLayout current_page={"ar_verification"}>
            <div class="ar-verification-page p-4">
                <div class="page-header flex justify-between items-center mb-4">
                    <h1 class="text-2xl font-bold">{"应收核销"}</h1>
                    <button class="btn-primary px-4 py-2 bg-blue-500 text-white rounded">{"新增核销"}</button>
                </div>
                
                <div class="grid-form mb-6 p-4 border rounded bg-white shadow-sm">
                    <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
                        <div class="form-item flex flex-col">
                            <label class="mb-1 text-sm text-gray-600">{"核销单号"}</label>
                            <input type="text" class="input-field border rounded px-3 py-2" placeholder="请输入核销单号" />
                        </div>
                        <div class="form-item flex flex-col">
                            <label class="mb-1 text-sm text-gray-600">{"客户名称"}</label>
                            <input type="text" class="input-field border rounded px-3 py-2" placeholder="请输入客户名称" />
                        </div>
                        <div class="form-item flex flex-col">
                            <label class="mb-1 text-sm text-gray-600">{"关联批次/发货单"}</label>
                            <input type="text" class="input-field border rounded px-3 py-2" placeholder="请输入批次/发货单号" />
                        </div>
                        <div class="form-item flex items-end">
                            <button class="btn-secondary px-4 py-2 border rounded bg-gray-100 w-full">{"查询"}</button>
                        </div>
                    </div>
                </div>

                <div class="table-container bg-white rounded shadow-sm overflow-x-auto">
                    <div class="overflow-x-auto w-full pb-4">
<table class="data-table w-full text-left border-collapse">
                        <thead class="bg-gray-50 border-b">
                            <tr>
                                <th class="p-3">{"ID"}</th>
                                <th class="p-3">{"核销单号"}</th>
                                <th class="p-3">{"关联批次/发货单"}</th>
                                <th class="p-3">{"客户名称"}</th>
                                <th class="p-3 text-right">{"核销金额"}</th>
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
                                        <td class="p-3">{&item.verify_no}</td>
                                        <td class="p-3">{&item.batch_no}</td>
                                        <td class="p-3">{&item.customer_name}</td>
                                        <td class="numeric-cell text-right p-3 font-mono">{format!("{:.2}", item.amount)}</td>
                                        <td class="p-3"><span class="status-badge px-2 py-1 rounded text-sm bg-green-100 text-green-800">{&item.status}</span></td>
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
