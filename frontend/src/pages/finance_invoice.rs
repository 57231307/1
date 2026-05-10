// 财务发票管理页面

use crate::utils::toast_helper;
use yew::prelude::*;
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
use crate::models::finance_invoice::{
    FinanceInvoice, InvoiceQueryParams, CreateInvoiceRequest, UpdateInvoiceRequest,
};
use crate::services::finance_invoice_service::FinanceInvoiceService;
use crate::services::crud_service::CrudService;

/// 财务发票管理页面状态
pub struct FinanceInvoicePage {
    invoices: Vec<FinanceInvoice>,
    filtered_invoices: Vec<FinanceInvoice>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_invoice: Option<FinanceInvoice>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    viewing_item: Option<FinanceInvoice>,
    filter_status: String,
    filter_type: String,
    // 表单字段
    form_invoice_no: String,
    form_customer_name: String,
    form_invoice_type: String,
    form_amount: String,
    form_tax_amount: String,
    form_total_amount: String,
    form_status: String,
    form_invoice_date: String,
    form_due_date: String,
    form_payment_method: String,
    form_notes: String,
    form_error: Option<String>,
}

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    Create,
    Edit,
}

/// 消息枚举
pub enum Msg {
    LoadInvoices,
    InvoicesLoaded(Vec<FinanceInvoice>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    SetFilterStatus(String),
    SetFilterType(String),
    OpenCreateModal,
    OpenEditModal(FinanceInvoice),
    CloseModal,
    CloseDetailModal,
    ViewInvoice(i32),
    SubmitForm,
    FormSubmitted,
    DeleteInvoice(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    ApproveInvoice(i32),
    VerifyInvoice(i32),
    Refresh,
    // 表单字段变更
    FormInvoiceNoChanged(String),
    FormCustomerNameChanged(String),
    FormInvoiceTypeChanged(String),
    FormAmountChanged(String),
    FormTaxAmountChanged(String),
    FormTotalAmountChanged(String),
    FormStatusChanged(String),
    FormInvoiceDateChanged(String),
    FormDueDateChanged(String),
    FormPaymentMethodChanged(String),
    FormNotesChanged(String),
}

impl Component for FinanceInvoicePage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            invoices: Vec::new(),
            filtered_invoices: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            show_modal: false,
            modal_mode: ModalMode::Create,
            editing_invoice: None,
            show_delete_confirm: false,
            deleting_id: None,
            viewing_item: None,
            filter_status: String::from("全部"),
            filter_type: String::from("全部"),
            form_invoice_no: String::new(),
            form_customer_name: String::new(),
            form_invoice_type: String::new(),
            form_amount: String::new(),
            form_tax_amount: String::new(),
            form_total_amount: String::new(),
            form_status: String::from("草稿"),
            form_invoice_date: String::new(),
            form_due_date: String::new(),
            form_payment_method: String::new(),
            form_notes: String::new(),
            form_error: None,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadInvoices);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadInvoices => {
                self.loading = true;
                self.error = None;
                let params = InvoiceQueryParams {
                    customer_id: None,
                    status: if self.filter_status == "全部" { None } else { Some(self.filter_status.clone()) },
                    invoice_type: if self.filter_type == "全部" { None } else { Some(self.filter_type.clone()) },
                    start_date: None,
                    end_date: None,
                    page: Some(1),
                    page_size: Some(1000),
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match FinanceInvoiceService::list_invoices(params).await {
                        Ok(response) => link.send_message(Msg::InvoicesLoaded(response.invoices)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::InvoicesLoaded(invoices) => {
                self.invoices = invoices;
                self.loading = false;
                self.apply_filter();
                true
            }
            Msg::LoadError(e) => {
                self.error = Some(e);
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
            Msg::SetFilterStatus(status) => {
                self.filter_status = status;
                self.page = 0;
                ctx.link().send_message(Msg::LoadInvoices);
                false
            }
            Msg::SetFilterType(tp) => {
                self.filter_type = tp;
                self.page = 0;
                ctx.link().send_message(Msg::LoadInvoices);
                false
            }
            Msg::OpenCreateModal => {
                self.reset_form();
                self.editing_invoice = None;
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(invoice) => {
                self.form_invoice_no = invoice.invoice_no.clone();
                self.form_customer_name = invoice.customer_name.clone();
                self.form_invoice_type = invoice.invoice_type.clone();
                self.form_amount = invoice.amount.clone();
                self.form_tax_amount = invoice.tax_amount.clone();
                self.form_total_amount = invoice.total_amount.clone();
                self.form_status = invoice.status.clone();
                self.form_invoice_date = invoice.invoice_date.clone().unwrap_or_default();
                self.form_due_date = invoice.due_date.clone().unwrap_or_default();
                self.form_payment_method = invoice.payment_method.clone().unwrap_or_default();
                self.form_notes = invoice.notes.clone().unwrap_or_default();
                self.form_error = None;
                self.editing_invoice = Some(invoice);
                self.modal_mode = ModalMode::Edit;
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.editing_invoice = None;
                self.form_error = None;
                true
            }
            Msg::CloseDetailModal => {
                self.viewing_item = None;
                true
            }
            Msg::ViewInvoice(id) => {
                self.viewing_item = self.invoices.iter().find(|i| i.id == id).cloned();
                true
            }
            Msg::SubmitForm => {
                if self.form_invoice_no.is_empty() {
                    self.form_error = Some("发票编号不能为空".to_string());
                    return true;
                }
                if self.form_customer_name.is_empty() {
                    self.form_error = Some("客户名称不能为空".to_string());
                    return true;
                }
                if self.form_invoice_type.is_empty() {
                    self.form_error = Some("发票类型不能为空".to_string());
                    return true;
                }
                if self.form_amount.is_empty() {
                    self.form_error = Some("金额不能为空".to_string());
                    return true;
                }

                self.form_error = None;

                if self.modal_mode == ModalMode::Edit {
                    if let Some(invoice) = &self.editing_invoice {
                        let id = invoice.id;
                        let req = UpdateInvoiceRequest {
                            invoice_no: Some(self.form_invoice_no.clone()),
                            order_id: None,
                            customer_id: None,
                            customer_name: Some(self.form_customer_name.clone()),
                            invoice_type: Some(self.form_invoice_type.clone()),
                            amount: Some(self.form_amount.clone()),
                            tax_amount: Some(self.form_tax_amount.clone()),
                            total_amount: Some(self.form_total_amount.clone()),
                            status: Some(self.form_status.clone()),
                            invoice_date: if self.form_invoice_date.is_empty() { None } else { Some(self.form_invoice_date.clone()) },
                            due_date: if self.form_due_date.is_empty() { None } else { Some(self.form_due_date.clone()) },
                            paid_date: None,
                            payment_method: if self.form_payment_method.is_empty() { None } else { Some(self.form_payment_method.clone()) },
                            notes: if self.form_notes.is_empty() { None } else { Some(self.form_notes.clone()) },
                        };
                        let link = ctx.link().clone();
                        spawn_local(async move {
                            match FinanceInvoiceService::update_invoice(id, req).await {
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
                    let req = CreateInvoiceRequest {
                        invoice_no: self.form_invoice_no.clone(),
                        order_id: None,
                        customer_id: None,
                        customer_name: self.form_customer_name.clone(),
                        invoice_type: self.form_invoice_type.clone(),
                        amount: self.form_amount.clone(),
                        tax_amount: self.form_tax_amount.clone(),
                        total_amount: self.form_total_amount.clone(),
                        status: Some(self.form_status.clone()),
                        invoice_date: if self.form_invoice_date.is_empty() { None } else { Some(self.form_invoice_date.clone()) },
                        due_date: if self.form_due_date.is_empty() { None } else { Some(self.form_due_date.clone()) },
                        payment_method: if self.form_payment_method.is_empty() { None } else { Some(self.form_payment_method.clone()) },
                        notes: if self.form_notes.is_empty() { None } else { Some(self.form_notes.clone()) },
                    };
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match FinanceInvoiceService::create_invoice(req).await {
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
                self.editing_invoice = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadInvoices);
                false
            }
            Msg::DeleteInvoice(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match FinanceInvoiceService::delete_invoice(id).await {
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
                ctx.link().send_message(Msg::LoadInvoices);
                false
            }
            Msg::ApproveInvoice(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match FinanceInvoiceService::approve_invoice(id).await {
                        Ok(_) => {
                            toast_helper::show_success("审核成功");
                            link.send_message(Msg::LoadInvoices);
                        }
                        Err(e) => {
                            toast_helper::show_error(&format!("审核失败: {}", e));
                        }
                    }
                });
                false
            }
            Msg::VerifyInvoice(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match FinanceInvoiceService::verify_invoice(id, "银行转账".to_string()).await {
                        Ok(_) => {
                            toast_helper::show_success("核销成功");
                            link.send_message(Msg::LoadInvoices);
                        }
                        Err(e) => {
                            toast_helper::show_error(&format!("核销失败: {}", e));
                        }
                    }
                });
                false
            }
            Msg::Refresh => {
                ctx.link().send_message(Msg::LoadInvoices);
                false
            }
            Msg::FormInvoiceNoChanged(v) => { self.form_invoice_no = v; true }
            Msg::FormCustomerNameChanged(v) => { self.form_customer_name = v; true }
            Msg::FormInvoiceTypeChanged(v) => { self.form_invoice_type = v; true }
            Msg::FormAmountChanged(v) => { self.form_amount = v; true }
            Msg::FormTaxAmountChanged(v) => { self.form_tax_amount = v; true }
            Msg::FormTotalAmountChanged(v) => { self.form_total_amount = v; true }
            Msg::FormStatusChanged(v) => { self.form_status = v; true }
            Msg::FormInvoiceDateChanged(v) => { self.form_invoice_date = v; true }
            Msg::FormDueDateChanged(v) => { self.form_due_date = v; true }
            Msg::FormPaymentMethodChanged(v) => { self.form_payment_method = v; true }
            Msg::FormNotesChanged(v) => { self.form_notes = v; true }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        let on_status_change = link.batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlSelectElement>().ok()?;
            Some(Msg::SetFilterStatus(target.value()))
        });

        let on_type_change = link.batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlSelectElement>().ok()?;
            Some(Msg::SetFilterType(target.value()))
        });

        let paginated = self.paginated_invoices();
        let total_count = self.filtered_invoices.len() as u64;

        html! {
            <div class="finance-invoice-page">
                <PageHeader title={"财务发票管理".to_string()} subtitle={Some("管理财务发票信息".to_string())}>
                    <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                        {"+ 新建发票"}
                    </button>
                </PageHeader>

                <div class="page-toolbar">
                    <div class="filter-bar">
                        <div class="filter-item">
                            <label>{"发票状态："}</label>
                            <select value={self.filter_status.clone()} onchange={on_status_change}>
                                <option value="全部">{"全部"}</option>
                                <option value="草稿">{"草稿"}</option>
                                <option value="待审核">{"待审核"}</option>
                                <option value="已审核">{"已审核"}</option>
                                <option value="已付款">{"已付款"}</option>
                                <option value="已核销">{"已核销"}</option>
                                <option value="已作废">{"已作废"}</option>
                            </select>
                        </div>
                        <div class="filter-item">
                            <label>{"发票类型："}</label>
                            <select value={self.filter_type.clone()} onchange={on_type_change}>
                                <option value="全部">{"全部"}</option>
                                <option value="增值税专用发票">{"增值税专用发票"}</option>
                                <option value="增值税普通发票">{"增值税普通发票"}</option>
                                <option value="电子发票">{"电子发票"}</option>
                                <option value="收据">{"收据"}</option>
                            </select>
                        </div>
                    </div>
                    <SearchBar
                        placeholder={"搜索发票号、客户名称...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载发票数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadInvoices)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_invoices.is_empty() {
                    <EmptyState
                        icon={"📋".to_string()}
                        title={"暂无财务发票".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一张发票".to_string()
                        } else {
                            "没有匹配搜索条件的发票".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"发票编号"}</th>
                                    <th>{"客户名称"}</th>
                                    <th>{"发票类型"}</th>
                                    <th>{"发票日期"}</th>
                                    <th>{"到期日期"}</th>
                                    <th>{"状态"}</th>
                                    <th class="numeric">{"金额"}</th>
                                    <th class="numeric">{"税额"}</th>
                                    <th class="numeric">{"价税合计"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for paginated.iter().map(|invoice| {
                                    let invoice_clone = invoice.clone();
                                    let id = invoice.id;
                                    let status = invoice.status.clone();
                                    let status_class = match status.as_str() {
                                        "草稿" => "status-badge status-draft",
                                        "待审核" => "status-badge status-pending",
                                        "已审核" => "status-badge status-approved",
                                        "已付款" => "status-badge status-paid",
                                        "已核销" => "status-badge status-verified",
                                        "已作废" => "status-badge status-cancelled",
                                        _ => "status-badge",
                                    };
                                    html! {
                                        <tr>
                                            <td>{&invoice.invoice_no}</td>
                                            <td>{&invoice.customer_name}</td>
                                            <td>{&invoice.invoice_type}</td>
                                            <td>{invoice.invoice_date.as_deref().unwrap_or("-")}</td>
                                            <td>{invoice.due_date.as_deref().unwrap_or("-")}</td>
                                            <td>
                                                <span class={status_class}>{&status}</span>
                                            </td>
                                            <td class="numeric">{&invoice.amount}</td>
                                            <td class="numeric">{&invoice.tax_amount}</td>
                                            <td class="numeric">{&invoice.total_amount}</td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-info"
                                                        onclick={link.callback(move |_| Msg::ViewInvoice(id))}
                                                    >
                                                        {"查看"}
                                                    </button>
                                                    if status == "草稿" {
                                                        <button
                                                            class="btn btn-sm btn-secondary"
                                                            onclick={link.callback(move |_| Msg::OpenEditModal(invoice_clone.clone()))}
                                                        >
                                                            {"编辑"}
                                                        </button>
                                                    }
                                                    if status == "待审核" {
                                                        <button
                                                            class="btn btn-sm btn-primary"
                                                            onclick={link.callback(move |_| Msg::ApproveInvoice(id))}
                                                        >
                                                            {"审核"}
                                                        </button>
                                                    }
                                                    if status == "已审核" {
                                                        <button
                                                            class="btn btn-sm btn-success"
                                                            onclick={link.callback(move |_| Msg::VerifyInvoice(id))}
                                                        >
                                                            {"核销"}
                                                        </button>
                                                    }
                                                    <button
                                                        class="btn btn-sm btn-danger"
                                                        onclick={link.callback(move |_| Msg::DeleteInvoice(id))}
                                                    >
                                                        {"删除"}
                                                    </button>
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
                            total={total_count}
                            on_page_change={link.callback(|page| Msg::PageChanged(page))}
                        />
                    </div>
                }

                // 新建/编辑弹窗
                if self.show_modal {
                    {self.render_form_modal(ctx)}
                }

                // 详情弹窗
                if let Some(ref item) = self.viewing_item {
                    {self.render_detail_modal(ctx, item)}
                }

                // 删除确认对话框
                <ConfirmDialog
                    title={"确认删除".to_string()}
                    message={"确定要删除这张发票吗？此操作不可撤销。".to_string()}
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

impl FinanceInvoicePage {
    fn apply_filter(&mut self) {
        let mut filtered = self.invoices.clone();

        if !self.search_keyword.is_empty() {
            let keyword = self.search_keyword.to_lowercase();
            filtered = filtered.into_iter()
                .filter(|i| {
                    i.invoice_no.to_lowercase().contains(&keyword) ||
                    i.customer_name.to_lowercase().contains(&keyword) ||
                    i.invoice_type.to_lowercase().contains(&keyword)
                })
                .collect();
        }

        self.filtered_invoices = filtered;
    }

    fn paginated_invoices(&self) -> Vec<FinanceInvoice> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_invoices[start..end.min(self.filtered_invoices.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_invoice_no = String::new();
        self.form_customer_name = String::new();
        self.form_invoice_type = String::new();
        self.form_amount = String::new();
        self.form_tax_amount = String::new();
        self.form_total_amount = String::new();
        self.form_status = "草稿".to_string();
        self.form_invoice_date = String::new();
        self.form_due_date = String::new();
        self.form_payment_method = String::new();
        self.form_notes = String::new();
        self.form_error = None;
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let is_edit = self.modal_mode == ModalMode::Edit;
        let title = if is_edit { "编辑财务发票" } else { "新建财务发票" };

        let on_invoice_no_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormInvoiceNoChanged(input.value()))
        });
        let on_customer_name_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormCustomerNameChanged(input.value()))
        });
        let on_invoice_type_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormInvoiceTypeChanged(input.value()))
        });
        let on_amount_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormAmountChanged(input.value()))
        });
        let on_tax_amount_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormTaxAmountChanged(input.value()))
        });
        let on_total_amount_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormTotalAmountChanged(input.value()))
        });
        let on_status_change = link.batch_callback(|e: Event| {
            e.target().and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok()).map(|select| Msg::FormStatusChanged(select.value()))
        });
        let on_invoice_date_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormInvoiceDateChanged(input.value()))
        });
        let on_due_date_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormDueDateChanged(input.value()))
        });
        let on_payment_method_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormPaymentMethodChanged(input.value()))
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
                            <label>{"发票编号 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_invoice_no.clone()}
                                oninput={on_invoice_no_change}
                                placeholder="请输入发票编号"
                                disabled={is_edit}
                            />
                        </div>
                        <div class="form-group">
                            <label>{"客户名称 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_customer_name.clone()}
                                oninput={on_customer_name_change}
                                placeholder="请输入客户名称"
                            />
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"发票类型 *"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_invoice_type.clone()}
                                    oninput={on_invoice_type_change}
                                    placeholder="如：增值税专用发票"
                                />
                            </div>
                            <div class="form-group">
                                <label>{"状态"}</label>
                                <select class="form-input" value={self.form_status.clone()} onchange={on_status_change}>
                                    <option value="草稿">{"草稿"}</option>
                                    <option value="待审核">{"待审核"}</option>
                                    <option value="已审核">{"已审核"}</option>
                                    <option value="已付款">{"已付款"}</option>
                                    <option value="已核销">{"已核销"}</option>
                                </select>
                            </div>
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"金额 *"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_amount.clone()}
                                    oninput={on_amount_change}
                                    placeholder="请输入金额"
                                />
                            </div>
                            <div class="form-group">
                                <label>{"税额"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_tax_amount.clone()}
                                    oninput={on_tax_amount_change}
                                    placeholder="请输入税额"
                                />
                            </div>
                            <div class="form-group">
                                <label>{"价税合计"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_total_amount.clone()}
                                    oninput={on_total_amount_change}
                                    placeholder="请输入价税合计"
                                />
                            </div>
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"发票日期"}</label>
                                <input
                                    type="date"
                                    class="form-input"
                                    value={self.form_invoice_date.clone()}
                                    oninput={on_invoice_date_change}
                                />
                            </div>
                            <div class="form-group">
                                <label>{"到期日期"}</label>
                                <input
                                    type="date"
                                    class="form-input"
                                    value={self.form_due_date.clone()}
                                    oninput={on_due_date_change}
                                />
                            </div>
                        </div>
                        <div class="form-group">
                            <label>{"付款方式"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_payment_method.clone()}
                                oninput={on_payment_method_change}
                                placeholder="如：银行转账、现金"
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
                            {if is_edit { "保存修改" } else { "创建发票" }}
                        </button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_detail_modal(&self, ctx: &Context<Self>, item: &FinanceInvoice) -> Html {
        html! {
            <div class="modal-overlay" onclick={ctx.link().callback(|_| Msg::CloseDetailModal)}>
                <div class="modal-content" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <div class="modal-header">
                        <h3>{"发票详情"}</h3>
                        <button class="close-btn" onclick={ctx.link().callback(|_| Msg::CloseDetailModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        <div class="detail-grid">
                            <div class="detail-item">
                                <span class="label">{"ID"}</span>
                                <span class="value">{item.id.to_string()}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"发票编号"}</span>
                                <span class="value">{&item.invoice_no}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"客户名称"}</span>
                                <span class="value">{&item.customer_name}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"发票类型"}</span>
                                <span class="value">{&item.invoice_type}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"金额"}</span>
                                <span class="value numeric">{&item.amount}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"税额"}</span>
                                <span class="value numeric">{&item.tax_amount}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"价税合计"}</span>
                                <span class="value numeric">{&item.total_amount}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"状态"}</span>
                                <span class="value">{&item.status}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"发票日期"}</span>
                                <span class="value">{item.invoice_date.as_deref().unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"到期日期"}</span>
                                <span class="value">{item.due_date.as_deref().unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"付款日期"}</span>
                                <span class="value">{item.paid_date.as_deref().unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"付款方式"}</span>
                                <span class="value">{item.payment_method.as_deref().unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item full-width">
                                <span class="label">{"备注"}</span>
                                <span class="value">{item.notes.as_deref().unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"创建时间"}</span>
                                <span class="value">{&item.created_at}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"更新时间"}</span>
                                <span class="value">{&item.updated_at}</span>
                            </div>
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseDetailModal)}>
                            {"关闭"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}
