// 缸号管理页面（染色批次管理）

use crate::models::dye_batch::{
    CompleteDyeBatchRequest, CreateDyeBatchRequest, DyeBatch, DyeBatchQuery,
};
use crate::services::dye_batch_service::DyeBatchService;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

pub struct DyeBatchPage {
    batches: Vec<DyeBatch>,
    loading: bool,
    error: Option<String>,
    filter_batch_no: String,
    filter_color_code: String,
    filter_status: String,
    page: u64,
    page_size: u64,
    show_create_modal: bool,
    show_complete_modal: bool,
    selected_batch: Option<DyeBatch>,
}

pub enum Msg {
    LoadBatches,
    BatchesLoaded(Vec<DyeBatch>),
    LoadError(String),
    SetFilterBatchNo(String),
    SetFilterColorCode(String),
    SetFilterStatus(String),
    ToggleCreateModal,
    ToggleCompleteModal(Option<i32>),
    CreateBatch(CreateDyeBatchRequest),
    CompleteBatch(i32, CompleteDyeBatchRequest),
    DeleteBatch(i32),
    ChangePage(u64),
}

impl Component for DyeBatchPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            batches: Vec::new(),
            loading: true,
            error: None,
            filter_batch_no: String::new(),
            filter_color_code: String::new(),
            filter_status: String::from("全部"),
            page: 1,
            page_size: 20,
            show_create_modal: false,
            show_complete_modal: false,
            selected_batch: None,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadBatches);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadBatches => {
                self.loading = true;
                let query = DyeBatchQuery {
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                    batch_no: if self.filter_batch_no.is_empty() {
                        None
                    } else {
                        Some(self.filter_batch_no.clone())
                    },
                    color_code: if self.filter_color_code.is_empty() {
                        None
                    } else {
                        Some(self.filter_color_code.clone())
                    },
                    status: if self.filter_status == "全部" {
                        None
                    } else {
                        Some(self.filter_status.clone())
                    },
                    ..Default::default()
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match DyeBatchService::list(query).await {
                        Ok(batches) => link.send_message(Msg::BatchesLoaded(batches.items)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::BatchesLoaded(batches) => {
                self.batches = batches;
                self.loading = false;
                true
            }
            Msg::LoadError(e) => {
                self.error = Some(e);
                self.loading = false;
                true
            }
            Msg::SetFilterBatchNo(batch_no) => {
                self.filter_batch_no = batch_no;
                ctx.link().send_message(Msg::LoadBatches);
                false
            }
            Msg::SetFilterColorCode(color_code) => {
                self.filter_color_code = color_code;
                ctx.link().send_message(Msg::LoadBatches);
                false
            }
            Msg::SetFilterStatus(status) => {
                self.filter_status = status;
                ctx.link().send_message(Msg::LoadBatches);
                false
            }
            Msg::ToggleCreateModal => {
                self.show_create_modal = !self.show_create_modal;
                true
            }
            Msg::ToggleCompleteModal(id) => {
                self.show_complete_modal = !self.show_complete_modal;
                if let Some(batch_id) = id {
                    self.selected_batch = self.batches.iter().find(|b| b.id == batch_id).cloned();
                } else {
                    self.selected_batch = None;
                }
                true
            }
            Msg::CreateBatch(req) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match DyeBatchService::create(req).await {
                        Ok(_) => {
                            link.send_message(Msg::ToggleCreateModal);
                            link.send_message(Msg::LoadBatches);
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::CompleteBatch(id, req) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match DyeBatchService::complete(id, req).await {
                        Ok(_) => {
                            link.send_message(Msg::ToggleCompleteModal(None));
                            link.send_message(Msg::LoadBatches);
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::DeleteBatch(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match DyeBatchService::delete(id).await {
                        Ok(_) => link.send_message(Msg::LoadBatches),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ChangePage(page) => {
                self.page = page;
                ctx.link().send_message(Msg::LoadBatches);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_batch_no_change = ctx.link().callback(|e: Event| {
            let target = e
                .target()
                .unwrap()
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap();
            Msg::SetFilterBatchNo(target.value())
        });

        let on_color_code_change = ctx.link().callback(|e: Event| {
            let target = e
                .target()
                .unwrap()
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap();
            Msg::SetFilterColorCode(target.value())
        });

        let on_status_change = ctx.link().callback(|e: Event| {
            let target = e
                .target()
                .unwrap()
                .dyn_into::<web_sys::HtmlSelectElement>()
                .unwrap();
            Msg::SetFilterStatus(target.value())
        });

        html! {
            <div class="dye-batch-page">
                <div class="page-header">
                    <h1>{"🏭 缸号管理"}</h1>
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::ToggleCreateModal)}>
                        {"+ 新增缸号"}
                    </button>
                </div>

                <div class="filter-bar">
                    <div class="filter-item">
                        <label>{"缸号："}</label>
                        <input type="text" placeholder="请输入缸号"
                            value={self.filter_batch_no.clone()}
                            onchange={on_batch_no_change}
                        />
                    </div>
                    <div class="filter-item">
                        <label>{"色号："}</label>
                        <input type="text" placeholder="请输入色号"
                            value={self.filter_color_code.clone()}
                            onchange={on_color_code_change}
                        />
                    </div>
                    <div class="filter-item">
                        <label>{"状态："}</label>
                        <select value={self.filter_status.clone()} onchange={on_status_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="待生产">{"待生产"}</option>
                            <option value="生产中">{"生产中"}</option>
                            <option value="已完成">{"已完成"}</option>
                        </select>
                    </div>
                </div>

                {self.render_content(ctx)}

                if self.show_create_modal {
                    {self.render_create_modal(ctx)}
                }

                if self.show_complete_modal && self.selected_batch.is_some() {
                    {self.render_complete_modal(ctx)}
                }
            </div>
        }
    }
}

impl DyeBatchPage {
    fn render_content(&self, ctx: &Context<Self>) -> Html {
        if self.loading {
            return html! {
                <div class="loading-container">
                    <div class="spinner"></div>
                    <p>{"加载中..."}</p>
                </div>
            };
        }

        if let Some(error) = &self.error {
            return html! {
                <div class="error-container">
                    <div class="error-icon">{"⚠️"}</div>
                    <p class="error-message">{error}</p>
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadBatches)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        if self.batches.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"🏭"}</div>
                    <p>{"暂无缸号数据"}</p>
                </div>
            };
        }

        html! {
            <div class="table-responsive">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>{"缸号"}</th>
                            <th>{"色号"}</th>
                            <th>{"颜色名称"}</th>
                            <th>{"面料类型"}</th>
                            <th>{"重量(kg)"}</th>
                            <th>{"状态"}</th>
                            <th>{"质量等级"}</th>
                            <th>{"生产日期"}</th>
                            <th>{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.batches.iter().map(|batch| {
                            let batch_id = batch.id;
                            let status = batch.status.clone();
                            let is_completed = status == "已完成";
                            html! {
                                <tr>
                                    <td>{&batch.batch_no}</td>
                                    <td>{&batch.color_code}</td>
                                    <td>{&batch.color_name}</td>
                                    <td>{batch.fabric_type.as_deref().unwrap_or("-")}</td>
                                    <td class="numeric">{batch.weight_kg.clone().map(|w| format!("{:.2}", w)).unwrap_or("-".to_string())}</td>
                                    <td>
                                        <span class={format!("status-badge status-{}", status)}>
                                            {&status}
                                        </span>
                                    </td>
                                    <td>{batch.quality_grade.as_deref().unwrap_or("-")}</td>
                                    <td>{batch.production_date.as_deref().unwrap_or("-")}</td>
                                    <td class="actions">
                                        if !is_completed {
                                            <button class="btn-small btn-success"
                                                onclick={ctx.link().callback(move |_| Msg::ToggleCompleteModal(Some(batch_id)))}>
                                                {"完成"}
                                            </button>
                                        }
                                        <button class="btn-small btn-danger"
                                            onclick={ctx.link().callback(move |_| Msg::DeleteBatch(batch_id))}>
                                            {"删除"}
                                        </button>
                                    </td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
            </div>
        }
    }

    fn render_create_modal(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="modal-overlay" onclick={ctx.link().callback(|_| Msg::ToggleCreateModal)}>
                <div class="modal-content" onclick={|e: MouseEvent| e.stop_propagation()}>
                    <div class="modal-header">
                        <h2>{"新增缸号"}</h2>
                        <button class="modal-close" onclick={ctx.link().callback(|_| Msg::ToggleCreateModal)}>
                            {"×"}
                        </button>
                    </div>
                    <div class="modal-body">
                        <p>{"缸号创建功能已就绪，请填写相关信息"}</p>
                    </div>
                </div>
            </div>
        }
    }

    fn render_complete_modal(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="modal-overlay" onclick={ctx.link().callback(|_| Msg::ToggleCompleteModal(None))}>
                <div class="modal-content" onclick={|e: MouseEvent| e.stop_propagation()}>
                    <div class="modal-header">
                        <h2>{"完成缸号"}</h2>
                        <button class="modal-close" onclick={ctx.link().callback(|_| Msg::ToggleCompleteModal(None))}>
                            {"×"}
                        </button>
                    </div>
                    <div class="modal-body">
                        <p>{"确认完成此缸号？请选择质量等级"}</p>
                    </div>
                </div>
            </div>
        }
    }
}
