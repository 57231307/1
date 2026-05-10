// 库存调拨管理页面
// 提供库存调拨单的列表、创建、编辑、删除、审核、发货、收货等功能

use crate::utils::toast_helper;
use yew::prelude::*;
use crate::components::permission_guard::PermissionGuard;
use crate::components::{
    confirm_dialog::ConfirmDialog,
    search_bar::SearchBar,
    pagination::Pagination,
    page_header::PageHeader,
    empty_state::EmptyState,
    loading_state::LoadingState,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use crate::models::inventory_transfer::{
    InventoryTransfer, InventoryTransferDetail,
    InventoryTransferQuery, CreateInventoryTransferRequest, UpdateInventoryTransferRequest,
};
use crate::services::inventory_transfer_service::InventoryTransferService;
use crate::services::crud_service::CrudService;

pub struct InventoryTransferPage {
    transfers: Vec<InventoryTransfer>,
    filtered_transfers: Vec<InventoryTransfer>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_transfer: Option<InventoryTransfer>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    // 表单字段
    form_from_warehouse_id: i32,
    form_to_warehouse_id: i32,
    form_notes: String,
    form_error: Option<String>,
}

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    Create,
    Edit,
    View,
    Approve,
}

pub enum Msg {
    LoadData,
    DataLoaded(Vec<InventoryTransfer>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    OpenEditModal(InventoryTransfer),
    OpenViewModal(i32),
    OpenApproveModal(i32),
    CloseModal,
    ViewTransferDetail(InventoryTransferDetail),
    SubmitForm,
    FormSubmitted,
    DeleteTransfer(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    ApproveTransfer(i32, bool),
    ShipTransfer(i32),
    ReceiveTransfer(i32),
    // 表单字段变更
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
            filtered_transfers: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            show_modal: false,
            modal_mode: ModalMode::Create,
            editing_transfer: None,
            show_delete_confirm: false,
            deleting_id: None,
            form_from_warehouse_id: 0,
            form_to_warehouse_id: 0,
            form_notes: String::new(),
            form_error: None,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadData);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadData => {
                self.loading = true;
                self.error = None;
                let link = ctx.link().clone();
                spawn_local(async move {
                    let query = InventoryTransferQuery {
                        page: Some(1),
                        page_size: Some(1000),
                        status: None,
                        from_warehouse_id: None,
                        to_warehouse_id: None,
                        transfer_no: None,
                    };
                    match InventoryTransferService::list(query).await {
                        Ok(res) => link.send_message(Msg::DataLoaded(res)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::DataLoaded(data) => {
                self.loading = false;
                self.transfers = data;
                self.apply_filter();
                true
            }
            Msg::LoadError(err) => {
                self.error = Some(err);
                self.loading = false;
                true
            }
            Msg::Search(keyword) => {
                self.search_keyword = keyword;
                self.page = 0;
                self.apply_filter();
                true
            }
            Msg::ResetSearch => {
                self.search_keyword = String::new();
                self.page = 0;
                self.apply_filter();
                true
            }
            Msg::PageChanged(page) => {
                self.page = page;
                true
            }
            Msg::OpenCreateModal => {
                self.reset_form();
                self.editing_transfer = None;
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(transfer) => {
                self.form_from_warehouse_id = transfer.from_warehouse_id;
                self.form_to_warehouse_id = transfer.to_warehouse_id;
                self.form_notes = transfer.notes.clone().unwrap_or_default();
                self.form_error = None;
                self.editing_transfer = Some(transfer.clone());
                self.modal_mode = ModalMode::Edit;
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
                self.editing_transfer = Some(InventoryTransfer {
                    id: detail.id,
                    transfer_no: detail.transfer_no,
                    from_warehouse_id: detail.from_warehouse_id,
                    to_warehouse_id: detail.to_warehouse_id,
                    transfer_date: detail.transfer_date,
                    status: detail.status,
                    total_quantity: detail.total_quantity,
                    notes: detail.notes,
                    created_by: detail.created_by,
                    approved_by: detail.approved_by,
                    approved_at: detail.approved_at,
                    shipped_at: detail.shipped_at,
                    received_at: detail.received_at,
                    created_at: detail.created_at,
                    updated_at: detail.updated_at,
                });
                true
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
                self.editing_transfer = None;
                self.form_error = None;
                true
            }
            Msg::SubmitForm => {
                if self.form_from_warehouse_id == 0 {
                    self.form_error = Some("请选择源仓库".to_string());
                    return true;
                }
                if self.form_to_warehouse_id == 0 {
                    self.form_error = Some("请选择目标仓库".to_string());
                    return true;
                }
                if self.form_from_warehouse_id == self.form_to_warehouse_id {
                    self.form_error = Some("源仓库和目标仓库不能相同".to_string());
                    return true;
                }

                self.form_error = None;

                let req = CreateInventoryTransferRequest {
                    from_warehouse_id: self.form_from_warehouse_id,
                    to_warehouse_id: self.form_to_warehouse_id,
                    transfer_date: None,
                    status: "pending".to_string(),
                    notes: if self.form_notes.is_empty() { None } else { Some(self.form_notes.clone()) },
                    items: vec![],
                };

                let link = ctx.link().clone();

                if self.modal_mode == ModalMode::Edit {
                    if let Some(transfer) = &self.editing_transfer {
                        let id = transfer.id;
                        let update_req = UpdateInventoryTransferRequest {
                            status: None,
                            notes: if self.form_notes.is_empty() { None } else { Some(self.form_notes.clone()) },
                            items: None,
                        };
                        spawn_local(async move {
                            match InventoryTransferService::update(id, update_req).await {
                                Ok(_) => {
                                    toast_helper::show_success("更新成功");
                                    link.send_message(Msg::FormSubmitted);
                                }
                                Err(e) => {
                                    toast_helper::show_error(&format!("更新失败: {}", e));
                                }
                            }
                        });
                    }
                } else {
                    spawn_local(async move {
                        match InventoryTransferService::create(req).await {
                            Ok(_) => {
                                toast_helper::show_success("创建成功");
                                link.send_message(Msg::FormSubmitted);
                            }
                            Err(e) => {
                                toast_helper::show_error(&format!("创建失败: {}", e));
                            }
                        }
                    });
                }
                false
            }
            Msg::FormSubmitted => {
                self.show_modal = false;
                self.editing_transfer = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::DeleteTransfer(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match InventoryTransferService::delete(id).await {
                            Ok(_) => {
                                toast_helper::show_success("删除成功");
                                link.send_message(Msg::Deleted);
                            }
                            Err(e) => {
                                toast_helper::show_error(&format!("删除失败: {}", e));
                                link.send_message(Msg::CancelDelete);
                            }
                        }
                    });
                }
                false
            }
            Msg::CancelDelete => {
                self.show_delete_confirm = false;
                self.deleting_id = None;
                true
            }
            Msg::Deleted => {
                self.show_delete_confirm = false;
                self.deleting_id = None;
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::ApproveTransfer(id, approved) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryTransferService::approve(id, approved, None).await {
                        Ok(_) => {
                            toast_helper::show_success("审核操作成功");
                            link.send_message(Msg::CloseModal);
                            link.send_message(Msg::LoadData);
                        }
                        Err(e) => {
                            toast_helper::show_error(&format!("审核失败: {}", e));
                        }
                    }
                });
                false
            }
            Msg::ShipTransfer(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryTransferService::ship(id).await {
                        Ok(_) => {
                            toast_helper::show_success("发货成功");
                            link.send_message(Msg::LoadData);
                        }
                        Err(e) => {
                            toast_helper::show_error(&format!("发货失败: {}", e));
                        }
                    }
                });
                false
            }
            Msg::ReceiveTransfer(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryTransferService::receive(id).await {
                        Ok(_) => {
                            toast_helper::show_success("收货成功");
                            link.send_message(Msg::LoadData);
                        }
                        Err(e) => {
                            toast_helper::show_error(&format!("收货失败: {}", e));
                        }
                    }
                });
                false
            }
            Msg::FormFromWarehouseChanged(v) => { self.form_from_warehouse_id = v; true }
            Msg::FormToWarehouseChanged(v) => { self.form_to_warehouse_id = v; true }
            Msg::FormNotesChanged(v) => { self.form_notes = v; true }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="inventory-transfer-page">
                <PageHeader title={"库存调拨管理".to_string()} subtitle={Some("管理库存调拨单据".to_string())}>
                    <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                        {"+ 新建调拨单"}
                    </button>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索调拨单号...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载调拨数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadData)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_transfers.is_empty() {
                    <EmptyState
                        icon={"🚚".to_string()}
                        title={"暂无调拨数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一个调拨单".to_string()
                        } else {
                            "没有匹配搜索条件的调拨单".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"调拨单号"}</th>
                                    <th>{"源仓库"}</th>
                                    <th>{"目标仓库"}</th>
                                    <th>{"调拨日期"}</th>
                                    <th>{"状态"}</th>
                                    <th class="numeric">{"总数量"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_transfers().iter().map(|t| {
                                    let t_clone = t.clone();
                                    let id = t.id;
                                    let status_class = match t.status.as_str() {
                                        "pending" => "status-pending",
                                        "approved" => "status-approved",
                                        "rejected" => "status-rejected",
                                        "shipped" => "status-shipped",
                                        "completed" => "status-completed",
                                        _ => "",
                                    };
                                    let status_text = match t.status.as_str() {
                                        "pending" => "待审核",
                                        "approved" => "已审核",
                                        "rejected" => "已拒绝",
                                        "shipped" => "已发货",
                                        "completed" => "已完成",
                                        _ => &t.status,
                                    };
                                    html! {
                                        <tr>
                                            <td>
                                                <a href="javascript:void(0);" onclick={link.callback(move |_| Msg::OpenViewModal(id))}>
                                                    {&t.transfer_no}
                                                </a>
                                            </td>
                                            <td>{t.from_warehouse_id}</td>
                                            <td>{t.to_warehouse_id}</td>
                                            <td>{&t.transfer_date}</td>
                                            <td><span class={format!("status-badge {}", status_class)}>{status_text}</span></td>
                                            <td class="numeric">{&t.total_quantity}</td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-secondary"
                                                        onclick={link.callback(move |_| Msg::OpenViewModal(id))}
                                                    >
                                                        {"查看"}
                                                    </button>
                                                    if t.status == "pending" {
                                                        <button
                                                            class="btn btn-sm btn-secondary"
                                                            onclick={link.callback(move |_| Msg::OpenEditModal(t_clone.clone()))}
                                                        >
                                                            {"编辑"}
                                                        </button>
                                                        <button
                                                            class="btn btn-sm btn-info"
                                                            onclick={link.callback(move |_| Msg::OpenApproveModal(id))}
                                                        >
                                                            {"审核"}
                                                        </button>
                                                        <PermissionGuard resource="inventory_transfer" action="delete">
                                                            <button
                                                                class="btn btn-sm btn-danger"
                                                                onclick={link.callback(move |_| Msg::DeleteTransfer(id))}
                                                            >
                                                                {"删除"}
                                                            </button>
                                                        </PermissionGuard>
                                                    } else if t.status == "approved" {
                                                        <button
                                                            class="btn btn-sm btn-warning"
                                                            onclick={link.callback(move |_| Msg::ShipTransfer(id))}
                                                        >
                                                            {"发货"}
                                                        </button>
                                                    } else if t.status == "shipped" {
                                                        <button
                                                            class="btn btn-sm btn-success"
                                                            onclick={link.callback(move |_| Msg::ReceiveTransfer(id))}
                                                        >
                                                            {"收货"}
                                                        </button>
                                                    }
                                                </div>
                                            </td>
                                        </tr>
                                    }
                                })}
                            </tbody>
                        </table>

                        <Pagination
                            current_page={self.page}
                            page_size={self.page_size}
                            total={self.filtered_transfers.len() as u64}
                            on_page_change={link.callback(|page| Msg::PageChanged(page))}
                        />
                    </div>
                }

                // 新建/编辑/查看/审核弹窗
                if self.show_modal {
                    {self.render_form_modal(ctx)}
                }

                // 删除确认对话框
                <ConfirmDialog
                    title={"确认删除".to_string()}
                    message={"确定要删除这个调拨单吗？此操作不可撤销。".to_string()}
                    confirm_text={"删除".to_string()}
                    cancel_text={"取消".to_string()}
                    confirm_class={"btn-danger".to_string()}
                    on_confirm={link.callback(|_| Msg::ConfirmDelete)}
                    on_cancel={link.callback(|_| Msg::CancelDelete)}
                    visible={self.show_delete_confirm}
                />
            </div>
        }
    }
}

impl InventoryTransferPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_transfers = self.transfers.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_transfers = self.transfers.iter()
                .filter(|t| {
                    t.transfer_no.to_lowercase().contains(&keyword) ||
                    t.status.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_transfers(&self) -> Vec<InventoryTransfer> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_transfers[start..end.min(self.filtered_transfers.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_from_warehouse_id = 0;
        self.form_to_warehouse_id = 0;
        self.form_notes = String::new();
        self.form_error = None;
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let title = match self.modal_mode {
            ModalMode::Create => "新建调拨单",
            ModalMode::Edit => "编辑调拨单",
            ModalMode::View => "调拨单详情",
            ModalMode::Approve => "审核调拨单",
        };

        html! {
            <div class="modal-overlay" onclick={link.callback(|_| Msg::CloseModal)}>
                <div class="modal-content" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <div class="modal-header">
                        <h3>{title}</h3>
                        <button class="close-btn" onclick={link.callback(|_| Msg::CloseModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        if let Some(err) = &self.form_error {
                            <div class="form-error">{err}</div>
                        }
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
        if let Some(ref transfer) = self.editing_transfer {
            let status_text = match transfer.status.as_str() {
                "pending" => "待审核",
                "approved" => "已审核",
                "rejected" => "已拒绝",
                "shipped" => "已发货",
                "completed" => "已完成",
                _ => &transfer.status,
            };
            html! {
                <div class="detail-content">
                    <div class="detail-row">
                        <span class="detail-label">{"调拨单号："}</span>
                        <span class="detail-value">{&transfer.transfer_no}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"源仓库ID："}</span>
                        <span class="detail-value">{transfer.from_warehouse_id}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"目标仓库ID："}</span>
                        <span class="detail-value">{transfer.to_warehouse_id}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"调拨日期："}</span>
                        <span class="detail-value">{&transfer.transfer_date}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"状态："}</span>
                        <span class="detail-value">{status_text}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"总数量："}</span>
                        <span class="detail-value">{&transfer.total_quantity}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"备注："}</span>
                        <span class="detail-value">{transfer.notes.as_deref().unwrap_or("-")}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"创建时间："}</span>
                        <span class="detail-value">{&transfer.created_at}</span>
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
        let on_from_warehouse_change = ctx.link().batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormFromWarehouseChanged(input.value().parse().unwrap_or(0)))
        });
        let on_to_warehouse_change = ctx.link().batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormToWarehouseChanged(input.value().parse().unwrap_or(0)))
        });
        let on_notes_change = ctx.link().batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormNotesChanged(input.value()))
        });

        html! {
            <div class="form-content">
                <div class="form-row">
                    <div class="form-group">
                        <label>{"源仓库ID *"}</label>
                        <input
                            type="number"
                            class="form-input"
                            value={self.form_from_warehouse_id.to_string()}
                            oninput={on_from_warehouse_change}
                            placeholder="请输入源仓库ID"
                        />
                    </div>
                    <div class="form-group">
                        <label>{"目标仓库ID *"}</label>
                        <input
                            type="number"
                            class="form-input"
                            value={self.form_to_warehouse_id.to_string()}
                            oninput={on_to_warehouse_change}
                            placeholder="请输入目标仓库ID"
                        />
                    </div>
                </div>
                <div class="form-group">
                    <label>{"备注"}</label>
                    <textarea
                        class="form-input"
                        value={self.form_notes.clone()}
                        oninput={on_notes_change}
                        placeholder="请输入备注信息"
                        rows="3"
                    />
                </div>
                <div class="modal-footer">
                    <button class="btn btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseModal)}>
                        {"取消"}
                    </button>
                    <button class="btn btn-primary" onclick={ctx.link().callback(|_| Msg::SubmitForm)}>
                        {"创建调拨单"}
                    </button>
                </div>
            </div>
        }
    }

    fn render_edit_content(&self, ctx: &Context<Self>) -> Html {
        let on_notes_change = ctx.link().batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormNotesChanged(input.value()))
        });

        html! {
            <div class="form-content">
                <div class="form-group">
                    <label>{"备注"}</label>
                    <textarea
                        class="form-input"
                        value={self.form_notes.clone()}
                        oninput={on_notes_change}
                        placeholder="请输入备注信息"
                        rows="3"
                    />
                </div>
                <div class="modal-footer">
                    <button class="btn btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseModal)}>
                        {"取消"}
                    </button>
                    <button class="btn btn-primary" onclick={ctx.link().callback(|_| Msg::SubmitForm)}>
                        {"保存修改"}
                    </button>
                </div>
            </div>
        }
    }

    fn render_approve_content(&self, ctx: &Context<Self>) -> Html {
        if let Some(ref transfer) = self.editing_transfer {
            let id = transfer.id;
            html! {
                <div class="approve-content">
                    <p>{"确定要审核调拨单 "}{&transfer.transfer_no}{" 吗？"}</p>
                    <p>{"总数量："}{&transfer.total_quantity}</p>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseModal)}>
                            {"取消"}
                        </button>
                        <PermissionGuard resource="inventory_transfer" action="approve">
                            <button class="btn btn-danger" onclick={ctx.link().callback(move |_| Msg::ApproveTransfer(id, false))}>
                                {"拒绝"}
                            </button>
                        </PermissionGuard>
                        <PermissionGuard resource="inventory_transfer" action="approve">
                            <button class="btn btn-primary" onclick={ctx.link().callback(move |_| Msg::ApproveTransfer(id, true))}>
                                {"审核通过"}
                            </button>
                        </PermissionGuard>
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
