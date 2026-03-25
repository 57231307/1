//! 应付核销管理页面
//!
//! 应付核销（AP Verification）管理功能

use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::services::ap_verification_service::{
    ApVerificationService, ApVerification, ApVerificationQueryParams,
    ManualVerifyRequest, UnverifiedInvoiceItem, UnverifiedPaymentItem,
};

/// 应付核销管理页面状态
pub struct ApVerificationPage {
    verifications: Vec<ApVerification>,
    loading: bool,
    error: Option<String>,
    filter_type: String,
    selected_supplier_id: Option<i32>,
    page: u64,
    page_size: u64,
    total: u64,
    // 模态框状态
    show_auto_verify_modal: bool,
    show_manual_verify_modal: bool,
    unverified_invoices: Vec<UnverifiedInvoiceItem>,
    unverified_payments: Vec<UnverifiedPaymentItem>,
    selected_invoice_ids: Vec<i32>,
    selected_payment_ids: Vec<i32>,
    manual_verify_amount: String,
    manual_verify_remarks: String,
}

/// 模态框模式
#[derive(Clone, PartialEq)]
pub enum ModalMode {
    View,
    AutoVerify,
    ManualVerify,
}

pub enum Msg {
    LoadVerifications,
    VerificationsLoaded(Vec<ApVerification>, u64),
    LoadError(String),
    SetFilterType(String),
    SetSupplierId(i32),
    ChangePage(u64),
    Refresh,
    OpenAutoVerifyModal,
    CloseAutoVerifyModal,
    OpenManualVerifyModal,
    CloseManualVerifyModal,
    LoadUnverifiedLists(i32),
    UnverifiedInvoicesLoaded(Vec<UnverifiedInvoiceItem>),
    UnverifiedPaymentsLoaded(Vec<UnverifiedPaymentItem>),
    ToggleInvoiceSelection(i32),
    TogglePaymentSelection(i32),
    SetManualVerifyAmount(String),
    SetManualVerifyRemarks(String),
    ExecuteAutoVerify,
    ExecuteManualVerify,
    CancelVerification(i32, String),
}

impl Component for ApVerificationPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            verifications: Vec::new(),
            loading: true,
            error: None,
            filter_type: String::from("全部"),
            selected_supplier_id: None,
            page: 1,
            page_size: 20,
            total: 0,
            show_auto_verify_modal: false,
            show_manual_verify_modal: false,
            unverified_invoices: Vec::new(),
            unverified_payments: Vec::new(),
            selected_invoice_ids: Vec::new(),
            selected_payment_ids: Vec::new(),
            manual_verify_amount: String::new(),
            manual_verify_remarks: String::new(),
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadVerifications);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadVerifications => {
                self.loading = true;
                let params = ApVerificationQueryParams {
                    supplier_id: self.selected_supplier_id,
                    verification_type: if self.filter_type == "全部" { None } else { Some(self.filter_type.clone()) },
                    start_date: None,
                    end_date: None,
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApVerificationService::list_verifications(params).await {
                        Ok(response) => link.send_message(Msg::VerificationsLoaded(response.items, response.total)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::VerificationsLoaded(verifications, total) => {
                self.verifications = verifications;
                self.total = total;
                self.loading = false;
                true
            }
            Msg::LoadError(e) => {
                self.error = Some(e);
                self.loading = false;
                true
            }
            Msg::SetFilterType(tp) => {
                self.filter_type = tp;
                self.page = 1;
                ctx.link().send_message(Msg::LoadVerifications);
                false
            }
            Msg::SetSupplierId(sid) => {
                self.selected_supplier_id = Some(sid);
                self.page = 1;
                ctx.link().send_message(Msg::LoadVerifications);
                false
            }
            Msg::ChangePage(page) => {
                self.page = page;
                ctx.link().send_message(Msg::LoadVerifications);
                false
            }
            Msg::Refresh => {
                ctx.link().send_message(Msg::LoadVerifications);
                false
            }
            Msg::OpenAutoVerifyModal => {
                self.show_auto_verify_modal = true;
                true
            }
            Msg::CloseAutoVerifyModal => {
                self.show_auto_verify_modal = false;
                true
            }
            Msg::OpenManualVerifyModal => {
                self.show_manual_verify_modal = true;
                if let Some(sid) = self.selected_supplier_id {
                    ctx.link().send_message(Msg::LoadUnverifiedLists(sid));
                }
                true
            }
            Msg::CloseManualVerifyModal => {
                self.show_manual_verify_modal = false;
                self.selected_invoice_ids.clear();
                self.selected_payment_ids.clear();
                self.manual_verify_amount.clear();
                self.manual_verify_remarks.clear();
                true
            }
            Msg::LoadUnverifiedLists(supplier_id) => {
                // 先加载发票列表
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApVerificationService::get_unverified_invoices(supplier_id).await {
                        Ok(invoices) => {
                            link.send_message(Msg::UnverifiedInvoicesLoaded(invoices));
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                // 再加载付款单列表
                let link2 = ctx.link().clone();
                spawn_local(async move {
                    match ApVerificationService::get_unverified_payments(supplier_id).await {
                        Ok(payments) => {
                            link2.send_message(Msg::UnverifiedPaymentsLoaded(payments));
                        }
                        Err(e) => link2.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::UnverifiedInvoicesLoaded(invoices) => {
                self.unverified_invoices = invoices;
                true
            }
            Msg::UnverifiedPaymentsLoaded(payments) => {
                self.unverified_payments = payments;
                true
            }
            Msg::ToggleInvoiceSelection(id) => {
                if self.selected_invoice_ids.contains(&id) {
                    self.selected_invoice_ids.retain(|x| *x != id);
                } else {
                    self.selected_invoice_ids.push(id);
                }
                true
            }
            Msg::TogglePaymentSelection(id) => {
                if self.selected_payment_ids.contains(&id) {
                    self.selected_payment_ids.retain(|x| *x != id);
                } else {
                    self.selected_payment_ids.push(id);
                }
                true
            }
            Msg::SetManualVerifyAmount(amount) => {
                self.manual_verify_amount = amount;
                false
            }
            Msg::SetManualVerifyRemarks(remarks) => {
                self.manual_verify_remarks = remarks;
                false
            }
            Msg::ExecuteAutoVerify => {
                if let Some(sid) = self.selected_supplier_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match ApVerificationService::auto_verify(sid).await {
                            Ok(_) => {
                                link.send_message(Msg::CloseAutoVerifyModal);
                                link.send_message(Msg::LoadVerifications);
                            }
                            Err(e) => link.send_message(Msg::LoadError(e)),
                        }
                    });
                }
                false
            }
            Msg::ExecuteManualVerify => {
                if let Some(sid) = self.selected_supplier_id {
                    let req = ManualVerifyRequest {
                        supplier_id: sid,
                        invoice_ids: self.selected_invoice_ids.clone(),
                        payment_ids: self.selected_payment_ids.clone(),
                        verification_amount: self.manual_verify_amount.clone(),
                        remarks: if self.manual_verify_remarks.is_empty() { None } else { Some(self.manual_verify_remarks.clone()) },
                    };
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match ApVerificationService::manual_verify(req).await {
                            Ok(_) => {
                                link.send_message(Msg::CloseManualVerifyModal);
                                link.send_message(Msg::LoadVerifications);
                            }
                            Err(e) => link.send_message(Msg::LoadError(e)),
                        }
                    });
                }
                false
            }
            Msg::CancelVerification(id, reason) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApVerificationService::cancel_verification(id, reason).await {
                        Ok(_) => link.send_message(Msg::LoadVerifications),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_type_change = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<web_sys::HtmlSelectElement>().unwrap();
            Msg::SetFilterType(target.value())
        });

        html! {
            <div class="ap-verification-page">
                <div class="page-header">
                    <h1>{"🔄 应付核销管理"}</h1>
                </div>

                <div class="filter-bar">
                    <div class="filter-item">
                        <label>{"核销类型："}</label>
                        <select value={self.filter_type.clone()} onchange={on_type_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="自动核销">{"自动核销"}</option>
                            <option value="手工核销">{"手工核销"}</option>
                        </select>
                    </div>
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::OpenAutoVerifyModal)}>
                        {"自动核销"}
                    </button>
                    <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::OpenManualVerifyModal)}>
                        {"手工核销"}
                    </button>
                    <button class="btn-refresh" onclick={ctx.link().callback(|_| Msg::Refresh)}>
                        {"刷新"}
                    </button>
                </div>

                {self.render_content(ctx)}

                {if self.show_auto_verify_modal {
                    self.render_auto_verify_modal(ctx)
                } else {
                    html! {}
                }}

                {if self.show_manual_verify_modal {
                    self.render_manual_verify_modal(ctx)
                } else {
                    html! {}
                }}
            </div>
        }
    }
}

impl ApVerificationPage {
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
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadVerifications)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        if self.verifications.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"🔄"}</div>
                    <p>{"暂无核销记录"}</p>
                </div>
            };
        }

        let total_pages = (self.total + self.page_size - 1) / self.page_size;
        let page = self.page;

        html! {
            <>
                <div class="table-responsive">
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th>{"核销单号"}</th>
                                <th>{"供应商"}</th>
                                <th>{"核销类型"}</th>
                                <th>{"核销日期"}</th>
                                <th>{"核销金额"}</th>
                                <th>{"发票数量"}</th>
                                <th>{"付款单数量"}</th>
                                <th>{"状态"}</th>
                                <th>{"核销人"}</th>
                                <th>{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {for self.verifications.iter().map(|v| {
                                let verification_id = v.id;
                                let status = v.status.clone();
                                let status_class = match status.as_str() {
                                    "已核销" => "status-verified",
                                    "已取消" => "status-cancelled",
                                    _ => "",
                                };
                                html! {
                                    <tr>
                                        <td>{&v.verification_no}</td>
                                        <td>{v.supplier_name.as_deref().unwrap_or("-")}</td>
                                        <td>{&v.verification_type}</td>
                                        <td>{&v.verification_date}</td>
                                        <td class="numeric">{&v.total_amount}</td>
                                        <td class="numeric">{v.invoice_count}</td>
                                        <td class="numeric">{v.payment_count}</td>
                                        <td>
                                            <span class={format!("status-badge {}", status_class)}>{status}</span>
                                        </td>
                                        <td>{v.verifier_name.as_deref().unwrap_or("-")}</td>
                                        <td>
                                            {if v.status == "已核销" {
                                                html! {
                                                    <button class="btn-danger btn-sm"
                                                        onclick={ctx.link().callback(move |_| Msg::CancelVerification(verification_id, "手工取消".to_string()))}>
                                                        {"取消核销"}
                                                    </button>
                                                }
                                            } else {
                                                html! { <span>{"-"}</span> }
                                            }}
                                        </td>
                                    </tr>
                                }
                            })}
                        </tbody>
                    </table>
                </div>

                <div class="pagination">
                    <span class="pagination-info">
                        {format!("共 {} 条记录，第 {}/{} 页", self.total, self.page, total_pages)}
                    </span>
                    <div class="pagination-buttons">
                        <button
                            class="btn-pagination"
                            disabled={self.page <= 1}
                            onclick={ctx.link().callback(move |_| Msg::ChangePage(page - 1))}
                        >
                            {"上一页"}
                        </button>
                        <button
                            class="btn-pagination"
                            disabled={self.page >= total_pages}
                            onclick={ctx.link().callback(move |_| Msg::ChangePage(page + 1))}
                        >
                            {"下一页"}
                        </button>
                    </div>
                </div>
            </>
        }
    }

    fn render_auto_verify_modal(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="modal-overlay">
                <div class="modal">
                    <div class="modal-header">
                        <h2>{"自动核销"}</h2>
                        <button class="modal-close" onclick={ctx.link().callback(|_| Msg::CloseAutoVerifyModal)}>
                            {"×"}
                        </button>
                    </div>
                    <div class="modal-body">
                        <p>{"确定要对选中的供应商执行自动核销吗？"}</p>
                        <p>{"系统将自动匹配应付发票和付款单进行核销。"}</p>
                    </div>
                    <div class="modal-footer">
                        <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseAutoVerifyModal)}>
                            {"取消"}
                        </button>
                        <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::ExecuteAutoVerify)}>
                            {"确认核销"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_manual_verify_modal(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="modal-overlay">
                <div class="modal modal-large">
                    <div class="modal-header">
                        <h2>{"手工核销"}</h2>
                        <button class="modal-close" onclick={ctx.link().callback(|_| Msg::CloseManualVerifyModal)}>
                            {"×"}
                        </button>
                    </div>
                    <div class="modal-body">
                        <div class="verify-section">
                            <h3>{"选择应付发票"}</h3>
                            <div class="selection-list">
                                {for self.unverified_invoices.iter().map(|inv| {
                                    let inv_id = inv.id;
                                    let checked = self.selected_invoice_ids.contains(&inv_id);
                                    html! {
                                        <label class="selection-item">
                                            <input type="checkbox"
                                                checked={checked}
                                                onchange={ctx.link().callback(move |_| Msg::ToggleInvoiceSelection(inv_id))}
                                            />
                                            <span class="item-info">
                                                <span>{"发票号: "}{&inv.invoice_no}</span>
                                                <span>{"日期: "}{&inv.invoice_date}</span>
                                                <span>{"金额: "}{&inv.total_amount}</span>
                                                <span>{"未付: "}{&inv.outstanding_amount}</span>
                                            </span>
                                        </label>
                                    }
                                })}
                            </div>
                        </div>

                        <div class="verify-section">
                            <h3>{"选择付款单"}</h3>
                            <div class="selection-list">
                                {for self.unverified_payments.iter().map(|pay| {
                                    let pay_id = pay.id;
                                    let checked = self.selected_payment_ids.contains(&pay_id);
                                    html! {
                                        <label class="selection-item">
                                            <input type="checkbox"
                                                checked={checked}
                                                onchange={ctx.link().callback(move |_| Msg::TogglePaymentSelection(pay_id))}
                                            />
                                            <span class="item-info">
                                                <span>{"付款单号: "}{&pay.payment_no}</span>
                                                <span>{"日期: "}{&pay.payment_date}</span>
                                                <span>{"金额: "}{&pay.payment_amount}</span>
                                                <span>{"未付: "}{&pay.outstanding_amount}</span>
                                            </span>
                                        </label>
                                    }
                                })}
                            </div>
                        </div>

                        <div class="verify-form">
                            <div class="form-item">
                                <label>{"核销金额："}</label>
                                <input type="text"
                                    value={self.manual_verify_amount.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
                                        Msg::SetManualVerifyAmount(target.value())
                                    })}
                                    placeholder="请输入核销金额"
                                />
                            </div>
                            <div class="form-item">
                                <label>{"备注："}</label>
                                <textarea
                                    value={self.manual_verify_remarks.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target().unwrap().dyn_into::<web_sys::HtmlTextAreaElement>().unwrap();
                                        Msg::SetManualVerifyRemarks(target.value())
                                    })}
                                    placeholder="请输入备注信息"
                                />
                            </div>
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseManualVerifyModal)}>
                            {"取消"}
                        </button>
                        <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::ExecuteManualVerify)}>
                            {"确认核销"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}