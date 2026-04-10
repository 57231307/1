use gloo_dialogs;
// 应付对账管理页面
//
// 应付对账（AP Reconciliation）管理功能

use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::models::ap_reconciliation::{
    ApReconciliation, ApReconciliationQueryParams,
};
use crate::services::ap_reconciliation_service::ApReconciliationService;

/// 应付对账管理页面状态
pub struct ApReconciliationPage {
    reconciliations: Vec<ApReconciliation>,
    loading: bool,
    error: Option<String>,
    filter_status: String,
    page: u64,
    page_size: u64,
    total: u64,
    show_generate_modal: bool,
    show_dispute_modal: bool,
    selected_id: Option<i32>,
    dispute_reason: String,
}

/// 模态框模式
#[derive(Clone, PartialEq)]
pub enum ModalMode {
    View,
    Create,
    Edit,
}

pub enum Msg {
    LoadReconciliations,
    ReconciliationsLoaded(Vec<ApReconciliation>, u64),
    LoadError(String),
    SetFilterStatus(String),
    ViewReconciliation(i32),
    GenerateReconciliation,
    ConfirmReconciliation(i32),
    DisputeReconciliation(i32),
    SubmitDispute,
    ChangePage(u64),
    Refresh,
    CloseGenerateModal,
    CloseDisputeModal,
    SetDisputeReason(String),
}

impl Component for ApReconciliationPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            reconciliations: Vec::new(),
            loading: true,
            error: None,
            filter_status: String::from("全部"),
            page: 1,
            page_size: 20,
            total: 0,
            show_generate_modal: false,
            show_dispute_modal: false,
            selected_id: None,
            dispute_reason: String::new(),
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadReconciliations);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadReconciliations => {
                self.loading = true;
                let params = ApReconciliationQueryParams {
                    supplier_id: None,
                    reconciliation_status: if self.filter_status == "全部" { None } else { Some(self.filter_status.clone()) },
                    start_date: None,
                    end_date: None,
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApReconciliationService::list_reconciliations(params).await {
                        Ok(response) => link.send_message(Msg::ReconciliationsLoaded(response.items, response.total)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ReconciliationsLoaded(reconciliations, total) => {
                self.reconciliations = reconciliations;
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
                ctx.link().send_message(Msg::LoadReconciliations);
                false
            }
            Msg::ViewReconciliation(id) => {
                gloo_dialogs::alert("详情页面功能开发中...");
                false
            }
            Msg::GenerateReconciliation => {
                self.show_generate_modal = true;
                true
            }
            Msg::ConfirmReconciliation(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApReconciliationService::confirm_reconciliation(id).await {
                        Ok(_) => link.send_message(Msg::LoadReconciliations),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::DisputeReconciliation(id) => {
                self.selected_id = Some(id);
                self.show_dispute_modal = true;
                true
            }
            Msg::SubmitDispute => {
                if let Some(id) = self.selected_id {
                    let reason = self.dispute_reason.clone();
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match ApReconciliationService::dispute_reconciliation(id, reason).await {
                            Ok(_) => link.send_message(Msg::LoadReconciliations),
                            Err(e) => link.send_message(Msg::LoadError(e)),
                        }
                    });
                }
                self.show_dispute_modal = false;
                self.selected_id = None;
                self.dispute_reason = String::new();
                false
            }
            Msg::ChangePage(page) => {
                self.page = page;
                ctx.link().send_message(Msg::LoadReconciliations);
                false
            }
            Msg::Refresh => {
                ctx.link().send_message(Msg::LoadReconciliations);
                false
            }
            Msg::CloseGenerateModal => {
                self.show_generate_modal = false;
                true
            }
            Msg::CloseDisputeModal => {
                self.show_dispute_modal = false;
                self.selected_id = None;
                self.dispute_reason = String::new();
                true
            }
            Msg::SetDisputeReason(reason) => {
                self.dispute_reason = reason;
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_status_change = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<web_sys::HtmlSelectElement>().unwrap();
            Msg::SetFilterStatus(target.value())
        });

        html! {
            <div class="ap-reconciliation-page">
                <div class="page-header">
                    <h1>{"应付对账管理"}</h1>
                    <div class="header-actions">
                        <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::GenerateReconciliation)}>
                            {"生成对账单"}
                        </button>
                    </div>
                </div>

                <div class="filter-bar">
                    <div class="filter-item">
                        <label>{"对账状态："}</label>
                        <select value={self.filter_status.clone()} onchange={on_status_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="PENDING">{"待对账"}</option>
                            <option value="CONFIRMED">{"已确认"}</option>
                            <option value="DISPUTED">{"有争议"}</option>
                            <option value="APPROVED">{"已核准"}</option>
                        </select>
                    </div>
                    <button class="btn-refresh" onclick={ctx.link().callback(|_| Msg::Refresh)}>
                        {"刷新"}
                    </button>
                </div>

                {self.render_content(ctx)}

                if self.show_dispute_modal {
                    {self.render_dispute_modal(ctx)}
                }
            </div>
        }
    }
}

impl ApReconciliationPage {
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
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadReconciliations)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        if self.reconciliations.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"!"}</div>
                    <p>{"暂无对账单"}</p>
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
                                <th>{"对账单号"}</th>
                                <th>{"供应商"}</th>
                                <th>{"对账日期"}</th>
                                <th>{"对账期间"}</th>
                                <th>{"对账状态"}</th>
                                <th>{"对账金额"}</th>
                                <th>{"已确认金额"}</th>
                                <th>{"争议金额"}</th>
                                <th>{"发票数量"}</th>
                                <th>{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {for self.reconciliations.iter().map(|reconciliation| {
                                let reconciliation_id = reconciliation.id;
                                let status = reconciliation.reconciliation_status.clone();
                                let status_text = match status.as_str() {
                                    "PENDING" => "待对账",
                                    "CONFIRMED" => "已确认",
                                    "DISPUTED" => "有争议",
                                    "APPROVED" => "已核准",
                                    _ => &status,
                                };
                                let status_class = match status.as_str() {
                                    "PENDING" => "status-draft",
                                    "CONFIRMED" => "status-approved",
                                    "DISPUTED" => "status-pending",
                                    "APPROVED" => "status-paid",
                                    _ => "",
                                };
                                html! {
                                    <tr>
                                        <td>{&reconciliation.reconciliation_no}</td>
                                        <td>{reconciliation.supplier_name.as_deref().unwrap_or("-")}</td>
                                        <td>{&reconciliation.reconciliation_date}</td>
                                        <td>{format!("{} ~ {}", reconciliation.period_start, reconciliation.period_end)}</td>
                                        <td>
                                            <span class={format!("status-badge {}", status_class)}>{status_text}</span>
                                        </td>
                                        <td class="numeric">{&reconciliation.total_amount}</td>
                                        <td class="numeric">{reconciliation.confirmed_amount.as_deref().unwrap_or("0.00")}</td>
                                        <td class="numeric">{reconciliation.disputed_amount.as_deref().unwrap_or("0.00")}</td>
                                        <td class="numeric">{reconciliation.invoice_count.to_string()}</td>
                                        <td>
                                            <div class="action-buttons">
                                                <button class="btn-action" onclick={ctx.link().callback(move |_| Msg::ViewReconciliation(reconciliation_id))}>
                                                    {"查看"}
                                                </button>
                                                if reconciliation.reconciliation_status == "PENDING" {
                                                    <button class="btn-action btn-success" onclick={ctx.link().callback(move |_| Msg::ConfirmReconciliation(reconciliation_id))}>
                                                        {"确认"}
                                                    </button>
                                                    <button class="btn-action btn-warning" onclick={ctx.link().callback(move |_| Msg::DisputeReconciliation(reconciliation_id))}>
                                                        {"争议"}
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

    fn render_dispute_modal(&self, ctx: &Context<Self>) -> Html {
        let on_dispute_reason_change = ctx.link().callback(|e: InputEvent| {
            let target = e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
            Msg::SetDisputeReason(target.value())
        });

        html! {
            <div class="modal-overlay">
                <div class="modal-content">
                    <div class="modal-header">
                        <h2>{"提出争议"}</h2>
                        <button class="modal-close" onclick={ctx.link().callback(|_| Msg::CloseDisputeModal)}>{"x"}</button>
                    </div>
                    <div class="modal-body">
                        <div class="form-group">
                            <label>{"争议原因："}</label>
                            <textarea
                                value={self.dispute_reason.clone()}
                                oninput={on_dispute_reason_change}
                                placeholder="请输入争议原因..."
                                rows="4"
                            />
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseDisputeModal)}>
                            {"取消"}
                        </button>
                        <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::SubmitDispute)}>
                            {"提交"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}