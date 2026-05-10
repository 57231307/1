// 采购收货页面

use crate::utils::permissions;
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
use crate::models::purchase_receipt::{
    PurchaseReceipt, PurchaseReceiptQuery, CreatePurchaseReceiptRequest, UpdatePurchaseReceiptRequest,
};
use crate::services::purchase_receipt_service::PurchaseReceiptService;
use crate::services::crud_service::CrudService;

pub struct PurchaseReceiptPage {
    receipts: Vec<PurchaseReceipt>,
    filtered_receipts: Vec<PurchaseReceipt>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_receipt: Option<PurchaseReceipt>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    viewing_item: Option<PurchaseReceipt>,
    // 表单字段
    form_receipt_no: String,
    form_order_id: String,
    form_supplier_id: String,
    form_warehouse_id: String,
    form_receipt_date: String,
    form_notes: String,
    form_error: Option<String>,
}

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    Create,
    Edit,
}

pub enum Msg {
    LoadData,
    DataLoaded(Vec<PurchaseReceipt>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    OpenEditModal(PurchaseReceipt),
    CloseModal,
    SubmitForm,
    FormSubmitted,
    DeleteReceipt(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    ViewReceipt(i32),
    CloseDetailModal,
    SubmitReceipt(i32),
    ApproveReceipt(i32),
    RejectReceipt(i32),
    // 表单字段变更
    FormReceiptNoChanged(String),
    FormOrderIdChanged(String),
    FormSupplierIdChanged(String),
    FormWarehouseIdChanged(String),
    FormReceiptDateChanged(String),
    FormNotesChanged(String),
}

impl Component for PurchaseReceiptPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            receipts: Vec::new(),
            filtered_receipts: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            show_modal: false,
            modal_mode: ModalMode::Create,
            editing_receipt: None,
            show_delete_confirm: false,
            deleting_id: None,
            viewing_item: None,
            form_receipt_no: String::new(),
            form_order_id: String::new(),
            form_supplier_id: String::new(),
            form_warehouse_id: String::new(),
            form_receipt_date: String::new(),
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
                    let query = PurchaseReceiptQuery {
                        page: Some(1),
                        page_size: Some(1000),
                        status: None,
                        supplier_id: None,
                        order_id: None,
                    };
                    match PurchaseReceiptService::list(query).await {
                        Ok(res) => link.send_message(Msg::DataLoaded(res.items)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::DataLoaded(data) => {
                self.loading = false;
                self.receipts = data;
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
                self.editing_receipt = None;
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(receipt) => {
                self.form_receipt_no = receipt.receipt_no.clone();
                self.form_order_id = receipt.order_id.to_string();
                self.form_supplier_id = receipt.supplier_id.to_string();
                self.form_warehouse_id = receipt.warehouse_id.to_string();
                self.form_receipt_date = receipt.receipt_date.clone();
                self.form_notes = receipt.notes.clone().unwrap_or_default();
                self.form_error = None;
                self.editing_receipt = Some(receipt);
                self.modal_mode = ModalMode::Edit;
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.editing_receipt = None;
                self.form_error = None;
                true
            }
            Msg::SubmitForm => {
                if self.form_receipt_no.is_empty() {
                    self.form_error = Some("收货单号不能为空".to_string());
                    return true;
                }
                if self.form_order_id.is_empty() {
                    self.form_error = Some("采购订单不能为空".to_string());
                    return true;
                }
                if self.form_supplier_id.is_empty() {
                    self.form_error = Some("供应商不能为空".to_string());
                    return true;
                }
                if self.form_warehouse_id.is_empty() {
                    self.form_error = Some("仓库不能为空".to_string());
                    return true;
                }
                if self.form_receipt_date.is_empty() {
                    self.form_error = Some("收货日期不能为空".to_string());
                    return true;
                }

                self.form_error = None;

                let order_id = self.form_order_id.parse::<i32>().unwrap_or(0);
                let supplier_id = self.form_supplier_id.parse::<i32>().unwrap_or(0);
                let warehouse_id = self.form_warehouse_id.parse::<i32>().unwrap_or(0);
                let req = CreatePurchaseReceiptRequest {
                    order_id: Some(order_id),
                    supplier_id,
                    warehouse_id,
                    receipt_date: self.form_receipt_date.clone(),
                    department_id: None,
                    inspector_id: None,
                    notes: if self.form_notes.is_empty() { None } else { Some(self.form_notes.clone()) },
                    attachment_urls: None,
                    items: vec![],
                };

                let link = ctx.link().clone();

                if self.modal_mode == ModalMode::Edit {
                    if let Some(receipt) = &self.editing_receipt {
                        let id = receipt.id;
                        let update_req = UpdatePurchaseReceiptRequest {
                            supplier_id: Some(supplier_id),
                            receipt_date: Some(self.form_receipt_date.clone()),
                            warehouse_id: Some(warehouse_id),
                            department_id: None,
                            inspector_id: None,
                            notes: if self.form_notes.is_empty() { None } else { Some(self.form_notes.clone()) },
                            attachment_urls: None,
                        };
                        spawn_local(async move {
                            match PurchaseReceiptService::update(id, update_req).await {
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
                        match PurchaseReceiptService::create(req).await {
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
                self.editing_receipt = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::DeleteReceipt(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match PurchaseReceiptService::delete(id).await {
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
            Msg::ViewReceipt(id) => {
                self.viewing_item = self.receipts.iter().find(|i| i.id == id).cloned();
                true
            }
            Msg::CloseDetailModal => {
                self.viewing_item = None;
                true
            }
            Msg::SubmitReceipt(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseReceiptService::confirm(id).await {
                        Ok(_) => {
                            toast_helper::show_success("提交成功");
                            link.send_message(Msg::LoadData);
                        }
                        Err(e) => toast_helper::show_error(&format!("提交失败: {}", e)),
                    }
                });
                false
            }
            Msg::ApproveReceipt(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseReceiptService::confirm(id).await {
                        Ok(_) => {
                            toast_helper::show_success("审批通过");
                            link.send_message(Msg::LoadData);
                        }
                        Err(e) => toast_helper::show_error(&format!("审批失败: {}", e)),
                    }
                });
                false
            }
            Msg::RejectReceipt(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseReceiptService::confirm(id).await {
                        Ok(_) => {
                            toast_helper::show_success("已驳回");
                            link.send_message(Msg::LoadData);
                        }
                        Err(e) => toast_helper::show_error(&format!("驳回失败: {}", e)),
                    }
                });
                false
            }
            Msg::FormReceiptNoChanged(v) => { self.form_receipt_no = v; true }
            Msg::FormOrderIdChanged(v) => { self.form_order_id = v; true }
            Msg::FormSupplierIdChanged(v) => { self.form_supplier_id = v; true }
            Msg::FormWarehouseIdChanged(v) => { self.form_warehouse_id = v; true }
            Msg::FormReceiptDateChanged(v) => { self.form_receipt_date = v; true }
            Msg::FormNotesChanged(v) => { self.form_notes = v; true }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="purchase-receipt-page">
                <PageHeader title={"采购收货管理".to_string()} subtitle={Some("管理所有采购收货单信息".to_string())}>
                    <PermissionGuard resource="purchase_receipt" action="create">
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                            {"+ 新建收货单"}
                        </button>
                    </PermissionGuard>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索收货单号或供应商...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载采购收货数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadData)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_receipts.is_empty() {
                    <EmptyState
                        icon={"📦".to_string()}
                        title={"暂无采购收货单数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一个采购收货单".to_string()
                        } else {
                            "没有匹配搜索条件的收货单".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"收货单号"}</th>
                                    <th>{"关联订单"}</th>
                                    <th>{"供应商"}</th>
                                    <th>{"收货日期"}</th>
                                    <th>{"状态"}</th>
                                    <th class="numeric">{"收货数量"}</th>
                                    <th class="numeric">{"收货金额"}</th>
                                    <th>{"仓库"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_receipts().iter().map(|receipt| {
                                    let receipt_clone = receipt.clone();
                                    let id = receipt.id;
                                    let status = receipt.status.clone();
                                    html! {
                                        <tr>
                                            <td>{&receipt.receipt_no}</td>
                                            <td>{receipt.order_no.as_deref().unwrap_or("-")}</td>
                                            <td>{receipt.supplier_name.as_deref().unwrap_or("-")}</td>
                                            <td>{&receipt.receipt_date}</td>
                                            <td>{&status}</td>
                                            <td class="numeric">{&receipt.total_quantity}</td>
                                            <td class="numeric">{&receipt.total_amount}</td>
                                            <td>{receipt.warehouse_name.as_deref().unwrap_or("-")}</td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-secondary"
                                                        onclick={link.callback(move |_| Msg::ViewReceipt(id))}
                                                    >
                                                        {"查看"}
                                                    </button>
                                                    if status == "DRAFT" {
                                                        <PermissionGuard resource="purchase_receipt" action="update">
                                                            <button
                                                                class="btn btn-sm btn-secondary"
                                                                onclick={link.callback(move |_| Msg::OpenEditModal(receipt_clone.clone()))}
                                                            >
                                                                {"编辑"}
                                                            </button>
                                                        </PermissionGuard>
                                                        <PermissionGuard resource="purchase_receipt" action="update">
                                                            <button
                                                                class="btn btn-sm btn-primary"
                                                                onclick={link.callback(move |_| Msg::SubmitReceipt(id))}
                                                            >
                                                                {"提交"}
                                                            </button>
                                                        </PermissionGuard>
                                                        <PermissionGuard resource="purchase_receipt" action="delete">
                                                            <button
                                                                class="btn btn-sm btn-danger"
                                                                onclick={link.callback(move |_| Msg::DeleteReceipt(id))}
                                                            >
                                                                {"删除"}
                                                            </button>
                                                        </PermissionGuard>
                                                    }
                                                    if status == "PENDING_APPROVAL" || status == "SUBMITTED" {
                                                        <PermissionGuard resource="purchase_receipt" action="approve">
                                                            <button
                                                                class="btn btn-sm btn-success"
                                                                onclick={link.callback(move |_| Msg::ApproveReceipt(id))}
                                                            >
                                                                {"通过"}
                                                            </button>
                                                        </PermissionGuard>
                                                        <PermissionGuard resource="purchase_receipt" action="approve">
                                                            <button
                                                                class="btn btn-sm btn-warning"
                                                                onclick={link.callback(move |_| Msg::RejectReceipt(id))}
                                                            >
                                                                {"驳回"}
                                                            </button>
                                                        </PermissionGuard>
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
                            total={self.filtered_receipts.len() as u64}
                            on_page_change={link.callback(|page| Msg::PageChanged(page))}
                        />
                    </div>
                }

                if self.show_modal {
                    {self.render_form_modal(ctx)}
                }

                if let Some(item) = &self.viewing_item {
                    {self.render_detail_modal(ctx, item)}
                }

                <ConfirmDialog
                    title={"确认删除".to_string()}
                    message={"确定要删除这个采购收货单吗？此操作不可撤销。".to_string()}
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

impl PurchaseReceiptPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_receipts = self.receipts.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_receipts = self.receipts.iter()
                .filter(|r| {
                    r.receipt_no.to_lowercase().contains(&keyword) ||
                    r.supplier_name.as_ref().map(|s| s.to_lowercase().contains(&keyword)).unwrap_or(false) ||
                    r.status.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_receipts(&self) -> Vec<PurchaseReceipt> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_receipts[start..end.min(self.filtered_receipts.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_receipt_no = String::new();
        self.form_order_id = String::new();
        self.form_supplier_id = String::new();
        self.form_warehouse_id = String::new();
        self.form_receipt_date = String::new();
        self.form_notes = String::new();
        self.form_error = None;
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let is_edit = self.modal_mode == ModalMode::Edit;
        let title = if is_edit { "编辑采购收货单" } else { "新建采购收货单" };

        let on_receipt_no_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormReceiptNoChanged(input.value()))
        });
        let on_order_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormOrderIdChanged(input.value()))
        });
        let on_supplier_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormSupplierIdChanged(input.value()))
        });
        let on_warehouse_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormWarehouseIdChanged(input.value()))
        });
        let on_date_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormReceiptDateChanged(input.value()))
        });
        let on_notes_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormNotesChanged(input.value()))
        });

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
                        <div class="form-group">
                            <label>{"收货单号 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_receipt_no.clone()}
                                oninput={on_receipt_no_change}
                                placeholder="请输入收货单号"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"采购订单ID *"}</label>
                            <input
                                type="number"
                                class="form-input"
                                value={self.form_order_id.clone()}
                                oninput={on_order_change}
                                placeholder="请输入采购订单ID"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"供应商ID *"}</label>
                            <input
                                type="number"
                                class="form-input"
                                value={self.form_supplier_id.clone()}
                                oninput={on_supplier_change}
                                placeholder="请输入供应商ID"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"仓库ID *"}</label>
                            <input
                                type="number"
                                class="form-input"
                                value={self.form_warehouse_id.clone()}
                                oninput={on_warehouse_change}
                                placeholder="请输入仓库ID"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"收货日期 *"}</label>
                            <input
                                type="date"
                                class="form-input"
                                value={self.form_receipt_date.clone()}
                                oninput={on_date_change}
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
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={link.callback(|_| Msg::CloseModal)}>
                            {"取消"}
                        </button>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::SubmitForm)}>
                            {if is_edit { "保存修改" } else { "创建收货单" }}
                        </button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_detail_modal(&self, ctx: &Context<Self>, item: &PurchaseReceipt) -> Html {
        html! {
            <div class="modal-overlay" onclick={ctx.link().callback(|_| Msg::CloseDetailModal)}>
                <div class="modal-content" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())} style="width: 800px; max-width: 90vw;">
                    <div class="modal-header">
                        <h2>{"采购收货单详情"}</h2>
                        <button class="close-btn" onclick={ctx.link().callback(|_| Msg::CloseDetailModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        <div class="detail-grid" style="display: grid; grid-template-columns: 1fr 1fr; gap: 1rem;">
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"收货单号: "}</span>
                                <span class="detail-value">{&item.receipt_no}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"关联订单: "}</span>
                                <span class="detail-value">{item.order_no.as_deref().unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"供应商: "}</span>
                                <span class="detail-value">{item.supplier_name.as_deref().unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"收货日期: "}</span>
                                <span class="detail-value">{&item.receipt_date}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"状态: "}</span>
                                <span class="detail-value">{&item.status}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"收货数量: "}</span>
                                <span class="detail-value">{&item.total_quantity}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"收货金额: "}</span>
                                <span class="detail-value">{&item.total_amount}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"仓库: "}</span>
                                <span class="detail-value">{item.warehouse_name.as_deref().unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"部门: "}</span>
                                <span class="detail-value">{item.department_name.as_deref().unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"备注: "}</span>
                                <span class="detail-value">{item.notes.as_deref().unwrap_or("-")}</span>
                            </div>
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-primary" onclick={ctx.link().callback(|_| Msg::CloseDetailModal)}>{"关闭"}</button>
                    </div>
                </div>
            </div>
        }
    }
}
