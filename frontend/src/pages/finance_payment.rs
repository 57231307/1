use crate::utils::permissions;
use crate::utils::toast_helper;
// 财务付款管理页面
//
// 财务付款（Finance Payment）管理功能

use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::models::finance_payment::{
    FinancePayment, PaymentQueryParams,
};
use crate::services::finance_payment_service::FinancePaymentService;
use crate::services::crud_service::CrudService;

/// 财务付款管理页面状态
pub struct FinancePaymentPage {
    printing_payment: Option<crate::models::finance_payment::FinancePayment>,
    print_trigger: bool,
    payments: Vec<FinancePayment>,
    loading: bool,
    error: Option<String>,
    filter_status: String,
    filter_type: String,
    page: u64,
    page_size: u64,
    total: u64,

    viewing_item: Option<FinancePayment>,}

/// 消息枚举
pub enum Msg {
    LoadPayments,
    PaymentsLoaded(Vec<FinancePayment>, u64),
    LoadError(String),
    SetFilterStatus(String),
    SetFilterType(String),
    ViewPayment(i32),
    DeletePayment(i32),
    ChangePage(u64),
    Refresh,

    CloseDetailModal,}

impl Component for FinancePaymentPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            viewing_item: None,
            payments: Vec::new(),
            loading: true,
            printing_payment: None,
            print_trigger: false,
            error: None,
            filter_status: String::from("全部"),
            filter_type: String::from("全部"),
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
            Msg::CloseDetailModal => {
                self.viewing_item = None;
                true
            }
            Msg::LoadPayments => {
                self.loading = true;
                let params = PaymentQueryParams {
                    status: if self.filter_status == "全部" { None } else { Some(self.filter_status.clone()) },
                    payment_type: if self.filter_type == "全部" { None } else { Some(self.filter_type.clone()) },
                    start_date: None,
                    end_date: None,
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match FinancePaymentService::list_with_query(&params).await {
                        Ok(response) => link.send_message(Msg::PaymentsLoaded(response.payments, response.total)),
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
            Msg::SetFilterType(tp) => {
                self.filter_type = tp;
                self.page = 1;
                ctx.link().send_message(Msg::LoadPayments);
                false
            }
            Msg::ViewPayment(id) => {
                self.viewing_item = self.payments.iter().find(|i| i.id == id).cloned();
                true
            }
            Msg::DeletePayment(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match FinancePaymentService::delete(id).await {
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
        let on_status_change = ctx.link().batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlSelectElement>().ok()?;
            Some(Msg::SetFilterStatus(target.value()))
        });

        let on_type_change = ctx.link().batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlSelectElement>().ok()?;
            Some(Msg::SetFilterType(target.value()))
        });

        html! {
            <div class="finance-payment-page">
                <div class="page-header">
                    <h1>{"财务付款管理"}</h1>
                </div>

                <div class="filter-bar">
                    <div class="filter-item">
                        <label>{"付款状态："}</label>
                        <select value={self.filter_status.clone()} onchange={on_status_change}>
                            <option value="全部">{"全部"}</option>
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
                            <option value="采购付款">{"采购付款"}</option>
                            <option value="费用付款">{"费用付款"}</option>
                            <option value="退款">{"退款"}</option>
                            <option value="其他付款">{"其他付款"}</option>
                        </select>
                    </div>
                    <button class="btn-refresh" onclick={ctx.link().callback(|_| Msg::Refresh)}>
                        {"刷新"}
                    </button>
                </div>

                {self.render_content(ctx)}
                {self.render_detail_modal(ctx)}
                {self.render_print_view()}
            </div>
        }
    }
}

impl FinancePaymentPage {
    
    fn render_print_view(&self) -> Html {
        if let Some(payment) = &self.printing_payment {
            html! {
                <div class="print-view" style="display: none;">
                    <div class="print-header">
                        <h2>{"秉羲面料管理 - 财务付款单"}</h2>
                    </div>
                    <div class="print-info-grid">
                        <div><strong>{"付款单号："}</strong> {&payment.payment_no}</div>
                        <div><strong>{"订单类型："}</strong> {payment.order_type.as_deref().unwrap_or("-")}</div>
                        <div><strong>{"订单 ID："}</strong> {payment.order_id.unwrap_or(0)}</div>
                        <div><strong>{"供应商 ID："}</strong> {payment.supplier_id.unwrap_or(0)}</div>
                        <div><strong>{"付款金额："}</strong> {payment.amount.to_string()}</div>
                        <div><strong>{"付款日期："}</strong> {&payment.payment_date}</div>
                        <div><strong>{"状态："}</strong> {&payment.status}</div>
                    </div>
                    <table class="print-table">
                        <thead>
                            <tr>
                                <th>{"付款方式"}</th>
                                <th>{"开户行"}</th>
                                <th>{"银行账号"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr>
                                <td>{payment.payment_method.as_deref().unwrap_or("-")}</td>
                                <td>{"-"}</td>
                                <td>{"-"}</td>
                            </tr>
                        </tbody>
                    </table>
                    <div class="print-footer">
                        <div class="print-signature">{"出纳"}</div>
                        <div class="print-signature">{"财务主管审批"}</div>
                    </div>
                </div>
            }
        } else {
            html! {}
        }
    }

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
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadPayments)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        if self.payments.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"💰"}</div>
                    <p>{"暂无财务付款记录"}</p>
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
                                <th>{"付款类型"}</th>
                                <th>{"订单类型"}</th>
                                <th>{"付款日期"}</th>
                                <th>{"付款状态"}</th>
                                <th>{"付款金额"}</th>
                                <th>{"付款方式"}</th>
                                <th>{"参考单号"}</th>
                                <th>{"创建时间"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {for self.payments.iter().map(|payment| {
                                let status = payment.status.clone();
                                let status_class = match status.as_str() {
                                    "待审核" => "status-pending",
                                    "已审核" => "status-approved",
                                    "已付款" => "status-paid",
                                    "已取消" => "status-cancelled",
                                    _ => "",
                                };
                                html! {
                                    <tr>
                                        <td>{&payment.payment_no}</td>
                                        <td>{&payment.payment_type}</td>
                                        <td>{payment.order_type.as_deref().unwrap_or("-")}</td>
                                        <td>{&payment.payment_date}</td>
                                        <td>
                                            <span class={format!("status-badge {}", status_class)}>{status}</span>
                                        </td>
                                        <td class="numeric">{payment.amount.to_string()}</td>
                                        <td>{payment.payment_method.as_deref().unwrap_or("-")}</td>
                                        <td>{payment.reference_no.as_deref().unwrap_or("-")}</td>
                                        <td>{&payment.created_at}</td>
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
    fn render_detail_modal(&self, ctx: &Context<Self>) -> Html {
        if let Some(item) = &self.viewing_item {
            html! {
                <div class="modal-overlay">
                    <div class="modal-content" style="width: 800px; max-width: 90vw;">
                        <div class="modal-header">
                            <h2>{"详情"}</h2>
                            <button class="close-btn" onclick={ctx.link().callback(|_| Msg::CloseDetailModal)}>{"×"}</button>
                        </div>
                        <div class="modal-body">
                            <div class="detail-grid" style="display: grid; grid-template-columns: 1fr 1fr; gap: 1rem;">
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Id: "}</span>
                                    <span class="detail-value">{item.id.to_string()}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Payment No: "}</span>
                                    <span class="detail-value">{&item.payment_no}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Payment Type: "}</span>
                                    <span class="detail-value">{&item.payment_type}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Order Type: "}</span>
                                    <span class="detail-value">{item.order_type.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Order Id: "}</span>
                                    <span class="detail-value">{item.order_id.map_or("-".to_string(), |v| v.to_string())}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Customer Id: "}</span>
                                    <span class="detail-value">{item.customer_id.map_or("-".to_string(), |v| v.to_string())}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Supplier Id: "}</span>
                                    <span class="detail-value">{item.supplier_id.map_or("-".to_string(), |v| v.to_string())}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Amount: "}</span>
                                    <span class="detail-value">{item.amount.to_string()}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Status: "}</span>
                                    <span class="detail-value">{&item.status}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Payment Date: "}</span>
                                    <span class="detail-value">{&item.payment_date}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Payment Method: "}</span>
                                    <span class="detail-value">{item.payment_method.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Reference No: "}</span>
                                    <span class="detail-value">{item.reference_no.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Notes: "}</span>
                                    <span class="detail-value">{item.notes.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Created At: "}</span>
                                    <span class="detail-value">{&item.created_at}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Updated At: "}</span>
                                    <span class="detail-value">{item.updated_at.as_deref().unwrap_or("-")}</span>
                                </div>
                            </div>
                        </div>
                        <div class="modal-footer">
                            <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseDetailModal)}>{"关闭"}</button>
                        </div>
                    </div>
                </div>
            }
        } else {
            html! {}
        }
    }
}
