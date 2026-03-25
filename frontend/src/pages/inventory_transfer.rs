//! 库存调拨管理页面
//! 提供库存调拨单的列表、创建、审核、发出、接收等功能

use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::services::inventory_transfer_service::{
    InventoryTransferService, InventoryTransfer, InventoryTransferDetail,
    InventoryTransferQuery, CreateInventoryTransferRequest, UpdateInventoryTransferRequest,
};
use crate::components::navigation::Navigation;

pub struct InventoryTransferPage {
    transfers: Vec<InventoryTransfer>,
    loading: bool,
    error: Option<String>,
    filter_status: String,
    filter_from_warehouse: Option<i32>,
    filter_to_warehouse: Option<i32>,
    filter_transfer_no: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    selected_transfer: Option<InventoryTransferDetail>,
    form_from_warehouse_id: i32,
    form_to_warehouse_id: i32,
    form_notes: String,
}

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    View,
    Create,
    Edit,
    Approve,
}

pub enum Msg {
    LoadTransfers,
    TransfersLoaded(Vec<InventoryTransfer>),
    LoadError(String),
    SetFilterStatus(String),
    SetFilterFromWarehouse(i32),
    SetFilterToWarehouse(i32),
    SetFilterTransferNo(String),
    ChangePage(u64),
    OpenCreateModal,
    OpenViewModal(i32),
    OpenEditModal(i32),
    OpenApproveModal(i32),
    CloseModal,
    ViewTransferDetail(InventoryTransferDetail),
    CreateTransfer,
    UpdateTransfer(i32),
    ApproveTransfer(i32, bool),
    ShipTransfer(i32),
    ReceiveTransfer(i32),
    DeleteTransfer(i32),
    FormFromWarehouseChanged(i32),
    FormToWarehouseChanged(i32),
    FormNotesChanged(String),
}

impl Component for InventoryTransferPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            transfers: Vec::new(),
            loading: true,
            error: None,
            filter_status: String::from("ALL"),
            filter_from_warehouse: None,
            filter_to_warehouse: None,
            filter_transfer_no: String::new(),
            page: 1,
            page_size: 20,
            show_modal: false,
            modal_mode: ModalMode::View,
            selected_transfer: None,
            form_from_warehouse_id: 0,
            form_to_warehouse_id: 0,
            form_notes: String::new(),
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadTransfers);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadTransfers => {
                self.loading = true;
                let query = InventoryTransferQuery {
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                    status: if self.filter_status == "ALL" { None } else { Some(self.filter_status.clone()) },
                    from_warehouse_id: self.filter_from_warehouse,
                    to_warehouse_id: self.filter_to_warehouse,
                    transfer_no: if self.filter_transfer_no.is_empty() { None } else { Some(self.filter_transfer_no.clone()) },
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryTransferService::list(query).await {
                        Ok(transfers) => link.send_message(Msg::TransfersLoaded(transfers)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::TransfersLoaded(transfers) => {
                self.transfers = transfers;
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
                ctx.link().send_message(Msg::LoadTransfers);
                false
            }
            Msg::SetFilterFromWarehouse(warehouse_id) => {
                self.filter_from_warehouse = if warehouse_id == 0 { None } else { Some(warehouse_id) };
                self.page = 1;
                ctx.link().send_message(Msg::LoadTransfers);
                false
            }
            Msg::SetFilterToWarehouse(warehouse_id) => {
                self.filter_to_warehouse = if warehouse_id == 0 { None } else { Some(warehouse_id) };
                self.page = 1;
                ctx.link().send_message(Msg::LoadTransfers);
                false
            }
            Msg::SetFilterTransferNo(transfer_no) => {
                self.filter_transfer_no = transfer_no;
                self.page = 1;
                ctx.link().send_message(Msg::LoadTransfers);
                false
            }
            Msg::ChangePage(page) => {
                self.page = page;
                ctx.link().send_message(Msg::LoadTransfers);
                false
            }
            Msg::OpenCreateModal => {
                self.modal_mode = ModalMode::Create;
                self.selected_transfer = None;
                self.form_from_warehouse_id = 0;
                self.form_to_warehouse_id = 0;
                self.form_notes = String::new();
                self.show_modal = true;
                true
            }
            Msg::OpenViewModal(id) => {
                self.modal_mode = ModalMode::View;
                self.show_modal = true;
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryTransferService::get(id).await {
                        Ok(detail) => link.send_message(Msg::ViewTransferDetail(detail)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ViewTransferDetail(detail) => {
                self.selected_transfer = Some(detail);
                true
            }
            Msg::OpenEditModal(id) => {
                self.modal_mode = ModalMode::Edit;
                self.show_modal = true;
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryTransferService::get(id).await {
                        Ok(detail) => link.send_message(Msg::ViewTransferDetail(detail)),
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
                    match InventoryTransferService::get(id).await {
                        Ok(detail) => link.send_message(Msg::ViewTransferDetail(detail)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.selected_transfer = None;
                true
            }
            Msg::CreateTransfer => {
                if self.form_from_warehouse_id == 0 || self.form_to_warehouse_id == 0 {
                    ctx.link().send_message(Msg::LoadError("请选择仓库".to_string()));
                    return false;
                }
                if self.form_from_warehouse_id == self.form_to_warehouse_id {
                    ctx.link().send_message(Msg::LoadError("源仓库和目标仓库不能相同".to_string()));
                    return false;
                }
                let req = CreateInventoryTransferRequest {
                    from_warehouse_id: self.form_from_warehouse_id,
                    to_warehouse_id: self.form_to_warehouse_id,
                    transfer_date: None,
                    status: "pending".to_string(),
                    notes: if self.form_notes.is_empty() { None } else { Some(self.form_notes.clone()) },
                    items: vec![],
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryTransferService::create(req).await {
                        Ok(_) => {
                            link.send_message(Msg::CloseModal);
                            link.send_message(Msg::LoadTransfers);
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::UpdateTransfer(id) => {
                let req = UpdateInventoryTransferRequest {
                    status: None,
                    notes: if self.form_notes.is_empty() { None } else { Some(self.form_notes.clone()) },
                    items: None,
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryTransferService::update(id, req).await {
                        Ok(_) => {
                            link.send_message(Msg::CloseModal);
                            link.send_message(Msg::LoadTransfers);
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ApproveTransfer(id, approved) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryTransferService::approve(id, approved, None).await {
                        Ok(_) => {
                            link.send_message(Msg::CloseModal);
                            link.send_message(Msg::LoadTransfers);
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ShipTransfer(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryTransferService::ship(id).await {
                        Ok(_) => link.send_message(Msg::LoadTransfers),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ReceiveTransfer(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryTransferService::receive(id).await {
                        Ok(_) => link.send_message(Msg::LoadTransfers),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::DeleteTransfer(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryTransferService::delete(id).await {
                        Ok(_) => link.send_message(Msg::LoadTransfers),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::FormFromWarehouseChanged(warehouse_id) => {
                self.form_from_warehouse_id = warehouse_id;
                false
            }
            Msg::FormToWarehouseChanged(warehouse_id) => {
                self.form_to_warehouse_id = warehouse_id;
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
            <div class="inventory-transfer-page">
                <Navigation current_page="transfers" />

                <div class="main-content">
                    <div class="page-header">
                        <h1>{"库存调拨管理"}</h1>
                        <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::OpenCreateModal)}>
                            {"新建调拨单"}
                        </button>
                    </div>

                    <div class="filter-bar">
                        <div class="filter-item">
                            <label>{"调拨单号:"}</label>
                            <input
                                type="text"
                                placeholder="输入调拨单号"
                                value={self.filter_transfer_no.clone()}
                                oninput={ctx.link().callback(|e: InputEvent| {
                                    let input = e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
                                    Msg::SetFilterTransferNo(input.value())
                                })}
                            />
                        </div>
                        <div class="filter-item">
                            <label>{"状态:"}</label>
                            <select
                                value={self.filter_status.clone()}
                                onchange={ctx.link().callback(|e: Event| {
                                    let target = e.target().unwrap().dyn_into::<web_sys::HtmlSelectElement>().unwrap();
                                    Msg::SetFilterStatus(target.value())
                                })}
                            >
                                <option value="ALL">{"全部"}</option>
                                <option value="pending">{"待审核"}</option>
                                <option value="approved">{"已审核"}</option>
                                <option value="rejected">{"已拒绝"}</option>
                                <option value="shipped">{"已发货"}</option>
                                <option value="completed">{"已完成"}</option>
                            </select>
                        </div>
                        <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::LoadTransfers)}>
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
        }
    }
}

impl InventoryTransferPage {
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
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadTransfers)}>
                        {"重试"}
                    </button>
                </div>
            };
        }

        if self.transfers.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"T"}</div>
                    <p>{"暂无调拨记录"}</p>
                </div>
            };
        }

        html! {
            <div class="table-responsive">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>{"调拨单号"}</th>
                            <th>{"源仓库"}</th>
                            <th>{"目标仓库"}</th>
                            <th>{"调拨日期"}</th>
                            <th>{"状态"}</th>
                            <th>{"总数量"}</th>
                            <th>{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.transfers.iter().map(|transfer| {
                            let transfer_id = transfer.id;
                            let status_class = match transfer.status.as_str() {
                                "pending" => "status-pending",
                                "approved" => "status-approved",
                                "rejected" => "status-rejected",
                                "shipped" => "status-shipped",
                                "completed" => "status-completed",
                                _ => "",
                            };
                            let status_text = match transfer.status.as_str() {
                                "pending" => "待审核",
                                "approved" => "已审核",
                                "rejected" => "已拒绝",
                                "shipped" => "已发货",
                                "completed" => "已完成",
                                _ => &transfer.status,
                            };
                            html! {
                                <tr>
                                    <td>
                                        <a href="#" onclick={ctx.link().callback(move |_| Msg::OpenViewModal(transfer_id))}>
                                            {&transfer.transfer_no}
                                        </a>
                                    </td>
                                    <td>{transfer.from_warehouse_id}</td>
                                    <td>{transfer.to_warehouse_id}</td>
                                    <td>{&transfer.transfer_date}</td>
                                    <td><span class={format!("status-badge {}", status_class)}>{status_text}</span></td>
                                    <td class="numeric">{&transfer.total_quantity}</td>
                                    <td>
                                        <div class="action-buttons">
                                            {if transfer.status == "pending" {
                                                html! {
                                                    <>
                                                        <button class="btn-link" onclick={ctx.link().callback(move |_| Msg::OpenApproveModal(transfer_id))}>
                                                            {"审核"}
                                                        </button>
                                                        <button class="btn-link" onclick={ctx.link().callback(move |_| Msg::OpenEditModal(transfer_id))}>
                                                            {"编辑"}
                                                        </button>
                                                        <button class="btn-link btn-danger" onclick={ctx.link().callback(move |_| Msg::DeleteTransfer(transfer_id))}>
                                                            {"删除"}
                                                        </button>
                                                    </>
                                                }
                                            } else if transfer.status == "approved" {
                                                html! {
                                                    <button class="btn-link" onclick={ctx.link().callback(move |_| Msg::ShipTransfer(transfer_id))}>
                                                        {"发货"}
                                                    </button>
                                                }
                                            } else if transfer.status == "shipped" {
                                                html! {
                                                    <button class="btn-link" onclick={ctx.link().callback(move |_| Msg::ReceiveTransfer(transfer_id))}>
                                                        {"收货"}
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
                                ModalMode::View => "调拨单详情",
                                ModalMode::Create => "新建调拨单",
                                ModalMode::Edit => "编辑调拨单",
                                ModalMode::Approve => "审核调拨单",
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
        if let Some(ref detail) = self.selected_transfer {
            let status_text = match detail.status.as_str() {
                "pending" => "待审核",
                "approved" => "已审核",
                "rejected" => "已拒绝",
                "shipped" => "已发货",
                "completed" => "已完成",
                _ => &detail.status,
            };
            html! {
                <div class="detail-content">
                    <div class="detail-row">
                        <span class="detail-label">{"调拨单号:"}</span>
                        <span class="detail-value">{&detail.transfer_no}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"源仓库ID:"}</span>
                        <span class="detail-value">{detail.from_warehouse_id}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"目标仓库ID:"}</span>
                        <span class="detail-value">{detail.to_warehouse_id}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"调拨日期:"}</span>
                        <span class="detail-value">{&detail.transfer_date}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"状态:"}</span>
                        <span class="detail-value">{status_text}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"总数量:"}</span>
                        <span class="detail-value">{&detail.total_quantity}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"备注:"}</span>
                        <span class="detail-value">{detail.notes.as_deref().unwrap_or("-")}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"创建时间:"}</span>
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
                    <label>{"源仓库ID:"}</label>
                    <input
                        type="number"
                        value={self.form_from_warehouse_id.to_string()}
                        oninput={ctx.link().callback(|e: InputEvent| {
                            let input = e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
                            Msg::FormFromWarehouseChanged(input.value().parse().unwrap_or(0))
                        })}
                    />
                </div>
                <div class="form-item">
                    <label>{"目标仓库ID:"}</label>
                    <input
                        type="number"
                        value={self.form_to_warehouse_id.to_string()}
                        oninput={ctx.link().callback(|e: InputEvent| {
                            let input = e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
                            Msg::FormToWarehouseChanged(input.value().parse().unwrap_or(0))
                        })}
                    />
                </div>
                <div class="form-item">
                    <label>{"备注:"}</label>
                    <textarea
                        value={self.form_notes.clone()}
                        oninput={ctx.link().callback(|e: InputEvent| {
                            let input = e.target().unwrap().dyn_into::<web_sys::HtmlTextAreaElement>().unwrap();
                            Msg::FormNotesChanged(input.value())
                        })}
                        placeholder="输入备注"
                    />
                </div>
                <div class="form-actions">
                    <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseModal)}>
                        {"取消"}
                    </button>
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::CreateTransfer)}>
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
                    <label>{"备注:"}</label>
                    <textarea
                        value={self.form_notes.clone()}
                        oninput={ctx.link().callback(|e: InputEvent| {
                            let input = e.target().unwrap().dyn_into::<web_sys::HtmlTextAreaElement>().unwrap();
                            Msg::FormNotesChanged(input.value())
                        })}
                        placeholder="输入备注"
                    />
                </div>
                <div class="form-actions">
                    <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseModal)}>
                        {"取消"}
                    </button>
                    {if let Some(ref detail) = self.selected_transfer {
                        let detail_id = detail.id;
                        html! {
                            <button class="btn-primary" onclick={ctx.link().callback(move |_| Msg::UpdateTransfer(detail_id))}>
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
        if let Some(ref detail) = self.selected_transfer {
            let detail_id = detail.id;
            html! {
                <div class="approve-content">
                    <p>{"确认审核调拨单 "}{&detail.transfer_no}{" ？"}</p>
                    <p>{"总数量："}{&detail.total_quantity}</p>
                    <div class="form-actions">
                        <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseModal)}>
                            {"取消"}
                        </button>
                        <button class="btn-danger" onclick={ctx.link().callback(move |_| Msg::ApproveTransfer(detail_id, false))}>
                            {"拒绝"}
                        </button>
                        <button class="btn-primary" onclick={ctx.link().callback(move |_| Msg::ApproveTransfer(detail_id, true))}>
                            {"审核通过"}
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