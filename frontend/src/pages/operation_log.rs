use crate::components::main_layout::MainLayout;
use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct LogResponse {
    pub id: i32,
    pub user_id: i32,
    pub action: String,
    pub details: String,
}

#[function_component(OperationLogPage)]
pub fn operation_log_page() -> Html {
    let logs = use_state(|| Vec::<LogResponse>::new());
    let is_loading = use_state(|| true);

    {
        let logs = logs.clone();
        let is_loading = is_loading.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(res) = ApiService::get::<Vec<LogResponse>>("/api/v1/erp/operation-logs").await {
                    logs.set(res);
                }
                is_loading.set(false);
            });
            || ()
        });
    }

    let on_print = Callback::from(|_: yew::MouseEvent| {
        if let Some(win) = web_sys::window() {
            let _ = win.print();
        }
    });

    html! {
        <MainLayout current_page="操作日志">
            <div class="card p-4 md:p-6">
                <div class="flex flex-col md:flex-row justify-between items-start md:items-center mb-6 gap-4">
                    <h2 class="text-xl font-bold text-slate-800">{"系统操作日志"}</h2>
                    <div class="flex gap-2 w-full md:w-auto">
                        <button onclick={on_print} class="btn-outline w-full md:w-auto text-slate-600 border-slate-300">{"🖨️ 打印"}</button>
                    </div>
                </div>

                if *is_loading {
                    <div class="skeleton skeleton-row"></div>
                    <div class="skeleton skeleton-row mt-2"></div>
                } else {
                    <div class="table-responsive overflow-x-auto w-full pb-4 shadow-sm sm:rounded-lg">
                        <table class="data-table w-full text-left text-sm text-slate-600">
                            <thead class="bg-slate-50 text-slate-700">
                                <tr>
                                    <th class="px-4 py-3 font-semibold">{"日志ID"}</th>
                                    <th class="px-4 py-3 font-semibold">{"操作用户ID"}</th>
                                    <th class="px-4 py-3 font-semibold">{"动作类型"}</th>
                                    <th class="px-4 py-3 font-semibold">{"详情"}</th>
                                </tr>
                            </thead>
                            <tbody class="divide-y divide-slate-200">
                                {for logs.iter().map(|log| html! {
                                    <tr class="hover:bg-slate-50 transition-colors">
                                        <td class="px-4 py-3 font-medium text-slate-900">{log.id}</td>
                                        <td class="px-4 py-3">{log.user_id}</td>
                                        <td class="px-4 py-3">
                                            <span class="status-badge bg-blue-100 text-blue-800">{&log.action}</span>
                                        </td>
                                        <td class="px-4 py-3 text-xs font-mono truncate max-w-xs">{&log.details}</td>
                                    </tr>
                                })}
                                if logs.is_empty() {
                                    <tr>
                                        <td colspan="4" class="px-4 py-8 text-center text-slate-500">{"暂无日志记录"}</td>
                                    </tr>
                                }
                            </tbody>
                        </table>
                    </div>
                }
            </div>
        </MainLayout>
    }
}
