// 批次管理页面

use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::models::batch::{
    Batch, BatchQuery,
};
use crate::services::batch_service::BatchService;

pub struct BatchPage {
    batches: Vec<Batch>,
    loading: bool,
    error: Option<String>,
    filter_grade: String,
    filter_batch_no: String,
    filter_color_no: String,
    page: u64,
    page_size: u64,
}

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    View,
    Create,
    Edit,
    Transfer,
}

pub enum Msg {
    LoadBatches,
    BatchesLoaded(Vec<Batch>),
    LoadError(String),
    SetFilterGrade(String),
    SetFilterBatchNo(String),
    SetFilterColorNo(String),
    DeleteBatch(i32),
    ChangePage(u64),
}

impl Component for BatchPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            batches: Vec::new(),
            loading: true,
            error: None,
            filter_grade: String::from("全部"),
            filter_batch_no: String::new(),
            filter_color_no: String::new(),
            page: 1,
            page_size: 20,
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
                let query = BatchQuery {
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                    product_id: None,
                    batch_no: if self.filter_batch_no.is_empty() { None } else { Some(self.filter_batch_no.clone()) },
                    color_no: if self.filter_color_no.is_empty() { None } else { Some(self.filter_color_no.clone()) },
                    grade: if self.filter_grade == "全部" { None } else { Some(self.filter_grade.clone()) },
                    warehouse_id: None,
                    start_date: None,
                    end_date: None,
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match BatchService::list(query).await {
                        Ok(batches) => link.send_message(Msg::BatchesLoaded(batches)),
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
            Msg::SetFilterGrade(grade) => {
                self.filter_grade = grade;
                ctx.link().send_message(Msg::LoadBatches);
                false
            }
            Msg::SetFilterBatchNo(batch_no) => {
                self.filter_batch_no = batch_no;
                ctx.link().send_message(Msg::LoadBatches);
                false
            }
            Msg::SetFilterColorNo(color_no) => {
                self.filter_color_no = color_no;
                ctx.link().send_message(Msg::LoadBatches);
                false
            }
            Msg::DeleteBatch(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match BatchService::delete(id).await {
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
        let on_batch_no_change = ctx.link().batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlInputElement>().ok()?;
            Some(Msg::SetFilterBatchNo(target.value()))
        });

        let on_color_no_change = ctx.link().batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlInputElement>().ok()?;
            Some(Msg::SetFilterColorNo(target.value()))
        });

        let on_grade_change = ctx.link().batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlSelectElement>().ok()?;
            Some(Msg::SetFilterGrade(target.value()))
        });

        html! {
            <div class="batch-page">
                <div class="page-header">
                    <h1>{"🏷️ 批次管理"}</h1>
                </div>

                <div class="filter-bar">
                    <div class="filter-item">
                        <label>{"批次号："}</label>
                        <input type="text" placeholder="请输入批次号"
                            value={self.filter_batch_no.clone()}
                            onchange={on_batch_no_change}
                        />
                    </div>
                    <div class="filter-item">
                        <label>{"色号："}</label>
                        <input type="text" placeholder="请输入色号"
                            value={self.filter_color_no.clone()}
                            onchange={on_color_no_change}
                        />
                    </div>
                    <div class="filter-item">
                        <label>{"等级："}</label>
                        <select value={self.filter_grade.clone()} onchange={on_grade_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="A级">{"A级"}</option>
                            <option value="B级">{"B级"}</option>
                            <option value="C级">{"C级"}</option>
                        </select>
                    </div>
                </div>

                {self.render_content(ctx)}
            </div>
        }
    }
}

impl BatchPage {
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
                    <div class="empty-icon">{"🏷️"}</div>
                    <p>{"暂无批次数据"}</p>
                </div>
            };
        }

        html! {
            <div class="table-responsive">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>{"批次号"}</th>
                            <th>{"产品名称"}</th>
                            <th>{"仓库"}</th>
                            <th>{"色号"}</th>
                            <th>{"等级"}</th>
                            <th>{"数量(米)"}</th>
                            <th>{"数量(公斤)"}</th>
                            <th>{"库存状态"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.batches.iter().map(|batch| {
                            let status = batch.stock_status.clone();
                            html! {
                                <tr>
                                    <td>{&batch.batch_no}</td>
                                    <td>{batch.product_name.as_deref().unwrap_or("-")}</td>
                                    <td>{batch.warehouse_name.as_deref().unwrap_or("-")}</td>
                                    <td>{&batch.color_no}</td>
                                    <td>{&batch.grade}</td>
                                    <td class="numeric">{batch.quantity_meters.to_string()}</td>
                                    <td class="numeric">{batch.quantity_kg.to_string()}</td>
                                    <td>{status}</td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
            </div>
        }
    }
}