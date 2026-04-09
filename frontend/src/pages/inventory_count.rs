//! 库存盘点管理页面
//! 提供库存盘点单的列表、创建、审核、完成等功能

use crate::components::main_layout::MainLayout;
use crate::components::navigation::Navigation;
use crate::models::inventory_count::{
    CreateInventoryCountRequest, InventoryCount, InventoryCountDetail, InventoryCountQuery,
    UpdateInventoryCountRequest,
};
use crate::services::inventory_count_service::InventoryCountService;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

pub struct InventoryCountPage {
    counts: Vec<InventoryCount>,
    loading: bool,
    error: Option<String>,
    filter_status: String,
    filter_warehouse_id: Option<i32>,
    filter_count_no: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    selected_count: Option<InventoryCountDetail>,
    // 创建/编辑表单数据
    form_warehouse_id: i32,
    form_notes: String,
}

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    View,    // 查看详情
    Create,  // 创建新盘点单
    Edit,    // 编辑盘点单
    Approve, // 审核盘点单
}

pub enum Msg {
    LoadCounts,
    CountsLoaded(Vec<InventoryCount>),
    LoadError(String),
    SetFilterStatus(String),
    SetFilterWarehouse(i32),
    SetFilterCountNo(String),
    ChangePage(u64),
    OpenCreateModal,
    OpenViewModal(i32),
    OpenEditModal(i32),
    OpenApproveModal(i32),
    CloseModal,
    ViewCountDetail(InventoryCountDetail),
    CreateCount,
    UpdateCount(i32),
    ApproveCount(i32, bool),
    CompleteCount(i32),
    FormWarehouseChanged(i32),
    FormNotesChanged(String),
}

impl Component for InventoryCountPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            counts: Vec::new(),
            loading: true,
            error: None,
            filter_status: String::from("全部"),
            filter_warehouse_id: None,
            filter_count_no: String::new(),
            page: 1,
            page_size: 20,
            show_modal: false,
            modal_mode: ModalMode::View,
            selected_count: None,
            form_warehouse_id: 0,
            form_notes: String::new(),
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadCounts);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadCounts => {
                self.loading = true;
                let query = InventoryCountQuery {
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                    status: if self.filter_status == "全部" {
                        None
                    } else {
                        Some(self.filter_status.clone())
                    },
                    warehouse_id: self.filter_warehouse_id,
                    count_no: if self.filter_count_no.is_empty() {
                        None
                    } else {
                        Some(self.filter_count_no.clone())
                    },
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryCountService::list(query).await {
                        Ok(counts) => link.send_message(Msg::CountsLoaded(counts)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::CountsLoaded(counts) => {
                self.counts = counts;
                self.loading = false;
                true
            }
            Msg::LoadError(e) => {
                self.error = Some(e);
                self.loading = false;
                true
            }
            Msg::SetFilterStatus(status) => {
                self.filter_status = status;
                self.page = 1;
                ctx.link().send_message(Msg::LoadCounts);
                false
            }
            Msg::SetFilterWarehouse(warehouse_id) => {
                self.filter_warehouse_id = if warehouse_id == 0 {
                    None
                } else {
                    Some(warehouse_id)
                };
                self.page = 1;
                ctx.link().send_message(Msg::LoadCounts);
                false
            }
            Msg::SetFilterCountNo(count_no) => {
                self.filter_count_no = count_no;
                self.page = 1;
                ctx.link().send_message(Msg::LoadCounts);
                false
            }
            Msg::ChangePage(page) => {
                self.page = page;
                ctx.link().send_message(Msg::LoadCounts);
                false
            }
            Msg::OpenCreateModal => {
                self.modal_mode = ModalMode::Create;
                self.selected_count = None;
                self.form_warehouse_id = 0;
                self.form_notes = String::new();
                self.show_modal = true;
                true
            }
            Msg::OpenViewModal(id) => {
                self.modal_mode = ModalMode::View;
                self.show_modal = true;
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryCountService::get(id).await {
                        Ok(detail) => link.send_message(Msg::ViewCountDetail(detail)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ViewCountDetail(detail) => {
                self.selected_count = Some(detail);
                true
            }
            Msg::OpenEditModal(id) => {
                self.modal_mode = ModalMode::Edit;
                self.show_modal = true;
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryCountService::get(id).await {
                        Ok(detail) => link.send_message(Msg::ViewCountDetail(detail)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::OpenApproveModal(id) => {
                self.modal_mode = ModalMode::Approve;
                self.show_modal = true;
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryCountService::get(id).await {
                        Ok(detail) => link.send_message(Msg::ViewCountDetail(detail)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.selected_count = None;
                true
            }
            Msg::CreateCount => {
                if self.form_warehouse_id == 0 {
                    ctx.link()
                        .send_message(Msg::LoadError("请选择仓库".to_string()));
                    return false;
                }
                let req = CreateInventoryCountRequest {
                    warehouse_id: self.form_warehouse_id,
                    count_date: None,
                    status: "pending".to_string(),
                    notes: if self.form_notes.is_empty() {
                        None
                    } else {
                        Some(self.form_notes.clone())
                    },
                    items: None, // 简化版本，实际应从表单获取
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryCountService::create(req).await {
                        Ok(_) => {
                            link.send_message(Msg::CloseModal);
                            link.send_message(Msg::LoadCounts);
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::UpdateCount(id) => {
                let req = UpdateInventoryCountRequest {
                    status: None,
                    notes: if self.form_notes.is_empty() {
                        None
                    } else {
                        Some(self.form_notes.clone())
                    },
                    items: None,
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryCountService::update(id, req).await {
                        Ok(_) => {
                            link.send_message(Msg::CloseModal);
                            link.send_message(Msg::LoadCounts);
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ApproveCount(id, approved) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryCountService::approve(id, approved, None).await {
                        Ok(_) => {
                            link.send_message(Msg::CloseModal);
                            link.send_message(Msg::LoadCounts);
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::CompleteCount(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryCountService::complete(id).await {
                        Ok(_) => link.send_message(Msg::LoadCounts),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::FormWarehouseChanged(warehouse_id) => {
                self.form_warehouse_id = warehouse_id;
                false
            }
            Msg::FormNotesChanged(notes) => {
                self.form_notes = notes;
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <MainLayout current_page={""}>
<div class="inventory-count-page">
                <Navigation current_page="counts" />

                <div class="main-content">
                    <div class="page-header">
                        <h1>{"库存盘点管理"}</h1>
                        <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::OpenCreateModal)}>
                            {"新建盘点单"}
                        </button>
                    </div>

                    <div class="filter-bar">
                        <div class="filter-item">
                            <label>{"盘点单号："}</label>
                            <input
                                type="text"
                                placeholder="请输入盘点单号"
                                value={self.filter_count_no.clone()}
                                oninput={ctx.link().callback(|e: InputEvent| {
                                    let input = e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
                                    Msg::SetFilterCountNo(input.value())
                                })}
                            />
                        </div>
                        <div class="filter-item">
                            <label>{"状态："}</label>
                            <select
                                value={self.filter_status.clone()}
                                onchange={ctx.link().callback(|e: Event| {
                                    let target = e.target().unwrap().dyn_into::<web_sys::HtmlSelectElement>().unwrap();
                                    Msg::SetFilterStatus(target.value())
                                })}
                            >
                                <option value="全部">{"全部"}</option>
                                <option value="pending">{"待审核"}</option>
                                <option value="approved">{"已审核"}</option>
                                <option value="rejected">{"已驳回"}</option>
                                <option value="completed">{"已完成"}</option>
                            </select>
                        </div>
                        <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::LoadCounts)}>
                            {"刷新"}
                        </button>
                    </div>

                    {self.render_content(ctx)}
                </div>

                {if self.show_modal {
                    self.render_modal(ctx)
                } else {
                    html! {}
                }}
            </div>
        
</MainLayout>}
    }
}

impl InventoryCountPage {
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
                    <div class="error-icon">{"!"}</div>
                    <p class="error-message">{error}</p>
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadCounts)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        if self.counts.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"C"}</div>
                    <p>{"暂无库存盘点记录"}</p>
                </div>
            };
        }

        html! {
            <div class="table-responsive">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>{"盘点单号"}</th>
                            <th>{"仓库ID"}</th>
                            <th>{"盘点日期"}</th>
                            <th>{"状态"}</th>
                            <th>{"总条目"}</th>
                            <th>{"已盘条目"}</th>
                            <th>{"差异条目"}</th>
                            <th>{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.counts.iter().map(|count| {
                            let count_id = count.id;
                            let status_class = match count.status.as_str() {
                                "pending" => "status-pending",
                                "approved" => "status-approved",
                                "rejected" => "status-rejected",
                                "completed" => "status-completed",
                                _ => "",
                            };
                            let status_text = match count.status.as_str() {
                                "pending" => "待审核",
                                "approved" => "已审核",
                                "rejected" => "已驳回",
                                "completed" => "已完成",
                                _ => &count.status,
                            };
                            html! {
                                <tr>
                                    <td>
                                        <a href="#" onclick={ctx.link().callback(move |_| Msg::OpenViewModal(count_id))}>
                                            {&count.count_no}
                                        </a>
                                    </td>
                                    <td>{count.warehouse_id}</td>
                                    <td>{&count.count_date}</td>
                                    <td><span class={format!("status-badge {}", status_class)}>{status_text}</span></td>
                                    <td class="numeric">{count.total_items}</td>
                                    <td class="numeric">{count.counted_items}</td>
                                    <td class="numeric">{count.variance_items}</td>
                                    <td>
                                        <div class="action-buttons">
                                            {if count.status == "pending" {
                                                html! {
                                                    <>
                                                        <button class="btn-link" onclick={ctx.link().callback(move |_| Msg::OpenApproveModal(count_id))}>
                                                            {"审核"}
                                                        </button>
                                                        <button class="btn-link" onclick={ctx.link().callback(move |_| Msg::OpenEditModal(count_id))}>
                                                            {"编辑"}
                                                        </button>
                                                    </>
                                                }
                                            } else if count.status == "approved" {
                                                html! {
                                                    <button class="btn-link" onclick={ctx.link().callback(move |_| Msg::CompleteCount(count_id))}>
                                                        {"完成盘点"}
                                                    </button>
                                                }
                                            } else {
                                                html! {}
                                            }}
                                        </div>
                                    </td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
            </div>
        }
    }

    fn render_modal(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="modal-overlay" onclick={ctx.link().callback(|_| Msg::CloseModal)}>
                <div class="modal-content" onclick={|e: MouseEvent| e.stop_propagation()}>
                    <div class="modal-header">
                        <h2>
                            {match self.modal_mode {
                                ModalMode::View => "盘点单详情",
                                ModalMode::Create => "新建盘点单",
                                ModalMode::Edit => "编辑盘点单",
                                ModalMode::Approve => "审核盘点单",
                            }}
                        </h2>
                        <button class="modal-close" onclick={ctx.link().callback(|_| Msg::CloseModal)}>{"x"}</button>
                    </div>
                    <div class="modal-body">
                        {match self.modal_mode {
                            ModalMode::View => self.render_view_content(ctx),
                            ModalMode::Create => self.render_create_content(ctx),
                            ModalMode::Edit => self.render_edit_content(ctx),
                            ModalMode::Approve => self.render_approve_content(ctx),
                        }}
                    </div>
                </div>
            </div>
        }
    }

    fn render_view_content(&self, _ctx: &Context<Self>) -> Html {
        if let Some(ref detail) = self.selected_count {
            let status_text = match detail.status.as_str() {
                "pending" => "待审核",
                "approved" => "已审核",
                "rejected" => "已驳回",
                "completed" => "已完成",
                _ => &detail.status,
            };
            html! {
                <div class="detail-content">
                    <div class="detail-row">
                        <span class="detail-label">{"盘点单号："}</span>
                        <span class="detail-value">{&detail.count_no}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"仓库ID："}</span>
                        <span class="detail-value">{detail.warehouse_id}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"盘点日期："}</span>
                        <span class="detail-value">{&detail.count_date}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"状态："}</span>
                        <span class="detail-value">{status_text}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"总条目："}</span>
                        <span class="detail-value">{detail.total_items}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"已盘条目："}</span>
                        <span class="detail-value">{detail.counted_items}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"差异条目："}</span>
                        <span class="detail-value">{detail.variance_items}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"备注："}</span>
                        <span class="detail-value">{detail.notes.as_deref().unwrap_or("-")}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"创建时间："}</span>
                        <span class="detail-value">{&detail.created_at}</span>
                    </div>
                </div>
            }
        } else {
            html! {
                <div class="loading">{"加载中..."}</div>
            }
        }
    }

    fn render_create_content(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="form-content">
                <div class="form-item">
                    <label>{"仓库ID："}</label>
                    <input
                        type="number"
                        value={self.form_warehouse_id.to_string()}
                        oninput={ctx.link().callback(|e: InputEvent| {
                            let input = e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
                            Msg::FormWarehouseChanged(input.value().parse().unwrap_or(0))
                        })}
                    />
                </div>
                <div class="form-item">
                    <label>{"备注："}</label>
                    <textarea
                        value={self.form_notes.clone()}
                        oninput={ctx.link().callback(|e: InputEvent| {
                            let input = e.target().unwrap().dyn_into::<web_sys::HtmlTextAreaElement>().unwrap();
                            Msg::FormNotesChanged(input.value())
                        })}
                        placeholder="请输入备注信息"
                    />
                </div>
                <div class="form-actions">
                    <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseModal)}>
                        {"取消"}
                    </button>
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::CreateCount)}>
                        {"创建"}
                    </button>
                </div>
            </div>
        }
    }

    fn render_edit_content(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="form-content">
                <div class="form-item">
                    <label>{"备注："}</label>
                    <textarea
                        value={self.form_notes.clone()}
                        oninput={ctx.link().callback(|e: InputEvent| {
                            let input = e.target().unwrap().dyn_into::<web_sys::HtmlTextAreaElement>().unwrap();
                            Msg::FormNotesChanged(input.value())
                        })}
                        placeholder="请输入备注信息"
                    />
                </div>
                <div class="form-actions">
                    <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseModal)}>
                        {"取消"}
                    </button>
                    {if let Some(ref detail) = self.selected_count {
                        let detail_id = detail.id;
                        html! {
                            <button class="btn-primary" onclick={ctx.link().callback(move |_| Msg::UpdateCount(detail_id))}>
                                {"保存"}
                            </button>
                        }
                    } else {
                        html! {}
                    }}
                </div>
            </div>
        }
    }

    fn render_approve_content(&self, ctx: &Context<Self>) -> Html {
        if let Some(ref detail) = self.selected_count {
            let detail_id = detail.id;
            html! {
                <div class="approve-content">
                    <p>{"确定要审核盘点单 "}{&detail.count_no}{" 吗？"}</p>
                    <p>{"总条目数："}{detail.total_items}</p>
                    <div class="form-actions">
                        <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseModal)}>
                            {"取消"}
                        </button>
                        <button class="btn-danger" onclick={ctx.link().callback(move |_| Msg::ApproveCount(detail_id, false))}>
                            {"驳回"}
                        </button>
                        <button class="btn-primary" onclick={ctx.link().callback(move |_| Msg::ApproveCount(detail_id, true))}>
                            {"通过"}
                        </button>
                    </div>
                </div>
            }
        } else {
            html! {
                <div class="loading">{"加载中..."}</div>
            }
        }
    }
}
