//! 会计科目管理页面

use yew::prelude::*;
use crate::components::main_layout::MainLayout;

#[derive(Clone, PartialEq)]
pub struct SubjectItem {
    pub id: u32,
    pub code: String,
    pub name: String,
    pub category: String,
    pub direction: String,
    pub status: String,
}

#[function_component(AccountSubjectPage)]
pub fn account_subject_page() -> Html {
    let subject_items = use_state(|| Vec::<SubjectItem>::new());

    {
        let subject_items = subject_items.clone();
        use_effect_with((), move |_| {
            subject_items.set(vec![
                SubjectItem {
                    id: 1,
                    code: "6001".to_string(),
                    name: "主营业务收入".to_string(),
                    category: "损益类".to_string(),
                    direction: "贷".to_string(),
                    status: "启用".to_string(),
                },
                SubjectItem {
                    id: 2,
                    code: "1403".to_string(),
                    name: "原材料-棉纱".to_string(),
                    category: "资产类".to_string(),
                    direction: "借".to_string(),
                    status: "启用".to_string(),
                },
                SubjectItem {
                    id: 3,
                    code: "1405".to_string(),
                    name: "库存商品-坯布".to_string(),
                    category: "资产类".to_string(),
                    direction: "借".to_string(),
                    status: "启用".to_string(),
                },
            ]);
            || ()
        });
    }

    html! {
        <MainLayout current_page={"account_subject"}>
            <div class="account-subject-page p-4">
                <div class="header mb-4 flex justify-between items-center">
                    <h1 class="text-2xl font-bold">{"会计科目管理"}</h1>
                    <button class="bg-indigo-500 hover:bg-indigo-600 text-white px-4 py-2 rounded">
                        {"新增科目"}
                    </button>
                </div>
                
                <div class="grid-form mb-6 grid grid-cols-1 md:grid-cols-4 gap-4 bg-white p-4 rounded shadow">
                    <div class="form-group">
                        <label class="block text-sm font-medium text-gray-700 mb-1">{"科目代码"}</label>
                        <input type="text" class="w-full border rounded px-3 py-2" placeholder="请输入科目代码" />
                    </div>
                    <div class="form-group">
                        <label class="block text-sm font-medium text-gray-700 mb-1">{"科目名称"}</label>
                        <input type="text" class="w-full border rounded px-3 py-2" placeholder="请输入科目名称" />
                    </div>
                    <div class="form-group">
                        <label class="block text-sm font-medium text-gray-700 mb-1">{"科目类别"}</label>
                        <select class="w-full border rounded px-3 py-2">
                            <option>{"全部"}</option>
                            <option>{"资产类"}</option>
                            <option>{"负债类"}</option>
                            <option>{"权益类"}</option>
                            <option>{"成本类"}</option>
                            <option>{"损益类"}</option>
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
                                    <th class="px-4 py-3 text-left">{"科目代码"}</th>
                                    <th class="px-4 py-3 text-left">{"科目名称"}</th>
                                    <th class="px-4 py-3 text-left">{"类别"}</th>
                                    <th class="px-4 py-3 text-center">{"余额方向"}</th>
                                    <th class="px-4 py-3 text-center">{"状态"}</th>
                                    <th class="px-4 py-3 text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {
                                    if subject_items.is_empty() {
                                        html! {
                                            <tr><td colspan="7" class="text-center py-4">{"暂无数据"}</td></tr>
                                        }
                                    } else {
                                        html! {
                                            for subject_items.iter().map(|item| html! {
                                                <tr key={item.id} class="border-b hover:bg-gray-50">
                                                    <td class="px-4 py-3">{ item.id }</td>
                                                    <td class="px-4 py-3">{ &item.code }</td>
                                                    <td class="px-4 py-3">{ &item.name }</td>
                                                    <td class="px-4 py-3">{ &item.category }</td>
                                                    <td class="px-4 py-3 text-center">
                                                        <span class="status-badge px-2 py-1 rounded text-xs bg-yellow-100 text-yellow-800">{ &item.direction }</span>
                                                    </td>
                                                    <td class="px-4 py-3 text-center">
                                                        <span class="status-badge px-2 py-1 rounded text-xs bg-indigo-100 text-indigo-800">{ &item.status }</span>
                                                    </td>
                                                    <td class="px-4 py-3 text-center">
                                                        <button class="text-blue-500 hover:text-blue-700 mr-2">{"编辑"}</button>
                                                        <button class="text-red-500 hover:text-red-700">{"停用"}</button>
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
