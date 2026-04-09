//! 应收发票管理页面
//!
//! 应收发票（AR Invoice）管理功能

use crate::components::main_layout::MainLayout;
use crate::models::ar_invoice::{ArInvoice, ArInvoiceQueryParams};
use crate::services::ar_invoice_service::ArInvoiceService;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

/// 应收发票管理页面状态
pub struct ArInvoicePage {
    invoices: Vec<ArInvoice>,
    loading: bool,
    error: Option<String>,
    filter_status: String,
    filter_customer_id: Option<i32>,
    filter_batch_no: String,
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
    InvoicesLoaded(Vec<ArInvoice>, u64),
    LoadError(String),
    SetFilterStatus(String),
    SetFilterBatchNo(String),
    ViewInvoice(i32),
    DeleteInvoice(i32),
    ApproveInvoice(i32),
    CancelInvoice(i32, String),
    ChangePage(u64),
    Refresh,
}

impl Component for ArInvoicePage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            invoices: Vec::new(),
            loading: true,
            error: None,
            filter_status: String::from("全部"),
            filter_customer_id: None,
            filter_batch_no: String::new(),
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
                let params = ArInvoiceQueryParams {
                    customer_id: self.filter_customer_id,
                    status: if self.filter_status == "全部" {
                        None
                    } else {
                        Some(self.filter_status.clone())
                    },
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ArInvoiceService::list_invoices(params).await {
                        Ok(response) => {
                            link.send_message(Msg::InvoicesLoaded(response.data, response.total))
                        }
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
            Msg::SetFilterBatchNo(batch_no) => {
                self.filter_batch_no = batch_no;
                self.page = 1;
                ctx.link().send_message(Msg::LoadInvoices);
                false
            }
            Msg::ViewInvoice(id) => {
                web_sys::window()
                    .unwrap()
                    .location()
                    .set_href(&format!("/ar-invoices/{}", id))
                    .ok();
                false
            }
            Msg::DeleteInvoice(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ArInvoiceService::delete_invoice(id).await {
                        Ok(_) => link.send_message(Msg::LoadInvoices),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ApproveInvoice(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ArInvoiceService::approve_invoice(id).await {
                        Ok(_) => link.send_message(Msg::LoadInvoices),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::CancelInvoice(id, reason) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ArInvoiceService::cancel_invoice(id, reason).await {
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
            let target = e
                .target()
                .unwrap()
                .dyn_into::<web_sys::HtmlSelectElement>()
                .unwrap();
            Msg::SetFilterStatus(target.value())
        });

        html! {
            <MainLayout current_page={"ar_invoices"}>
<div class="ar-invoice-page">
                <div class="page-header">
                    <h1>{"应收发票管理"}</h1>
                </div>

                <div class="filter-bar">
                    <div class="filter-item">
                        <label>{"发票状态："}</label>
                        <select value={self.filter_status.clone()} onchange={on_status_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="草稿">{"草稿"}</option>
                            <option value="待审核">{"待审核"}</option>
                            <option value="已审核">{"已审核"}</option>
                            <option value="已收款">{"已收款"}</option>
                            <option value="已取消">{"已取消"}</option>
                            <option value="已核销">{"已核销"}</option>
                        </select>
                    </div>
                    <div class="filter-item">
                        <label>{"关联批次/发货单："}</label>
                        <input type="text"
                            value={self.filter_batch_no.clone()}
                            oninput={ctx.link().callback(|e: InputEvent| {
                                let target = e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
                                Msg::SetFilterBatchNo(target.value())
                            })}
                            placeholder="请输入批次/发货单号"
                        />
                    </div>
                    <button class="btn-refresh" onclick={ctx.link().callback(|_| Msg::Refresh)}>
                        {"刷新"}
                    </button>
                </div>

                {self.render_content(ctx)}
            </div>
        
</MainLayout>}
    }
}

impl ArInvoicePage {
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
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadInvoices)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        if self.invoices.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"!"}</div>
                    <p>{"暂无应收发票"}</p>
                </div>
            };
        }

        let total_pages = (self.total + self.page_size - 1) / self.page_size;
        let page = self.page;

        html! {
            <>
                <div class="table-responsive">
                    <table class="data-table w-full">
                        <thead>
                            <tr>
                                <th>{"发票编号"}</th>
                                <th>{"客户"}</th>
                                <th>{"发票日期"}</th>
                                <th>{"到期日期"}</th>
                                <th>{"发票状态"}</th>
                                <th>{"审批状态"}</th>
                                <th>{"发票金额"}</th>
                                <th>{"已收金额"}</th>
                                <th>{"未收金额"}</th>
                                <th>{"关联批次/发货单"}</th>
                                <th>{"色号"}</th>
                                <th>{"销售订单号"}</th>
                                <th>{"来源单据"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {for self.invoices.iter().map(|invoice| {
                                let status = invoice.status.clone();
                                let status_class = match status.as_str() {
                                    "草稿" => "status-draft",
                                    "待审核" => "status-pending",
                                    "已审核" => "status-approved",
                                    "已收款" => "status-paid",
                                    "已取消" => "status-cancelled",
                                    "已核销" => "status-written-off",
                                    _ => "",
                                };
                                let approval_status = invoice.approval_status.clone();
                                let approval_class = match approval_status.as_str() {
                                    "待审批" => "status-pending",
                                    "已审批" => "status-approved",
                                    "已拒绝" => "status-rejected",
                                    _ => "",
                                };
                                html! {
                                    <tr>
                                        <td>{&invoice.invoice_no}</td>
                                        <td>{invoice.customer_name.as_deref().unwrap_or("-")}</td>
                                        <td>{&invoice.invoice_date}</td>
                                        <td>{&invoice.due_date}</td>
                                        <td>
                                            <span class={format!("status-badge {}", status_class)}>{status}</span>
                                        </td>
                                        <td>
                                            <span class={format!("status-badge {}", approval_class)}>{approval_status}</span>
                                        </td>
                                        <td class="numeric-cell text-right">{&invoice.invoice_amount}</td>
                                        <td class="numeric-cell text-right">{&invoice.received_amount}</td>
                                        <td class="numeric-cell text-right">{&invoice.unpaid_amount}</td>
                                        <td>{if self.filter_batch_no.is_empty() { invoice.batch_no.as_deref().unwrap_or("-") } else { &self.filter_batch_no }}</td>
                                        <td>{invoice.color_no.as_deref().unwrap_or("-")}</td>
                                        <td>{invoice.sales_order_no.as_deref().unwrap_or("-")}</td>
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
