//! 财务发票管理页面
//!
//! 财务发票（Finance Invoice）管理功能

use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::services::finance_invoice_service::{
    FinanceInvoiceService, FinanceInvoice, InvoiceQueryParams,
};

/// 财务发票管理页面状态
pub struct FinanceInvoicePage {
    invoices: Vec<FinanceInvoice>,
    loading: bool,
    error: Option<String>,
    filter_status: String,
    filter_type: String,
    page: u64,
    page_size: u64,
    total: u64,
}

/// 消息枚举
pub enum Msg {
    LoadInvoices,
    InvoicesLoaded(Vec<FinanceInvoice>, u64),
    LoadError(String),
    SetFilterStatus(String),
    SetFilterType(String),
    ViewInvoice(i32),
    DeleteInvoice(i32),
    ApproveInvoice(i32),
    VerifyInvoice(i32, String),
    ChangePage(u64),
    Refresh,
}

impl Component for FinanceInvoicePage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            invoices: Vec::new(),
            loading: true,
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
            ctx.link().send_message(Msg::LoadInvoices);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadInvoices => {
                self.loading = true;
                let params = InvoiceQueryParams {
                    customer_id: None,
                    status: if self.filter_status == "全部" { None } else { Some(self.filter_status.clone()) },
                    invoice_type: if self.filter_type == "全部" { None } else { Some(self.filter_type.clone()) },
                    start_date: None,
                    end_date: None,
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match FinanceInvoiceService::list_invoices(params).await {
                        Ok(response) => link.send_message(Msg::InvoicesLoaded(response.invoices, response.total)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::InvoicesLoaded(invoices, total) => {
                self.invoices = invoices;
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
                ctx.link().send_message(Msg::LoadInvoices);
                false
            }
            Msg::SetFilterType(tp) => {
                self.filter_type = tp;
                self.page = 1;
                ctx.link().send_message(Msg::LoadInvoices);
                false
            }
            Msg::ViewInvoice(id) => {
                web_sys::window().unwrap().location().set_href(&format!("/finance-invoices/{}", id)).ok();
                false
            }
            Msg::DeleteInvoice(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match FinanceInvoiceService::delete_invoice(id).await {
                        Ok(_) => link.send_message(Msg::LoadInvoices),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ApproveInvoice(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match FinanceInvoiceService::approve_invoice(id).await {
                        Ok(_) => link.send_message(Msg::LoadInvoices),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::VerifyInvoice(id, payment_method) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match FinanceInvoiceService::verify_invoice(id, payment_method).await {
                        Ok(_) => link.send_message(Msg::LoadInvoices),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ChangePage(page) => {
                self.page = page;
                ctx.link().send_message(Msg::LoadInvoices);
                false
            }
            Msg::Refresh => {
                ctx.link().send_message(Msg::LoadInvoices);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_status_change = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<web_sys::HtmlSelectElement>().unwrap();
            Msg::SetFilterStatus(target.value())
        });

        let on_type_change = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<web_sys::HtmlSelectElement>().unwrap();
            Msg::SetFilterType(target.value())
        });

        html! {
            <div class="finance-invoice-page">
                <div class="page-header">
                    <h1>{"财务发票管理"}</h1>
                </div>

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
                    <button class="btn-refresh" onclick={ctx.link().callback(|_| Msg::Refresh)}>
                        {"刷新"}
                    </button>
                </div>

                {self.render_content(ctx)}
            </div>
        }
    }
}

impl FinanceInvoicePage {
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
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadInvoices)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        if self.invoices.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"📋"}</div>
                    <p>{"暂无财务发票"}</p>
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
                                <th>{"发票编号"}</th>
                                <th>{"客户名称"}</th>
                                <th>{"发票类型"}</th>
                                <th>{"发票日期"}</th>
                                <th>{"到期日期"}</th>
                                <th>{"发票状态"}</th>
                                <th>{"金额"}</th>
                                <th>{"税额"}</th>
                                <th>{"价税合计"}</th>
                                <th>{"付款方式"}</th>
                                <th>{"付款日期"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {for self.invoices.iter().map(|invoice| {
                                let status = invoice.status.clone();
                                let status_class = match status.as_str() {
                                    "草稿" => "status-draft",
                                    "待审核" => "status-pending",
                                    "已审核" => "status-approved",
                                    "已付款" => "status-paid",
                                    "已核销" => "status-verified",
                                    "已作废" => "status-cancelled",
                                    _ => "",
                                };
                                html! {
                                    <tr>
                                        <td>{&invoice.invoice_no}</td>
                                        <td>{&invoice.customer_name}</td>
                                        <td>{&invoice.invoice_type}</td>
                                        <td>{invoice.invoice_date.as_deref().unwrap_or("-")}</td>
                                        <td>{invoice.due_date.as_deref().unwrap_or("-")}</td>
                                        <td>
                                            <span class={format!("status-badge {}", status_class)}>{status}</span>
                                        </td>
                                        <td class="numeric">{&invoice.amount}</td>
                                        <td class="numeric">{&invoice.tax_amount}</td>
                                        <td class="numeric">{&invoice.total_amount}</td>
                                        <td>{invoice.payment_method.as_deref().unwrap_or("-")}</td>
                                        <td>{invoice.paid_date.as_deref().unwrap_or("-")}</td>
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
}