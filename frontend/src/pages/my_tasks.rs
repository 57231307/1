use yew::prelude::*;
use serde::{Deserialize, Serialize};
use crate::services::api::ApiService;
use crate::utils::toast_helper;
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct BpmTask {
    pub id: i32,
    pub process_instance_id: i32,
    pub task_no: String,
    pub node_name: String,
    pub name: String,
    pub status: String,
    pub business_type: Option<String>,
    pub business_id: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PaginatedTasks {
    pub data: Vec<BpmTask>,
    pub total: u64,
}

pub enum Msg {
    LoadTasks,
    TasksLoaded(Result<PaginatedTasks, String>),
    OpenApprovalModal(BpmTask),
    CloseApprovalModal,
    UpdateOpinion(String),
    SubmitApproval(String), // "approve" or "reject"
    ApprovalSubmitted(Result<(), String>),
    ChangeTab(String), // "PENDING", "COMPLETED"
}

pub struct MyTasksPage {
    tasks: Vec<BpmTask>,
    total: u64,
    loading: bool,
    current_tab: String,
    
    // Approval Modal State
    show_modal: bool,
    selected_task: Option<BpmTask>,
    approval_opinion: String,
}

impl Component for MyTasksPage {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::LoadTasks);
        Self {
            tasks: vec![],
            total: 0,
            loading: false,
            current_tab: "PENDING".to_string(),
            show_modal: false,
            selected_task: None,
            approval_opinion: String::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadTasks => {
                self.loading = true;
                let status = self.current_tab.clone();
                // user_id will be extracted from token by backend usually, but our API requires user_id in query.
                // For now, let's hardcode user_id=2 (or whatever is the current user).
                // Actually, the auth context should provide user_id. Let's get it from storage if possible, or just pass 2 for demo.
                let user_id = crate::utils::storage::Storage::get_item("user_id").unwrap_or_else(|| "2".to_string());
                
                ctx.link().send_future(async move {
                    let url = format!("/bpm/tasks?user_id={}&status={}", user_id, status);
                    let result = ApiService::get::<PaginatedTasks>(&url).await;
                    Msg::TasksLoaded(result)
                });
                true
            }
            Msg::TasksLoaded(Ok(data)) => {
                self.loading = false;
                self.tasks = data.data;
                self.total = data.total;
                true
            }
            Msg::TasksLoaded(Err(e)) => {
                self.loading = false;
                toast_helper::show_error(&format!("加载待办失败: {}", e));
                true
            }
            Msg::ChangeTab(tab) => {
                self.current_tab = tab;
                ctx.link().send_message(Msg::LoadTasks);
                true
            }
            Msg::OpenApprovalModal(task) => {
                self.selected_task = Some(task);
                self.approval_opinion = String::new();
                self.show_modal = true;
                true
            }
            Msg::CloseApprovalModal => {
                self.show_modal = false;
                self.selected_task = None;
                true
            }
            Msg::UpdateOpinion(val) => {
                self.approval_opinion = val;
                true
            }
            Msg::SubmitApproval(action) => {
                if let Some(task) = &self.selected_task {
                    let user_id: i32 = crate::utils::storage::Storage::get_item("user_id")
                        .unwrap_or_else(|| "2".to_string())
                        .parse()
                        .unwrap_or(2);
                        
                    let req = serde_json::json!({
                        "task_id": task.id,
                        "handler_id": user_id,
                        "handler_name": "CurrentUser", // should be from context
                        "action": action,
                        "approval_opinion": self.approval_opinion,
                    });
                    
                    ctx.link().send_future(async move {
                        let result = ApiService::post::<String, _>("/bpm/tasks/approve", &req).await;
                        Msg::ApprovalSubmitted(result.map(|_| ()))
                    });
                }
                false
            }
            Msg::ApprovalSubmitted(Ok(_)) => {
                self.show_modal = false;
                toast_helper::show_success("审批提交成功");
                ctx.link().send_message(Msg::LoadTasks);
                true
            }
            Msg::ApprovalSubmitted(Err(e)) => {
                toast_helper::show_error(&format!("审批失败: {}", e));
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="p-6">
                <div class="flex justify-between items-center mb-6">
                    <h1 class="text-2xl font-bold text-gray-800">{"我的待办"}</h1>
                </div>
                
                <div class="mb-4 border-b border-gray-200">
                    <ul class="flex flex-wrap -mb-px">
                        <li class="mr-2">
                            <button 
                                class={format!("inline-block p-4 border-b-2 rounded-t-lg {}", if self.current_tab == "PENDING" { "border-blue-600 text-blue-600" } else { "border-transparent hover:text-gray-600 hover:border-gray-300" })}
                                onclick={ctx.link().callback(|_| Msg::ChangeTab("PENDING".to_string()))}>
                                {"待处理"}
                            </button>
                        </li>
                        <li class="mr-2">
                            <button 
                                class={format!("inline-block p-4 border-b-2 rounded-t-lg {}", if self.current_tab == "COMPLETED" { "border-blue-600 text-blue-600" } else { "border-transparent hover:text-gray-600 hover:border-gray-300" })}
                                onclick={ctx.link().callback(|_| Msg::ChangeTab("COMPLETED".to_string()))}>
                                {"已处理"}
                            </button>
                        </li>
                    </ul>
                </div>

                <div class="bg-white rounded-lg shadow overflow-hidden">
                    <table class="min-w-full divide-y divide-gray-200">
                        <thead class="bg-gray-50">
                            <tr>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"任务编号"}</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"业务类型"}</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"任务节点"}</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"创建时间"}</th>
                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody class="bg-white divide-y divide-gray-200">
                            { for self.tasks.iter().map(|task| {
                                let t = task.clone();
                                html! {
                                    <tr>
                                        <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{ &task.task_no }</td>
                                        <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                                            { task.business_type.as_deref().unwrap_or("-") } 
                                            { format!(" #{}", task.business_id.unwrap_or(0)) }
                                        </td>
                                        <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{ &task.name }</td>
                                        <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{ task.created_at.format("%Y-%m-%d %H:%M").to_string() }</td>
                                        <td class="px-6 py-4 whitespace-nowrap text-sm font-medium">
                                            if self.current_tab == "PENDING" {
                                                <button 
                                                    class="text-indigo-600 hover:text-indigo-900"
                                                    onclick={ctx.link().callback(move |_| Msg::OpenApprovalModal(t.clone()))}>
                                                    {"去审批"}
                                                </button>
                                            } else {
                                                <span class="text-gray-400">{"已完成"}</span>
                                            }
                                        </td>
                                    </tr>
                                }
                            }) }
                        </tbody>
                    </table>
                    if self.tasks.is_empty() && !self.loading {
                        <div class="p-6 text-center text-gray-500">{"暂无数据"}</div>
                    }
                </div>

                // Approval Modal
                if self.show_modal {
                    <div class="fixed inset-0 z-50 overflow-y-auto">
                        <div class="flex items-end justify-center min-h-screen pt-4 px-4 pb-20 text-center sm:block sm:p-0">
                            <div class="fixed inset-0 transition-opacity" aria-hidden="true">
                                <div class="absolute inset-0 bg-gray-500 opacity-75"></div>
                            </div>
                            <span class="hidden sm:inline-block sm:align-middle sm:h-screen" aria-hidden="true">{"\u{200b}"}</span>
                            <div class="inline-block align-bottom bg-white rounded-lg text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-lg sm:w-full">
                                <div class="bg-white px-4 pt-5 pb-4 sm:p-6 sm:pb-4">
                                    <h3 class="text-lg leading-6 font-medium text-gray-900 mb-4">{"审批处理"}</h3>
                                    if let Some(task) = &self.selected_task {
                                        <div class="mb-4 text-sm text-gray-600">
                                            <p>{"任务: "}{ &task.name }</p>
                                            <p>{"单据: "}{ task.business_type.as_deref().unwrap_or("-") }{ " #" }{ task.business_id.unwrap_or(0) }</p>
                                        </div>
                                    }
                                    <div class="mt-2">
                                        <label class="block text-sm font-medium text-gray-700">{"审批意见"}</label>
                                        <textarea 
                                            class="mt-1 block w-full border border-gray-300 rounded-md shadow-sm p-2" 
                                            rows="3"
                                            value={self.approval_opinion.clone()}
                                            oninput={ctx.link().callback(|e: InputEvent| {
                                                use wasm_bindgen::JsCast;
                                                let input = e.target().unwrap().unchecked_into::<web_sys::HtmlTextAreaElement>();
                                                Msg::UpdateOpinion(input.value())
                                            })}
                                            placeholder="请输入审批意见..."></textarea>
                                    </div>
                                </div>
                                <div class="bg-gray-50 px-4 py-3 sm:px-6 sm:flex sm:flex-row-reverse">
                                    <button 
                                        type="button" 
                                        class="w-full inline-flex justify-center rounded-md border border-transparent shadow-sm px-4 py-2 bg-blue-600 text-base font-medium text-white hover:bg-blue-700 sm:ml-3 sm:w-auto sm:text-sm"
                                        onclick={ctx.link().callback(|_| Msg::SubmitApproval("approve".to_string()))}>
                                        {"同意"}
                                    </button>
                                    <button 
                                        type="button" 
                                        class="mt-3 w-full inline-flex justify-center rounded-md border border-gray-300 shadow-sm px-4 py-2 bg-red-600 text-base font-medium text-white hover:bg-red-700 sm:mt-0 sm:ml-3 sm:w-auto sm:text-sm"
                                        onclick={ctx.link().callback(|_| Msg::SubmitApproval("reject".to_string()))}>
                                        {"驳回"}
                                    </button>
                                    <button 
                                        type="button" 
                                        class="mt-3 w-full inline-flex justify-center rounded-md border border-gray-300 shadow-sm px-4 py-2 bg-white text-base font-medium text-gray-700 hover:bg-gray-50 sm:mt-0 sm:w-auto sm:text-sm"
                                        onclick={ctx.link().callback(|_| Msg::CloseApprovalModal)}>
                                        {"取消"}
                                    </button>
                                </div>
                            </div>
                        </div>
                    </div>
                }
            </div>
        }
    }
}
