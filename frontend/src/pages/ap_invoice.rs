//! 应付发票管理页面
//!
//! 应付发票（AP Invoice）管理功能

use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::services::ap_invoice_service::{
    ApInvoiceService, ApInvoice, ApInvoiceQueryParams,
};

/// 应付发票管理页面状态
pub struct ApInvoicePage {
    invoices: Vec<ApInvoice>,
    loading: bool,
    error: Option<String>,
    filter_status: String,
    filter_type: String,
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
    LoadInvoices,
    InvoicesLoaded(Vec<ApInvoice>, u64),
    LoadError(String),
    SetFilterStatus(String),
    SetFilterType(String),
    ViewInvoice(i32),
    DeleteInvoice(i32),
    ApproveInvoice(i32),
    CancelInvoice(i32, String),
    ChangePage(u64),
    Refresh,
}

impl Component for ApInvoicePage {
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
                let params = ApInvoiceQueryParams {
                    supplier_id: None,
                    invoice_status: if self.filter_status == "全部" { None } else { Some(self.filter_status.clone()) },
                    invoice_type: if self.filter_type == "全部" { None } else { Some(self.filter_type.clone()) },
                    start_date: None,
                    end_date: None,
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApInvoiceService::list_invoices(params).await {
                        Ok(response) => link.send_message(Msg::InvoicesLoaded(response.data, response.total)),
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
                web_sys::window().unwrap().location().set_href(&format!("/ap-invoices/{}", id)).ok();
                false
            }
            Msg::DeleteInvoice(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApInvoiceService::delete_invoice(id).await {
                        Ok(_) => link.send_message(Msg::LoadInvoices),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ApproveInvoice(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApInvoiceService::approve_invoice(id).await {
                        Ok(_) => link.send_message(Msg::LoadInvoices),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::CancelInvoice(id, reason) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApInvoiceService::cancel_invoice(id, reason).await {
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
            <div class="ap-invoice-page">
                <div class="page-header">
                    <h1>{"📋 应付发票管理"}</h1>
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
                            <option value="已取消">{"已取消"}</option>
                        </select>
                    </div>
                    <div class="filter-item">
                        <label>{"发票类型："}</label>
                        <select value={self.filter_type.clone()} onchange={on_type_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="采购发票">{"采购发票"}</option>
                            <option value="费用发票">{"费用发票"}</option>
                            <option value="调整发票">{"调整发票"}</option>
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

impl ApInvoicePage {
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
                    <p>{"暂无应付发票"}</p>
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
                                <th>{"供应商"}</th>
                                <th>{"发票日期"}</th>
                                <th>{"到期日期"}</th>
                                <th>{"发票状态"}</th>
                                <th>{"发票类型"}</th>
                                <th>{"总金额"}</th>
                                <th>{"已付金额"}</th>
                                <th>{"未付金额"}</th>
                                <th>{"来源单据"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {for self.invoices.iter().map(|invoice| {
                                let status = invoice.invoice_status.clone();
                                let status_class = match status.as_str() {
                                    "草稿" => "status-draft",
                                    "待审核" => "status-pending",
                                    "已审核" => "status-approved",
                                    "已付款" => "status-paid",
                                    "已取消" => "status-cancelled",
                                    _ => "",
                                };
                                html! {
                                    <tr>
                                        <td>{&invoice.invoice_no}</td>
                                        <td>{invoice.supplier_name.as_deref().unwrap_or("-")}</td>
                                        <td>{&invoice.invoice_date}</td>
                                        <td>{invoice.due_date.as_deref().unwrap_or("-")}</td>
                                        <td>
                                            <span class={format!("status-badge {}", status_class)}>{status}</span>
                                        </td>
                                        <td>{&invoice.invoice_type}</td>
                                        <td class="numeric">{&invoice.total_amount}</td>
                                        <td class="numeric">{invoice.paid_amount.as_deref().unwrap_or("0.00")}</td>
                                        <td class="numeric">{invoice.outstanding_amount.as_deref().unwrap_or("0.00")}</td>
                                        <td>{invoice.source_bill_no.as_deref().unwrap_or("-")}</td>
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