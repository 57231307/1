// 财务付款管理页面

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
use crate::models::finance_payment::{
    FinancePayment, PaymentQueryParams, CreatePaymentRequest, UpdatePaymentRequest,
};
use crate::services::finance_payment_service::FinancePaymentService;
use crate::services::crud_service::CrudService;

/// 财务付款管理页面状态
pub struct FinancePaymentPage {
    payments: Vec<FinancePayment>,
    filtered_payments: Vec<FinancePayment>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_payment: Option<FinancePayment>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    viewing_item: Option<FinancePayment>,
    filter_status: String,
    filter_type: String,
    // 表单字段
    form_payment_no: String,
    form_payment_type: String,
    form_amount: String,
    form_status: String,
    form_payment_date: String,
    form_payment_method: String,
    form_reference_no: String,
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
    LoadPayments,
    PaymentsLoaded(Vec<FinancePayment>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    SetFilterStatus(String),
    SetFilterType(String),
    OpenCreateModal,
    OpenEditModal(FinancePayment),
    CloseModal,
    CloseDetailModal,
    ViewPayment(i32),
    SubmitForm,
    FormSubmitted,
    DeletePayment(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    Refresh,
    // 表单字段变更
    FormPaymentNoChanged(String),
    FormPaymentTypeChanged(String),
    FormAmountChanged(String),
    FormStatusChanged(String),
    FormPaymentDateChanged(String),
    FormPaymentMethodChanged(String),
    FormReferenceNoChanged(String),
    FormNotesChanged(String),
}

impl Component for FinancePaymentPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            payments: Vec::new(),
            filtered_payments: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            show_modal: false,
            modal_mode: ModalMode::Create,
            editing_payment: None,
            show_delete_confirm: false,
            deleting_id: None,
            viewing_item: None,
            filter_status: String::from("全部"),
            filter_type: String::from("全部"),
            form_payment_no: String::new(),
            form_payment_type: String::new(),
            form_amount: String::new(),
            form_status: String::from("草稿"),
            form_payment_date: String::new(),
            form_payment_method: String::new(),
            form_reference_no: String::new(),
            form_notes: String::new(),
            form_error: None,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadPayments);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadPayments => {
                self.loading = true;
                self.error = None;
                let params = PaymentQueryParams {
                    status: if self.filter_status == "全部" { None } else { Some(self.filter_status.clone()) },
                    payment_type: if self.filter_type == "全部" { None } else { Some(self.filter_type.clone()) },
                    start_date: None,
                    end_date: None,
                    page: Some(1),
                    page_size: Some(1000),
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match FinancePaymentService::list_payments(params).await {
                        Ok(response) => link.send_message(Msg::PaymentsLoaded(response.payments)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::PaymentsLoaded(payments) => {
                self.payments = payments;
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
                ctx.link().send_message(Msg::LoadPayments);
                false
            }
            Msg::SetFilterType(tp) => {
                self.filter_type = tp;
                self.page = 0;
                ctx.link().send_message(Msg::LoadPayments);
                false
            }
            Msg::OpenCreateModal => {
                self.reset_form();
                self.editing_payment = None;
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(payment) => {
                self.form_payment_no = payment.payment_no.clone();
                self.form_payment_type = payment.payment_type.clone();
                self.form_amount = payment.amount.to_string();
                self.form_status = payment.status.clone();
                self.form_payment_date = payment.payment_date.clone();
                self.form_payment_method = payment.payment_method.clone().unwrap_or_default();
                self.form_reference_no = payment.reference_no.clone().unwrap_or_default();
                self.form_notes = payment.notes.clone().unwrap_or_default();
                self.form_error = None;
                self.editing_payment = Some(payment);
                self.modal_mode = ModalMode::Edit;
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.editing_payment = None;
                self.form_error = None;
                true
            }
            Msg::CloseDetailModal => {
                self.viewing_item = None;
                true
            }
            Msg::ViewPayment(id) => {
                self.viewing_item = self.payments.iter().find(|p| p.id == id).cloned();
                true
            }
            Msg::SubmitForm => {
                if self.form_payment_no.is_empty() {
                    self.form_error = Some("付款编号不能为空".to_string());
                    return true;
                }
                if self.form_payment_type.is_empty() {
                    self.form_error = Some("付款类型不能为空".to_string());
                    return true;
                }
                if self.form_amount.is_empty() {
                    self.form_error = Some("付款金额不能为空".to_string());
                    return true;
                }

                self.form_error = None;

                let amount = match self.form_amount.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        self.form_error = Some("付款金额格式不正确".to_string());
                        return true;
                    }
                };

                if self.modal_mode == ModalMode::Edit {
                    if let Some(payment) = &self.editing_payment {
                        let id = payment.id;
                        let req = UpdatePaymentRequest {
                            payment_no: Some(self.form_payment_no.clone()),
                            payment_type: Some(self.form_payment_type.clone()),
                            order_type: None,
                            order_id: None,
                            customer_id: None,
                            supplier_id: None,
                            amount: Some(self.form_amount.clone()),
                            status: Some(self.form_status.clone()),
                            payment_date: if self.form_payment_date.is_empty() { None } else { Some(self.form_payment_date.clone()) },
                            payment_method: if self.form_payment_method.is_empty() { None } else { Some(self.form_payment_method.clone()) },
                            reference_no: if self.form_reference_no.is_empty() { None } else { Some(self.form_reference_no.clone()) },
                            notes: if self.form_notes.is_empty() { None } else { Some(self.form_notes.clone()) },
                        };
                        let link = ctx.link().clone();
                        spawn_local(async move {
                            match FinancePaymentService::update_payment(id, req).await {
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
                    let req = CreatePaymentRequest {
                        payment_no: self.form_payment_no.clone(),
                        payment_type: self.form_payment_type.clone(),
                        order_type: String::new(),
                        order_id: None,
                        customer_id: None,
                        supplier_id: None,
                        amount,
                        payment_date: if self.form_payment_date.is_empty() { chrono::Local::now().format("%Y-%m-%d").to_string() } else { self.form_payment_date.clone() },
                        payment_method: if self.form_payment_method.is_empty() { None } else { Some(self.form_payment_method.clone()) },
                        reference_no: if self.form_reference_no.is_empty() { None } else { Some(self.form_reference_no.clone()) },
                        notes: if self.form_notes.is_empty() { None } else { Some(self.form_notes.clone()) },
                    };
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match FinancePaymentService::create_payment(req).await {
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
                self.editing_payment = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadPayments);
                false
            }
            Msg::DeletePayment(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match FinancePaymentService::delete_payment(id).await {
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
                ctx.link().send_message(Msg::LoadPayments);
                false
            }
            Msg::Refresh => {
                ctx.link().send_message(Msg::LoadPayments);
                false
            }
            Msg::FormPaymentNoChanged(v) => { self.form_payment_no = v; true }
            Msg::FormPaymentTypeChanged(v) => { self.form_payment_type = v; true }
            Msg::FormAmountChanged(v) => { self.form_amount = v; true }
            Msg::FormStatusChanged(v) => { self.form_status = v; true }
            Msg::FormPaymentDateChanged(v) => { self.form_payment_date = v; true }
            Msg::FormPaymentMethodChanged(v) => { self.form_payment_method = v; true }
            Msg::FormReferenceNoChanged(v) => { self.form_reference_no = v; true }
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

        html! {
            <div class="finance-payment-page">
                <PageHeader title={"财务付款管理".to_string()} subtitle={Some("管理财务付款信息".to_string())}>
                    <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                        {"+ 新建付款"}
                    </button>
                </PageHeader>

                <div class="page-toolbar">
                    <div class="filter-bar">
                        <div class="filter-item">
                            <label>{"付款状态："}</label>
                            <select value={self.filter_status.clone()} onchange={on_status_change}>
                                <option value="全部">{"全部"}</option>
                                <option value="草稿">{"草稿"}</option>
                                <option value="待审核">{"待审核"}</option>
                                <option value="已审核">{"已审核"}</option>
                                <option value="已付款">{"已付款"}</option>
                                <option value="已取消">{"已取消"}</option>
                            </select>
                        </div>
                        <div class="filter-item">
                            <label>{"付款类型："}</label>
                            <select value={self.filter_type.clone()} onchange={on_type_change}>
                                <option value="全部">{"全部"}</option>
                                <option value="货款">{"货款"}</option>
                                <option value="费用">{"费用"}</option>
                                <option value="工资">{"工资"}</option>
                                <option value="税费">{"税费"}</option>
                                <option value="其他">{"其他"}</option>
                            </select>
                        </div>
                    </div>
                    <SearchBar
                        placeholder={"搜索付款号、类型...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载付款数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadPayments)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_payments.is_empty() {
                    <EmptyState
                        icon={"💳".to_string()}
                        title={"暂无财务付款".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一笔付款".to_string()
                        } else {
                            "没有匹配搜索条件的付款".to_string()
                        }}
                    />
                } else {
                    {self.render_table(ctx)}
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
                    message={"确定要删除这笔付款吗？此操作不可撤销。".to_string()}
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

impl FinancePaymentPage {
    fn apply_filter(&mut self) {
        let mut filtered = self.payments.clone();

        if !self.search_keyword.is_empty() {
            let keyword = self.search_keyword.to_lowercase();
            filtered = filtered.into_iter()
                .filter(|p| {
                    p.payment_no.to_lowercase().contains(&keyword) ||
                    p.payment_type.to_lowercase().contains(&keyword)
                })
                .collect();
        }

        self.filtered_payments = filtered;
    }

    fn paginated_payments(&self) -> Vec<FinancePayment> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_payments[start..end.min(self.filtered_payments.len())].to_vec()
    }

    fn render_table(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let paginated = self.paginated_payments();
        let total_count = self.filtered_payments.len() as u64;

        html! {
            <div class="table-container">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>{"付款编号"}</th>
                            <th>{"付款类型"}</th>
                            <th>{"付款日期"}</th>
                            <th>{"状态"}</th>
                            <th class="numeric">{"付款金额"}</th>
                            <th>{"付款方式"}</th>
                            <th class="text-center">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for paginated.iter().map(|payment| {
                            let payment_clone = payment.clone();
                            let id = payment.id;
                            let status = payment.status.clone();
                            let status_class = match status.as_str() {
                                "草稿" => "status-badge status-draft",
                                "待审核" => "status-badge status-pending",
                                "已审核" => "status-badge status-approved",
                                "已付款" => "status-badge status-paid",
                                "已取消" => "status-badge status-cancelled",
                                _ => "status-badge",
                            };
                            html! {
                                <tr>
                                    <td>{&payment.payment_no}</td>
                                    <td>{&payment.payment_type}</td>
                                    <td>{&payment.payment_date}</td>
                                    <td>
                                        <span class={status_class}>{&status}</span>
                                    </td>
                                    <td class="numeric">{payment.amount.to_string()}</td>
                                    <td>{payment.payment_method.as_ref().map(|s| s.as_str()).unwrap_or("-")}</td>
                                    <td class="text-center">
                                        <div class="action-buttons">
                                            <button
                                                class="btn btn-sm btn-info"
                                                onclick={link.callback(move |_| Msg::ViewPayment(id))}
                                            >
                                                {"查看"}
                                            </button>
                                            <button
                                                class="btn btn-sm btn-secondary"
                                                onclick={link.callback(move |_| Msg::OpenEditModal(payment_clone.clone()))}
                                            >
                                                {"编辑"}
                                            </button>
                                            <button
                                                class="btn btn-sm btn-danger"
                                                onclick={link.callback(move |_| Msg::DeletePayment(id))}
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
    }

    fn reset_form(&mut self) {
        self.form_payment_no = String::new();
        self.form_payment_type = String::new();
        self.form_amount = String::new();
        self.form_status = "草稿".to_string();
        self.form_payment_date = String::new();
        self.form_payment_method = String::new();
        self.form_reference_no = String::new();
        self.form_notes = String::new();
        self.form_error = None;
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let is_edit = self.modal_mode == ModalMode::Edit;
        let title = if is_edit { "编辑财务付款" } else { "新建财务付款" };

        let on_payment_no_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormPaymentNoChanged(input.value()))
        });
        let on_payment_type_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormPaymentTypeChanged(input.value()))
        });
        let on_amount_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormAmountChanged(input.value()))
        });
        let on_status_change = link.batch_callback(|e: Event| {
            e.target().and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok()).map(|select| Msg::FormStatusChanged(select.value()))
        });
        let on_payment_date_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormPaymentDateChanged(input.value()))
        });
        let on_payment_method_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormPaymentMethodChanged(input.value()))
        });
        let on_reference_no_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormReferenceNoChanged(input.value()))
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
                            <label>{"付款编号 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_payment_no.clone()}
                                oninput={on_payment_no_change}
                                placeholder="请输入付款编号"
                                disabled={is_edit}
                            />
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"付款类型 *"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_payment_type.clone()}
                                    oninput={on_payment_type_change}
                                    placeholder="如：货款、费用、工资"
                                />
                            </div>
                            <div class="form-group">
                                <label>{"状态"}</label>
                                <select class="form-input" value={self.form_status.clone()} onchange={on_status_change}>
                                    <option value="草稿">{"草稿"}</option>
                                    <option value="待审核">{"待审核"}</option>
                                    <option value="已审核">{"已审核"}</option>
                                    <option value="已付款">{"已付款"}</option>
                                    <option value="已取消">{"已取消"}</option>
                                </select>
                            </div>
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"付款金额 *"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_amount.clone()}
                                    oninput={on_amount_change}
                                    placeholder="请输入付款金额"
                                />
                            </div>
                            <div class="form-group">
                                <label>{"付款日期"}</label>
                                <input
                                    type="date"
                                    class="form-input"
                                    value={self.form_payment_date.clone()}
                                    oninput={on_payment_date_change}
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
                            <label>{"参考编号"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_reference_no.clone()}
                                oninput={on_reference_no_change}
                                placeholder="请输入参考编号"
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
                            {if is_edit { "保存修改" } else { "创建付款" }}
                        </button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_detail_modal(&self, ctx: &Context<Self>, item: &FinancePayment) -> Html {
        html! {
            <div class="modal-overlay" onclick={ctx.link().callback(|_| Msg::CloseDetailModal)}>
                <div class="modal-content" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <div class="modal-header">
                        <h3>{"付款详情"}</h3>
                        <button class="close-btn" onclick={ctx.link().callback(|_| Msg::CloseDetailModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        <div class="detail-grid">
                            <div class="detail-item">
                                <span class="label">{"ID"}</span>
                                <span class="value">{item.id.to_string()}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"付款编号"}</span>
                                <span class="value">{&item.payment_no}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"付款类型"}</span>
                                <span class="value">{&item.payment_type}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"付款金额"}</span>
                                <span class="value numeric">{item.amount.to_string()}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"状态"}</span>
                                <span class="value">{&item.status}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"付款日期"}</span>
                                <span class="value">{&item.payment_date}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"付款方式"}</span>
                                <span class="value">{item.payment_method.as_ref().map(|s| s.as_str()).unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"参考编号"}</span>
                                <span class="value">{item.reference_no.as_ref().map(|s| s.as_str()).unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item full-width">
                                <span class="label">{"备注"}</span>
                                <span class="value">{item.notes.as_ref().map(|s| s.as_str()).unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"创建时间"}</span>
                                <span class="value">{&item.created_at}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"更新时间"}</span>
                                <span class="value">{item.updated_at.as_ref().map(|s| s.as_str()).unwrap_or("-")}</span>
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
