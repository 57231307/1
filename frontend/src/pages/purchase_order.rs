// 采购订单管理页面

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
use crate::models::purchase_order::{PurchaseOrder, PurchaseOrderQuery, CreatePurchaseOrderRequest};
use crate::services::purchase_order_service::PurchaseOrderService;
use crate::services::crud_service::CrudService;

pub struct PurchaseOrderPage {
    orders: Vec<PurchaseOrder>,
    filtered_orders: Vec<PurchaseOrder>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_order: Option<PurchaseOrder>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    viewing_item: Option<PurchaseOrder>,
    printing_order: Option<PurchaseOrder>,
    print_trigger: bool,
    // 表单字段
    form_supplier_id: String,
    form_order_date: String,
    form_expected_delivery_date: String,
    form_warehouse_id: String,
    form_department_id: String,
    form_currency: String,
    form_payment_terms: String,
    form_shipping_terms: String,
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
    DataLoaded(Vec<PurchaseOrder>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    OpenEditModal(PurchaseOrder),
    CloseModal,
    SubmitForm,
    FormSubmitted,
    DeleteOrder(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    ViewOrder(i32),
    CloseDetailModal,
    PrintOrder(PurchaseOrder),
    ClearPrint,
    SubmitOrder(i32),
    ApproveOrder(i32),
    RejectOrder(i32),
    CloseOrder(i32),
    // 表单字段变更
    FormSupplierIdChanged(String),
    FormOrderDateChanged(String),
    FormExpectedDeliveryDateChanged(String),
    FormWarehouseIdChanged(String),
    FormDepartmentIdChanged(String),
    FormCurrencyChanged(String),
    FormPaymentTermsChanged(String),
    FormShippingTermsChanged(String),
    FormNotesChanged(String),
}

impl Component for PurchaseOrderPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            orders: Vec::new(),
            filtered_orders: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            show_modal: false,
            modal_mode: ModalMode::Create,
            editing_order: None,
            show_delete_confirm: false,
            deleting_id: None,
            viewing_item: None,
            printing_order: None,
            print_trigger: false,
            form_supplier_id: String::new(),
            form_order_date: String::new(),
            form_expected_delivery_date: String::new(),
            form_warehouse_id: String::new(),
            form_department_id: String::new(),
            form_currency: "CNY".to_string(),
            form_payment_terms: String::new(),
            form_shipping_terms: String::new(),
            form_notes: String::new(),
            form_error: None,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadData);
        }
        if self.print_trigger {
            self.print_trigger = false;
            if let Some(window) = web_sys::window() {
                let _ = window.print();
            }
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadData => {
                self.loading = true;
                self.error = None;
                let link = ctx.link().clone();
                spawn_local(async move {
                    let query = PurchaseOrderQuery {
                        page: Some(1),
                        page_size: Some(1000),
                        status: None,
                        supplier_id: None,
                    };
                    match PurchaseOrderService::list(query).await {
                        Ok(res) => link.send_message(Msg::DataLoaded(res)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::DataLoaded(data) => {
                self.loading = false;
                self.orders = data;
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
                self.editing_order = None;
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(order) => {
                self.form_supplier_id = order.supplier_id.to_string();
                self.form_order_date = order.order_date.clone();
                self.form_expected_delivery_date = order.expected_delivery_date.clone().unwrap_or_default();
                self.form_warehouse_id = order.warehouse_id.to_string();
                self.form_department_id = order.department_id.to_string();
                self.form_currency = order.currency.clone().unwrap_or_else(|| "CNY".to_string());
                self.form_payment_terms = order.payment_terms.clone().unwrap_or_default();
                self.form_shipping_terms = order.shipping_terms.clone().unwrap_or_default();
                self.form_notes = order.notes.clone().unwrap_or_default();
                self.form_error = None;
                self.editing_order = Some(order);
                self.modal_mode = ModalMode::Edit;
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.editing_order = None;
                self.form_error = None;
                true
            }
            Msg::SubmitForm => {
                if self.form_supplier_id.is_empty() {
                    self.form_error = Some("供应商不能为空".to_string());
                    return true;
                }
                if self.form_order_date.is_empty() {
                    self.form_error = Some("订单日期不能为空".to_string());
                    return true;
                }
                if self.form_warehouse_id.is_empty() {
                    self.form_error = Some("仓库不能为空".to_string());
                    return true;
                }

                self.form_error = None;

                let supplier_id = self.form_supplier_id.parse::<i32>().unwrap_or(0);
                let warehouse_id = self.form_warehouse_id.parse::<i32>().unwrap_or(0);
                let department_id = if self.form_department_id.is_empty() { None } else { Some(self.form_department_id.parse::<i32>().unwrap_or(0)) };

                let req = CreatePurchaseOrderRequest {
                    supplier_id,
                    order_date: self.form_order_date.clone(),
                    expected_delivery_date: if self.form_expected_delivery_date.is_empty() { None } else { Some(self.form_expected_delivery_date.clone()) },
                    warehouse_id,
                    department_id: department_id.unwrap_or(0),
                    currency: if self.form_currency.is_empty() { None } else { Some(self.form_currency.clone()) },
                    exchange_rate: None,
                    payment_terms: if self.form_payment_terms.is_empty() { None } else { Some(self.form_payment_terms.clone()) },
                    shipping_terms: if self.form_shipping_terms.is_empty() { None } else { Some(self.form_shipping_terms.clone()) },
                    notes: if self.form_notes.is_empty() { None } else { Some(self.form_notes.clone()) },
                    attachment_urls: None,
                    items: vec![],
                };

                let link = ctx.link().clone();

                if self.modal_mode == ModalMode::Edit {
                    if let Some(order) = &self.editing_order {
                        let id = order.id;
                        let update_req = crate::models::purchase_order::UpdatePurchaseOrderRequest {
                            supplier_id: Some(supplier_id),
                            order_date: Some(self.form_order_date.clone()),
                            expected_delivery_date: if self.form_expected_delivery_date.is_empty() { None } else { Some(self.form_expected_delivery_date.clone()) },
                            warehouse_id: Some(warehouse_id),
                            department_id,
                            currency: if self.form_currency.is_empty() { None } else { Some(self.form_currency.clone()) },
                            exchange_rate: None,
                            payment_terms: if self.form_payment_terms.is_empty() { None } else { Some(self.form_payment_terms.clone()) },
                            shipping_terms: if self.form_shipping_terms.is_empty() { None } else { Some(self.form_shipping_terms.clone()) },
                            notes: if self.form_notes.is_empty() { None } else { Some(self.form_notes.clone()) },
                            attachment_urls: None,
                        };
                        spawn_local(async move {
                            match PurchaseOrderService::update(id, update_req).await {
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
                        match PurchaseOrderService::create(req).await {
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
                self.editing_order = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::DeleteOrder(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match PurchaseOrderService::delete(id).await {
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
            Msg::ViewOrder(id) => {
                self.viewing_item = self.orders.iter().find(|i| i.id == id).cloned();
                true
            }
            Msg::CloseDetailModal => {
                self.viewing_item = None;
                true
            }
            Msg::PrintOrder(order) => {
                self.printing_order = Some(order);
                self.print_trigger = true;
                true
            }
            Msg::ClearPrint => {
                self.printing_order = None;
                self.print_trigger = false;
                true
            }
            Msg::SubmitOrder(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseOrderService::submit(id).await {
                        Ok(_) => {
                            toast_helper::show_success("提交成功");
                            link.send_message(Msg::LoadData);
                        }
                        Err(e) => toast_helper::show_error(&format!("提交失败: {}", e)),
                    }
                });
                false
            }
            Msg::ApproveOrder(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseOrderService::approve(id).await {
                        Ok(_) => {
                            toast_helper::show_success("审批通过");
                            link.send_message(Msg::LoadData);
                        }
                        Err(e) => toast_helper::show_error(&format!("审批失败: {}", e)),
                    }
                });
                false
            }
            Msg::RejectOrder(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseOrderService::reject(id, "不符合要求".to_string()).await {
                        Ok(_) => {
                            toast_helper::show_success("已驳回");
                            link.send_message(Msg::LoadData);
                        }
                        Err(e) => toast_helper::show_error(&format!("驳回失败: {}", e)),
                    }
                });
                false
            }
            Msg::CloseOrder(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseOrderService::close(id).await {
                        Ok(_) => {
                            toast_helper::show_success("订单已关闭");
                            link.send_message(Msg::LoadData);
                        }
                        Err(e) => toast_helper::show_error(&format!("关闭失败: {}", e)),
                    }
                });
                false
            }
            Msg::FormSupplierIdChanged(v) => { self.form_supplier_id = v; true }
            Msg::FormOrderDateChanged(v) => { self.form_order_date = v; true }
            Msg::FormExpectedDeliveryDateChanged(v) => { self.form_expected_delivery_date = v; true }
            Msg::FormWarehouseIdChanged(v) => { self.form_warehouse_id = v; true }
            Msg::FormDepartmentIdChanged(v) => { self.form_department_id = v; true }
            Msg::FormCurrencyChanged(v) => { self.form_currency = v; true }
            Msg::FormPaymentTermsChanged(v) => { self.form_payment_terms = v; true }
            Msg::FormShippingTermsChanged(v) => { self.form_shipping_terms = v; true }
            Msg::FormNotesChanged(v) => { self.form_notes = v; true }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="purchase-order-page">
                <PageHeader title={"采购订单管理".to_string()} subtitle={Some("管理所有采购订单信息".to_string())}>
                    <PermissionGuard resource="purchase_order" action="create">
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                            {"+ 新建采购订单"}
                        </button>
                    </PermissionGuard>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索订单编号或供应商...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载采购订单数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadData)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_orders.is_empty() {
                    <EmptyState
                        icon={"📦".to_string()}
                        title={"暂无采购订单数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一个采购订单".to_string()
                        } else {
                            "没有匹配搜索条件的订单".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"订单编号"}</th>
                                    <th>{"供应商"}</th>
                                    <th>{"订单日期"}</th>
                                    <th>{"要求交货日期"}</th>
                                    <th>{"订单状态"}</th>
                                    <th class="numeric">{"总金额"}</th>
                                    <th>{"仓库"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_orders().iter().map(|order| {
                                    let order_clone = order.clone();
                                    let order_clone2 = order.clone();
                                    let id = order.id;
                                    let status = order.status.clone();
                                    html! {
                                        <tr>
                                            <td>{&order.order_no}</td>
                                            <td>{order.supplier_name.as_deref().unwrap_or("-")}</td>
                                            <td>{&order.order_date}</td>
                                            <td>{order.expected_delivery_date.as_deref().unwrap_or("-")}</td>
                                            <td>{&status}</td>
                                            <td class="numeric">{&order.total_amount}</td>
                                            <td>{order.warehouse_name.as_deref().unwrap_or("-")}</td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-secondary"
                                                        onclick={link.callback(move |_| Msg::ViewOrder(id))}
                                                    >
                                                        {"查看"}
                                                    </button>
                                                    if status == "DRAFT" || status == "REJECTED" {
                                                        <PermissionGuard resource="purchase_order" action="update">
                                                            <button
                                                                class="btn btn-sm btn-secondary"
                                                                onclick={link.callback(move |_| Msg::OpenEditModal(order_clone.clone()))}
                                                            >
                                                                {"编辑"}
                                                            </button>
                                                        </PermissionGuard>
                                                        <PermissionGuard resource="purchase_order" action="update">
                                                            <button
                                                                class="btn btn-sm btn-primary"
                                                                onclick={link.callback(move |_| Msg::SubmitOrder(id))}
                                                            >
                                                                {"提交"}
                                                            </button>
                                                        </PermissionGuard>
                                                    }
                                                    if status == "PENDING_APPROVAL" || status == "SUBMITTED" {
                                                        <PermissionGuard resource="purchase_order" action="approve">
                                                            <button
                                                                class="btn btn-sm btn-success"
                                                                onclick={link.callback(move |_| Msg::ApproveOrder(id))}
                                                            >
                                                                {"通过"}
                                                            </button>
                                                        </PermissionGuard>
                                                        <PermissionGuard resource="purchase_order" action="approve">
                                                            <button
                                                                class="btn btn-sm btn-warning"
                                                                onclick={link.callback(move |_| Msg::RejectOrder(id))}
                                                            >
                                                                {"驳回"}
                                                            </button>
                                                        </PermissionGuard>
                                                    }
                                                    if status == "APPROVED" {
                                                        <PermissionGuard resource="purchase_order" action="update">
                                                            <button
                                                                class="btn btn-sm btn-secondary"
                                                                onclick={link.callback(move |_| Msg::CloseOrder(id))}
                                                            >
                                                                {"关闭"}
                                                            </button>
                                                        </PermissionGuard>
                                                    }
                                                    <button
                                                        class="btn btn-sm btn-secondary"
                                                        onclick={link.callback(move |_| Msg::PrintOrder(order_clone2.clone()))}
                                                    >
                                                        {"打印"}
                                                    </button>
                                                    if status == "DRAFT" {
                                                        <PermissionGuard resource="purchase_order" action="delete">
                                                            <button
                                                                class="btn btn-sm btn-danger"
                                                                onclick={link.callback(move |_| Msg::DeleteOrder(id))}
                                                            >
                                                                {"删除"}
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
                            total={self.filtered_orders.len() as u64}
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

                {self.render_print_view()}

                <ConfirmDialog
                    title={"确认删除".to_string()}
                    message={"确定要删除这个采购订单吗？此操作不可撤销。".to_string()}
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

impl PurchaseOrderPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_orders = self.orders.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_orders = self.orders.iter()
                .filter(|o| {
                    o.order_no.to_lowercase().contains(&keyword) ||
                    o.supplier_name.as_ref().map(|s| s.to_lowercase().contains(&keyword)).unwrap_or(false) ||
                    o.status.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_orders(&self) -> Vec<PurchaseOrder> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_orders[start..end.min(self.filtered_orders.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_supplier_id = String::new();
        self.form_order_date = String::new();
        self.form_expected_delivery_date = String::new();
        self.form_warehouse_id = String::new();
        self.form_department_id = String::new();
        self.form_currency = "CNY".to_string();
        self.form_payment_terms = String::new();
        self.form_shipping_terms = String::new();
        self.form_notes = String::new();
        self.form_error = None;
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let is_edit = self.modal_mode == ModalMode::Edit;
        let title = if is_edit { "编辑采购订单" } else { "新建采购订单" };

        let on_supplier_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormSupplierIdChanged(input.value()))
        });
        let on_date_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormOrderDateChanged(input.value()))
        });
        let on_delivery_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormExpectedDeliveryDateChanged(input.value()))
        });
        let on_warehouse_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormWarehouseIdChanged(input.value()))
        });
        let on_department_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormDepartmentIdChanged(input.value()))
        });
        let on_currency_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormCurrencyChanged(input.value()))
        });
        let on_payment_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormPaymentTermsChanged(input.value()))
        });
        let on_shipping_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormShippingTermsChanged(input.value()))
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
                            <label>{"供应商ID *"}</label>
                            <input
                                type="number"
                                class="form-input"
                                value={self.form_supplier_id.clone()}
                                oninput={on_supplier_change}
                                placeholder="请输入供应商ID"
                            />
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"订单日期 *"}</label>
                                <input
                                    type="date"
                                    class="form-input"
                                    value={self.form_order_date.clone()}
                                    oninput={on_date_change}
                                />
                            </div>
                            <div class="form-group">
                                <label>{"要求交货日期"}</label>
                                <input
                                    type="date"
                                    class="form-input"
                                    value={self.form_expected_delivery_date.clone()}
                                    oninput={on_delivery_change}
                                />
                            </div>
                        </div>
                        <div class="form-row">
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
                                <label>{"部门ID"}</label>
                                <input
                                    type="number"
                                    class="form-input"
                                    value={self.form_department_id.clone()}
                                    oninput={on_department_change}
                                    placeholder="请输入部门ID"
                                />
                            </div>
                        </div>
                        <div class="form-group">
                            <label>{"币种"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_currency.clone()}
                                oninput={on_currency_change}
                                placeholder="如：CNY、USD"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"付款条款"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_payment_terms.clone()}
                                oninput={on_payment_change}
                                placeholder="请输入付款条款"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"运输条款"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_shipping_terms.clone()}
                                oninput={on_shipping_change}
                                placeholder="请输入运输条款"
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
                            {if is_edit { "保存修改" } else { "创建订单" }}
                        </button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_detail_modal(&self, ctx: &Context<Self>, item: &PurchaseOrder) -> Html {
        html! {
            <div class="modal-overlay" onclick={ctx.link().callback(|_| Msg::CloseDetailModal)}>
                <div class="modal-content" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())} style="width: 800px; max-width: 90vw;">
                    <div class="modal-header">
                        <h2>{"采购订单详情"}</h2>
                        <button class="close-btn" onclick={ctx.link().callback(|_| Msg::CloseDetailModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        <div class="detail-grid" style="display: grid; grid-template-columns: 1fr 1fr; gap: 1rem;">
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"订单编号: "}</span>
                                <span class="detail-value">{&item.order_no}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"供应商: "}</span>
                                <span class="detail-value">{item.supplier_name.as_deref().unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"订单日期: "}</span>
                                <span class="detail-value">{&item.order_date}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"要求交货日期: "}</span>
                                <span class="detail-value">{item.expected_delivery_date.as_deref().unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"状态: "}</span>
                                <span class="detail-value">{&item.status}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"总金额: "}</span>
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
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"币种: "}</span>
                                <span class="detail-value">{item.currency.as_deref().unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"付款条款: "}</span>
                                <span class="detail-value">{item.payment_terms.as_deref().unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"运输条款: "}</span>
                                <span class="detail-value">{item.shipping_terms.as_deref().unwrap_or("-")}</span>
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

    fn render_print_view(&self) -> Html {
        if let Some(order) = &self.printing_order {
            html! {
                <div class="print-view" style="display: none;">
                    <style>
                    {r#"
                    @media print {
                        body * { visibility: hidden; }
                        .print-view, .print-view * { visibility: visible; }
                        .print-view { position: absolute; left: 0; top: 0; width: 100%; display: block !important; padding: 20px; }
                        .no-print { display: none !important; }
                        body { font-size: 11pt; }
                        table { border-collapse: collapse; width: 100%; }
                        th, td { border: 1px solid #333; padding: 6px; }
                    }
                    "#}
                    </style>
                    <div class="print-header" style="text-align: center; margin-bottom: 20px;">
                        <h2>{"采购订单"}</h2>
                        <p>{"单号: "}{&order.order_no}</p>
                    </div>
                    <div class="print-info-grid" style="display: grid; grid-template-columns: 1fr 1fr; gap: 10px; margin-bottom: 20px;">
                        <div><strong>{"供应商："}</strong> {order.supplier_name.as_deref().unwrap_or("-")}</div>
                        <div><strong>{"订单日期："}</strong> {&order.order_date}</div>
                        <div><strong>{"要求交货期："}</strong> {order.expected_delivery_date.as_deref().unwrap_or("-")}</div>
                        <div><strong>{"采购总金额："}</strong> {&order.total_amount} {order.currency.as_deref().unwrap_or("")}</div>
                    </div>
                    <div class="print-footer" style="margin-top: 40px; display: flex; justify-content: space-around;">
                        <div>{"制单人签字"}</div>
                        <div>{"审批人签字"}</div>
                        <div>{"供应商确认盖章"}</div>
                    </div>
                </div>
            }
        } else {
            html! {}
        }
    }
}
