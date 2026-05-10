// 库存盘点管理页面
// 提供库存盘点单的列表、创建、编辑、删除、审核、完成等功能

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
use crate::models::inventory_count::{
    InventoryCount, InventoryCountDetail,
    InventoryCountQuery, CreateInventoryCountRequest, UpdateInventoryCountRequest,
};
use crate::services::inventory_count_service::InventoryCountService;
use crate::services::crud_service::CrudService;

pub struct InventoryCountPage {
    counts: Vec<InventoryCount>,
    filtered_counts: Vec<InventoryCount>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_count: Option<InventoryCount>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    // 表单字段
    form_warehouse_id: i32,
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
    DataLoaded(Vec<InventoryCount>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    OpenEditModal(InventoryCount),
    OpenViewModal(i32),
    OpenApproveModal(i32),
    CloseModal,
    ViewCountDetail(InventoryCountDetail),
    SubmitForm,
    FormSubmitted,
    DeleteCount(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    ApproveCount(i32, bool),
    CompleteCount(i32),
    // 表单字段变更
    FormWarehouseChanged(i32),
    FormNotesChanged(String),
}

impl Component for InventoryCountPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            counts: Vec::new(),
            filtered_counts: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            show_modal: false,
            modal_mode: ModalMode::Create,
            editing_count: None,
            show_delete_confirm: false,
            deleting_id: None,
            form_warehouse_id: 0,
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
                    let query = InventoryCountQuery {
                        page: Some(1),
                        page_size: Some(1000),
                        status: None,
                        warehouse_id: None,
                        count_no: None,
                    };
                    match InventoryCountService::list(query).await {
                        Ok(res) => link.send_message(Msg::DataLoaded(res)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::DataLoaded(data) => {
                self.loading = false;
                self.counts = data;
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
                self.editing_count = None;
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(count) => {
                self.form_warehouse_id = count.warehouse_id;
                self.form_notes = count.notes.clone().unwrap_or_default();
                self.form_error = None;
                self.editing_count = Some(count.clone());
                self.modal_mode = ModalMode::Edit;
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
                self.editing_count = Some(InventoryCount {
                    id: detail.id,
                    count_no: detail.count_no,
                    warehouse_id: detail.warehouse_id,
                    count_date: detail.count_date,
                    status: detail.status,
                    total_items: detail.total_items,
                    counted_items: detail.counted_items,
                    variance_items: detail.variance_items,
                    notes: detail.notes,
                    created_by: detail.created_by,
                    approved_by: detail.approved_by,
                    approved_at: detail.approved_at,
                    completed_at: detail.completed_at,
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
                    match InventoryCountService::get(id).await {
                        Ok(detail) => link.send_message(Msg::ViewCountDetail(detail)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.editing_count = None;
                self.form_error = None;
                true
            }
            Msg::SubmitForm => {
                if self.form_warehouse_id == 0 {
                    self.form_error = Some("请选择仓库".to_string());
                    return true;
                }

                self.form_error = None;

                let req = CreateInventoryCountRequest {
                    warehouse_id: self.form_warehouse_id,
                    count_date: None,
                    status: "pending".to_string(),
                    notes: if self.form_notes.is_empty() { None } else { Some(self.form_notes.clone()) },
                    items: None,
                };

                let link = ctx.link().clone();

                if self.modal_mode == ModalMode::Edit {
                    if let Some(count) = &self.editing_count {
                        let id = count.id;
                        let update_req = UpdateInventoryCountRequest {
                            status: None,
                            notes: if self.form_notes.is_empty() { None } else { Some(self.form_notes.clone()) },
                            items: None,
                        };
                        spawn_local(async move {
                            match InventoryCountService::update(id, update_req).await {
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
                        match InventoryCountService::create(req).await {
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
                self.editing_count = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::DeleteCount(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match InventoryCountService::delete(id).await {
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
            Msg::ApproveCount(id, approved) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryCountService::approve(id, approved, None).await {
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
            Msg::CompleteCount(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryCountService::complete(id).await {
                        Ok(_) => {
                            toast_helper::show_success("盘点完成");
                            link.send_message(Msg::LoadData);
                        }
                        Err(e) => {
                            toast_helper::show_error(&format!("操作失败: {}", e));
                        }
                    }
                });
                false
            }
            Msg::FormWarehouseChanged(v) => { self.form_warehouse_id = v; true }
            Msg::FormNotesChanged(v) => { self.form_notes = v; true }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="inventory-count-page">
                <PageHeader title={"库存盘点管理".to_string()} subtitle={Some("管理库存盘点单据".to_string())}>
                    <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                        {"+ 新建盘点单"}
                    </button>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索盘点单号...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载盘点数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadData)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_counts.is_empty() {
                    <EmptyState
                        icon={"📋".to_string()}
                        title={"暂无盘点数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一个盘点单".to_string()
                        } else {
                            "没有匹配搜索条件的盘点单".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"盘点单号"}</th>
                                    <th>{"仓库ID"}</th>
                                    <th>{"盘点日期"}</th>
                                    <th>{"状态"}</th>
                                    <th class="numeric">{"总条目"}</th>
                                    <th class="numeric">{"已盘条目"}</th>
                                    <th class="numeric">{"差异条目"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_counts().iter().map(|c| {
                                    let c_clone = c.clone();
                                    let id = c.id;
                                    let status_class = match c.status.as_str() {
                                        "pending" => "status-pending",
                                        "approved" => "status-approved",
                                        "rejected" => "status-rejected",
                                        "completed" => "status-completed",
                                        _ => "",
                                    };
                                    let status_text = match c.status.as_str() {
                                        "pending" => "待审核",
                                        "approved" => "已审核",
                                        "rejected" => "已驳回",
                                        "completed" => "已完成",
                                        _ => &c.status,
                                    };
                                    html! {
                                        <tr>
                                            <td>
                                                <a href="javascript:void(0);" onclick={link.callback(move |_| Msg::OpenViewModal(id))}>
                                                    {&c.count_no}
                                                </a>
                                            </td>
                                            <td>{c.warehouse_id}</td>
                                            <td>{&c.count_date}</td>
                                            <td><span class={format!("status-badge {}", status_class)}>{status_text}</span></td>
                                            <td class="numeric">{c.total_items}</td>
                                            <td class="numeric">{c.counted_items}</td>
                                            <td class="numeric">{c.variance_items}</td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-secondary"
                                                        onclick={link.callback(move |_| Msg::OpenViewModal(id))}
                                                    >
                                                        {"查看"}
                                                    </button>
                                                    if c.status == "pending" {
                                                        <button
                                                            class="btn btn-sm btn-secondary"
                                                            onclick={link.callback(move |_| Msg::OpenEditModal(c_clone.clone()))}
                                                        >
                                                            {"编辑"}
                                                        </button>
                                                        <button
                                                            class="btn btn-sm btn-info"
                                                            onclick={link.callback(move |_| Msg::OpenApproveModal(id))}
                                                        >
                                                            {"审核"}
                                                        </button>
                                                        <PermissionGuard resource="inventory_count" action="delete">
                                                            <button
                                                                class="btn btn-sm btn-danger"
                                                                onclick={link.callback(move |_| Msg::DeleteCount(id))}
                                                            >
                                                                {"删除"}
                                                            </button>
                                                        </PermissionGuard>
                                                    } else if c.status == "approved" {
                                                        <button
                                                            class="btn btn-sm btn-success"
                                                            onclick={link.callback(move |_| Msg::CompleteCount(id))}
                                                        >
                                                            {"完成盘点"}
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
                            total={self.filtered_counts.len() as u64}
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
                    message={"确定要删除这个盘点单吗？此操作不可撤销。".to_string()}
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

impl InventoryCountPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_counts = self.counts.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_counts = self.counts.iter()
                .filter(|c| {
                    c.count_no.to_lowercase().contains(&keyword) ||
                    c.status.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_counts(&self) -> Vec<InventoryCount> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_counts[start..end.min(self.filtered_counts.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_warehouse_id = 0;
        self.form_notes = String::new();
        self.form_error = None;
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let title = match self.modal_mode {
            ModalMode::Create => "新建盘点单",
            ModalMode::Edit => "编辑盘点单",
            ModalMode::View => "盘点单详情",
            ModalMode::Approve => "审核盘点单",
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
        if let Some(ref count) = self.editing_count {
            let status_text = match count.status.as_str() {
                "pending" => "待审核",
                "approved" => "已审核",
                "rejected" => "已驳回",
                "completed" => "已完成",
                _ => &count.status,
            };
            html! {
                <div class="detail-content">
                    <div class="detail-row">
                        <span class="detail-label">{"盘点单号："}</span>
                        <span class="detail-value">{&count.count_no}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"仓库ID："}</span>
                        <span class="detail-value">{count.warehouse_id}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"盘点日期："}</span>
                        <span class="detail-value">{&count.count_date}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"状态："}</span>
                        <span class="detail-value">{status_text}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"总条目："}</span>
                        <span class="detail-value">{count.total_items}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"已盘条目："}</span>
                        <span class="detail-value">{count.counted_items}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"差异条目："}</span>
                        <span class="detail-value">{count.variance_items}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"备注："}</span>
                        <span class="detail-value">{count.notes.as_deref().unwrap_or("-")}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"创建时间："}</span>
                        <span class="detail-value">{&count.created_at}</span>
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
        let on_warehouse_change = ctx.link().batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormWarehouseChanged(input.value().parse().unwrap_or(0)))
        });
        let on_notes_change = ctx.link().batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormNotesChanged(input.value()))
        });

        html! {
            <div class="form-content">
                <div class="form-group">
                    <label>{"仓库ID *"}</label>
                    <input
                        type="number"
                        class="form-input"
                        value={self.form_warehouse_id.to_string()}
                        oninput={on_warehouse_change}
                        placeholder="请输入仓库ID"
                    />
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
                        {"创建盘点单"}
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
        if let Some(ref count) = self.editing_count {
            let id = count.id;
            html! {
                <div class="approve-content">
                    <p>{"确定要审核盘点单 "}{&count.count_no}{" 吗？"}</p>
                    <p>{"总条目数："}{count.total_items}</p>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseModal)}>
                            {"取消"}
                        </button>
                        <PermissionGuard resource="inventory_count" action="approve">
                            <button class="btn btn-danger" onclick={ctx.link().callback(move |_| Msg::ApproveCount(id, false))}>
                                {"驳回"}
                            </button>
                        </PermissionGuard>
                        <PermissionGuard resource="inventory_count" action="approve">
                            <button class="btn btn-primary" onclick={ctx.link().callback(move |_| Msg::ApproveCount(id, true))}>
                                {"通过"}
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
