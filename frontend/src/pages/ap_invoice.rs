use gloo_dialogs;
// 应付发票管理页面
//
// 应付发票（AP Invoice）管理功能，包含账龄分析和余额汇总

use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::models::ap_invoice::{
    ApInvoice, ApInvoiceQueryParams, AgingAnalysisItem, BalanceSummaryItem,
};
use crate::services::ap_invoice_service::ApInvoiceService;
use crate::components::main_layout::MainLayout;

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
    // 当前标签页
    active_tab: String,
    // 账龄分析数据
    aging_data: Vec<AgingAnalysisItem>,
    aging_loading: bool,
    // 余额汇总数据
    balance_data: Vec<BalanceSummaryItem>,
    balance_loading: bool,
    // 筛选供应商ID
    filter_supplier_id: Option<i32>,

    viewing_item: Option<ApInvoice>,}

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
    // 标签页切换
    SetActiveTab(String),
    // 账龄分析
    LoadAgingAnalysis,
    AgingAnalysisLoaded(Vec<AgingAnalysisItem>),
    // 余额汇总
    LoadBalanceSummary,
    BalanceSummaryLoaded(Vec<BalanceSummaryItem>),

    CloseDetailModal,}

impl Component for ApInvoicePage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            viewing_item: None,
            invoices: Vec::new(),
            loading: true,
            error: None,
            filter_status: String::from("全部"),
            filter_type: String::from("全部"),
            page: 1,
            page_size: 20,
            total: 0,
            active_tab: "list".to_string(),
            aging_data: Vec::new(),
            aging_loading: false,
            balance_data: Vec::new(),
            balance_loading: false,
            filter_supplier_id: None,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadInvoices);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::CloseDetailModal => {
                self.viewing_item = None;
                true
            }
            Msg::LoadInvoices => {
                self.loading = true;
                let params = ApInvoiceQueryParams {
                    supplier_id: self.filter_supplier_id,
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
                self.viewing_item = self.invoices.iter().find(|i| i.id == id).cloned();
                true
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
            Msg::SetActiveTab(tab) => {
                self.active_tab = tab.clone();
                // 切换到账龄分析标签页时加载数据
                if tab == "aging" && self.aging_data.is_empty() {
                    ctx.link().send_message(Msg::LoadAgingAnalysis);
                }
                // 切换到余额汇总标签页时加载数据
                if tab == "balance" && self.balance_data.is_empty() {
                    ctx.link().send_message(Msg::LoadBalanceSummary);
                }
                true
            }
            Msg::LoadAgingAnalysis => {
                self.aging_loading = true;
                let link = ctx.link().clone();
                let supplier_id = self.filter_supplier_id;
                spawn_local(async move {
                    match ApInvoiceService::get_aging_analysis(supplier_id).await {
                        Ok(data) => link.send_message(Msg::AgingAnalysisLoaded(data)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::AgingAnalysisLoaded(data) => {
                self.aging_data = data;
                self.aging_loading = false;
                true
            }
            Msg::LoadBalanceSummary => {
                self.balance_loading = true;
                let link = ctx.link().clone();
                let supplier_id = self.filter_supplier_id;
                spawn_local(async move {
                    match ApInvoiceService::get_balance_summary(supplier_id).await {
                        Ok(data) => link.send_message(Msg::BalanceSummaryLoaded(data)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::BalanceSummaryLoaded(data) => {
                self.balance_data = data;
                self.balance_loading = false;
                true
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
            <MainLayout current_page="ap_invoices">
                <div class="ap-invoice-page">
                    <div class="page-header">
                        <h1>{"应付发票管理"}</h1>
                        <p class="subtitle">{"供应商应付发票管理、账龄分析和余额汇总"}</p>
                    </div>

                    // 标签页导航
                    {self.render_tabs(ctx)}

                    // 根据标签页显示内容
                    if self.active_tab == "list" {
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
                        {self.render_list_content(ctx)}
                    } else if self.active_tab == "aging" {
                        {self.render_aging_content(ctx)}
                    } else if self.active_tab == "balance" {
                        {self.render_balance_content(ctx)}
                    }
                </div>
            </MainLayout>
        }
    }
}

impl ApInvoicePage {
    // 渲染标签页
    fn render_tabs(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="tabs">
                <button
                    class={if self.active_tab == "list" { "tab-btn active" } else { "tab-btn" }}
                    onclick={ctx.link().callback(|_| Msg::SetActiveTab("list".to_string()))}
                >
                    {"发票列表"}
                </button>
                <button
                    class={if self.active_tab == "aging" { "tab-btn active" } else { "tab-btn" }}
                    onclick={ctx.link().callback(|_| Msg::SetActiveTab("aging".to_string()))}
                >
                    {"账龄分析"}
                </button>
                <button
                    class={if self.active_tab == "balance" { "tab-btn active" } else { "tab-btn" }}
                    onclick={ctx.link().callback(|_| Msg::SetActiveTab("balance".to_string()))}
                >
                    {"余额汇总"}
                </button>
            </div>
        }
    }

    fn render_list_content(&self, ctx: &Context<Self>) -> Html {
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

    // 渲染账龄分析内容
    fn render_aging_content(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="card">
                <div class="card-header">
                    <h2>{"应付账款账龄分析"}</h2>
                    <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::LoadAgingAnalysis)}>
                        {"刷新"}
                    </button>
                </div>
                <div class="card-body">
                    if self.aging_loading {
                        <div class="loading-container">
                            <div class="spinner"></div>
                            <p>{"加载中..."}</p>
                        </div>
                    } else if self.aging_data.is_empty() {
                        <div class="empty-state">
                            <div class="empty-icon">{"📊"}</div>
                            <p>{"暂无账龄分析数据"}</p>
                        </div>
                    } else {
                        <div class="table-responsive">
                            <table class="data-table">
                                <thead>
                                    <tr>
                                        <th>{"供应商ID"}</th>
                                        <th>{"供应商名称"}</th>
                                        <th>{"当前金额"}</th>
                                        <th>{"1-30天"}</th>
                                        <th>{"31-60天"}</th>
                                        <th>{"61-90天"}</th>
                                        <th>{"90天以上"}</th>
                                        <th>{"未付总额"}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {for self.aging_data.iter().map(|item| {
                                        html! {
                                            <tr>
                                                <td>{item.supplier_id.to_string()}</td>
                                                <td>{&item.supplier_name}</td>
                                                <td class="numeric">{&item.current_amount}</td>
                                                <td class="numeric">{&item.days_1_30}</td>
                                                <td class="numeric">{&item.days_31_60}</td>
                                                <td class="numeric">{&item.days_61_90}</td>
                                                <td class="numeric highlight">{&item.days_over_90}</td>
                                                <td class="numeric total">{&item.total_outstanding}</td>
                                            </tr>
                                        }
                                    })}
                                </tbody>
                            </table>
                        </div>
                    }
                </div>
            </div>
        }
    }

    // 渲染余额汇总内容
    fn render_balance_content(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="card">
                <div class="card-header">
                    <h2>{"应付账款余额汇总"}</h2>
                    <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::LoadBalanceSummary)}>
                        {"刷新"}
                    </button>
                </div>
                <div class="card-body">
                    if self.balance_loading {
                        <div class="loading-container">
                            <div class="spinner"></div>
                            <p>{"加载中..."}</p>
                        </div>
                    } else if self.balance_data.is_empty() {
                        <div class="empty-state">
                            <div class="empty-icon">{"💰"}</div>
                            <p>{"暂无余额汇总数据"}</p>
                        </div>
                    } else {
                        <div class="table-responsive">
                            <table class="data-table">
                                <thead>
                                    <tr>
                                        <th>{"供应商ID"}</th>
                                        <th>{"供应商名称"}</th>
                                        <th>{"发票数量"}</th>
                                        <th>{"总金额"}</th>
                                        <th>{"已付金额"}</th>
                                        <th>{"未付金额"}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {for self.balance_data.iter().map(|item| {
                                        html! {
                                            <tr>
                                                <td>{item.supplier_id.to_string()}</td>
                                                <td>{&item.supplier_name}</td>
                                                <td class="numeric">{item.invoice_count.to_string()}</td>
                                                <td class="numeric">{&item.total_amount}</td>
                                                <td class="numeric">{&item.paid_amount}</td>
                                                <td class="numeric highlight">{&item.outstanding_amount}</td>
                                            </tr>
                                        }
                                    })}
                                </tbody>
                            </table>
                        </div>
                    }
                </div>
            </div>
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
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Invoice No: "}</span>
                                    <span class="detail-value">{&item.invoice_no}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Supplier Id: "}</span>
                                    <span class="detail-value">{item.supplier_id.to_string()}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Supplier Name: "}</span>
                                    <span class="detail-value">{item.supplier_name.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Invoice Date: "}</span>
                                    <span class="detail-value">{&item.invoice_date}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Due Date: "}</span>
                                    <span class="detail-value">{item.due_date.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Invoice Status: "}</span>
                                    <span class="detail-value">{&item.invoice_status}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Invoice Type: "}</span>
                                    <span class="detail-value">{&item.invoice_type}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Currency Code: "}</span>
                                    <span class="detail-value">{item.currency_code.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Exchange Rate: "}</span>
                                    <span class="detail-value">{item.exchange_rate.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Total Amount: "}</span>
                                    <span class="detail-value">{&item.total_amount}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Tax Amount: "}</span>
                                    <span class="detail-value">{item.tax_amount.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Paid Amount: "}</span>
                                    <span class="detail-value">{item.paid_amount.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Outstanding Amount: "}</span>
                                    <span class="detail-value">{item.outstanding_amount.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Source Type: "}</span>
                                    <span class="detail-value">{item.source_type.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Source Module: "}</span>
                                    <span class="detail-value">{item.source_module.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Source Bill Id: "}</span>
                                    <span class="detail-value">{item.source_bill_id.map_or("-".to_string(), |v| v.to_string())}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Source Bill No: "}</span>
                                    <span class="detail-value">{item.source_bill_no.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Receipt Id: "}</span>
                                    <span class="detail-value">{item.receipt_id.map_or("-".to_string(), |v| v.to_string())}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Receipt No: "}</span>
                                    <span class="detail-value">{item.receipt_no.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Remarks: "}</span>
                                    <span class="detail-value">{item.remarks.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Approver Id: "}</span>
                                    <span class="detail-value">{item.approver_id.map_or("-".to_string(), |v| v.to_string())}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Approver Name: "}</span>
                                    <span class="detail-value">{item.approver_name.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Approved At: "}</span>
                                    <span class="detail-value">{item.approved_at.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Cancel Reason: "}</span>
                                    <span class="detail-value">{item.cancel_reason.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Cancelled At: "}</span>
                                    <span class="detail-value">{item.cancelled_at.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Cancelled By: "}</span>
                                    <span class="detail-value">{item.cancelled_by.map_or("-".to_string(), |v| v.to_string())}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Creator Id: "}</span>
                                    <span class="detail-value">{item.creator_id.to_string()}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Creator Name: "}</span>
                                    <span class="detail-value">{item.creator_name.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Created At: "}</span>
                                    <span class="detail-value">{&item.created_at}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Updated At: "}</span>
                                    <span class="detail-value">{&item.updated_at}</span>
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
