//! 付款管理页面
//!
//! 付款（AP Payment）管理功能

use crate::models::ap_payment::{ApPayment, ApPaymentQueryParams};
use crate::services::ap_payment_service::ApPaymentService;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

/// 付款管理页面状态
pub struct ApPaymentPage {
    payments: Vec<ApPayment>,
    loading: bool,
    error: Option<String>,
    filter_status: String,
    filter_method: String,
    page: u64,
    page_size: u64,
    total: u64,
}

/// 模态框模式
#[derive(Clone, PartialEq)]
pub enum ModalMode {
    View,
    Create,
    Edit,
}

pub enum Msg {
    LoadPayments,
    PaymentsLoaded(Vec<ApPayment>, u64),
    LoadError(String),
    SetFilterStatus(String),
    SetFilterMethod(String),
    ViewPayment(i32),
    DeletePayment(i32),
    ConfirmPayment(i32),
    ChangePage(u64),
    Refresh,
}

impl Component for ApPaymentPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            payments: Vec::new(),
            loading: true,
            error: None,
            filter_status: String::from("全部"),
            filter_method: String::from("全部"),
            page: 1,
            page_size: 20,
            total: 0,
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
                let params = ApPaymentQueryParams {
                    supplier_id: None,
                    payment_status: if self.filter_status == "全部" {
                        None
                    } else {
                        Some(self.filter_status.clone())
                    },
                    payment_method: if self.filter_method == "全部" {
                        None
                    } else {
                        Some(self.filter_method.clone())
                    },
                    start_date: None,
                    end_date: None,
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApPaymentService::list_payments(params).await {
                        Ok(response) => {
                            link.send_message(Msg::PaymentsLoaded(response.items, response.total))
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::PaymentsLoaded(payments, total) => {
                self.payments = payments;
                self.total = total;
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
                ctx.link().send_message(Msg::LoadPayments);
                false
            }
            Msg::SetFilterMethod(method) => {
                self.filter_method = method;
                self.page = 1;
                ctx.link().send_message(Msg::LoadPayments);
                false
            }
            Msg::ViewPayment(id) => {
                web_sys::window()
                    .unwrap()
                    .location()
                    .set_href(&format!("/ap-payments/{}", id))
                    .ok();
                false
            }
            Msg::DeletePayment(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApPaymentService::delete_payment(id).await {
                        Ok(_) => link.send_message(Msg::LoadPayments),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ConfirmPayment(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApPaymentService::confirm_payment(id).await {
                        Ok(_) => link.send_message(Msg::LoadPayments),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ChangePage(page) => {
                self.page = page;
                ctx.link().send_message(Msg::LoadPayments);
                false
            }
            Msg::Refresh => {
                ctx.link().send_message(Msg::LoadPayments);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_status_change = ctx.link().callback(|e: Event| {
            let target = e
                .target()
                .unwrap()
                .dyn_into::<web_sys::HtmlSelectElement>()
                .unwrap();
            Msg::SetFilterStatus(target.value())
        });

        let on_method_change = ctx.link().callback(|e: Event| {
            let target = e
                .target()
                .unwrap()
                .dyn_into::<web_sys::HtmlSelectElement>()
                .unwrap();
            Msg::SetFilterMethod(target.value())
        });

        html! {
            <div class="ap-payment-page">
                <div class="page-header">
                    <h1>{"付款管理"}</h1>
                </div>

                <div class="filter-bar">
                    <div class="filter-item">
                        <label>{"付款状态："}</label>
                        <select value={self.filter_status.clone()} onchange={on_status_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="PENDING">{"待确认"}</option>
                            <option value="CONFIRMED">{"已确认"}</option>
                            <option value="CANCELLED">{"已取消"}</option>
                        </select>
                    </div>
                    <div class="filter-item">
                        <label>{"付款方式："}</label>
                        <select value={self.filter_method.clone()} onchange={on_method_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="TT">{"电汇"}</option>
                            <option value="LC">{"信用证"}</option>
                            <option value="DP">{"付款交单"}</option>
                            <option value="DA">{"承兑交单"}</option>
                            <option value="CHECK">{"支票"}</option>
                            <option value="CASH">{"现金"}</option>
                        </select>
                    </div>
                    <button class="btn-refresh" onclick={ctx.link().callback(|_| Msg::Refresh)}>
                        {"刷新"}
                    </button>
                </div>

                {self.render_content(ctx)}
            </div>
        }
    }
}

impl ApPaymentPage {
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
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadPayments)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        if self.payments.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"!"}</div>
                    <p>{"暂无付款记录"}</p>
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
                                <th>{"付款单号"}</th>
                                <th>{"供应商"}</th>
                                <th>{"付款日期"}</th>
                                <th>{"付款类型"}</th>
                                <th>{"付款方式"}</th>
                                <th>{"付款金额"}</th>
                                <th>{"付款状态"}</th>
                                <th>{"收款银行"}</th>
                                <th>{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {for self.payments.iter().map(|payment| {
                                let payment_id = payment.id;
                                let status = payment.payment_status.clone();
                                let status_text = match status.as_str() {
                                    "PENDING" => "待确认",
                                    "CONFIRMED" => "已确认",
                                    "CANCELLED" => "已取消",
                                    _ => &status,
                                };
                                let status_class = match status.as_str() {
                                    "PENDING" => "status-draft",
                                    "CONFIRMED" => "status-approved",
                                    "CANCELLED" => "status-rejected",
                                    _ => "",
                                };
                                let payment_type_text = match payment.payment_type.as_str() {
                                    "PREPAYMENT" => "预付款",
                                    "PROGRESS" => "进度款",
                                    "FINAL" => "尾款",
                                    "WARRANTY" => "质保金",
                                    _ => &payment.payment_type,
                                };
                                let payment_method_text = match payment.payment_method.as_str() {
                                    "TT" => "电汇",
                                    "LC" => "信用证",
                                    "DP" => "付款交单",
                                    "DA" => "承兑交单",
                                    "CHECK" => "支票",
                                    "CASH" => "现金",
                                    _ => &payment.payment_method,
                                };
                                html! {
                                    <tr>
                                        <td>{&payment.payment_no}</td>
                                        <td>{payment.supplier_name.as_deref().unwrap_or("-")}</td>
                                        <td>{&payment.payment_date}</td>
                                        <td>{payment_type_text}</td>
                                        <td>{payment_method_text}</td>
                                        <td class="numeric">{&payment.payment_amount.to_string()}</td>
                                        <td>
                                            <span class={format!("status-badge {}", status_class)}>{status_text}</span>
                                        </td>
                                        <td>{payment.bank_name.as_deref().unwrap_or("-")}</td>
                                        <td>
                                            <div class="action-buttons">
                                                <button class="btn-action" onclick={ctx.link().callback(move |_| Msg::ViewPayment(payment_id))}>
                                                    {"查看"}
                                                </button>
                                                if payment.payment_status == "PENDING" {
                                                    <button class="btn-action btn-success" onclick={ctx.link().callback(move |_| Msg::ConfirmPayment(payment_id))}>
                                                        {"确认"}
                                                    </button>
                                                    <button class="btn-action btn-danger" onclick={ctx.link().callback(move |_| Msg::DeletePayment(payment_id))}>
                                                        {"删除"}
                                                    </button>
                                                }
                                            </div>
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
                            disabled={page <= 1}
                            onclick={ctx.link().callback(move |_| Msg::ChangePage(page - 1))}
                        >
                            {"上一页"}
                        </button>
                        <button
                            class="btn-pagination"
                            disabled={page >= total_pages}
                            onclick={ctx.link().callback(move |_| Msg::ChangePage(page + 1))}
                        >
                            {"下一页"}
                        </button>
                    </div>
                </div>
            </>
        }
    }
}
