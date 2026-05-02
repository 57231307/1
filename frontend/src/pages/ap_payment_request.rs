use gloo_dialogs;
// 付款申请管理页面
//
// 付款申请（AP Payment Request）管理功能

use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::models::ap_payment_request::{
    ApPaymentRequest, ApPaymentRequestQueryParams,
};
use crate::services::ap_payment_request_service::ApPaymentRequestService;

/// 付款申请管理页面状态
pub struct ApPaymentRequestPage {
    requests: Vec<ApPaymentRequest>,
    loading: bool,
    error: Option<String>,
    filter_status: String,
    filter_type: String,
    page: u64,
    page_size: u64,
    total: u64,

    viewing_item: Option<ApPaymentRequest>,}

/// 模态框模式
#[derive(Clone, PartialEq)]
pub enum ModalMode {
    View,
    Create,
    Edit,
}

pub enum Msg {
    LoadRequests,
    RequestsLoaded(Vec<ApPaymentRequest>, u64),
    LoadError(String),
    SetFilterStatus(String),
    SetFilterType(String),
    ViewRequest(i32),
    DeleteRequest(i32),
    SubmitRequest(i32),
    ApproveRequest(i32),
    RejectRequest(i32, String),
    ChangePage(u64),
    Refresh,

    CloseDetailModal,}

impl Component for ApPaymentRequestPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            viewing_item: None,
            requests: Vec::new(),
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
            ctx.link().send_message(Msg::LoadRequests);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::CloseDetailModal => {
                self.viewing_item = None;
                true
            }
            Msg::LoadRequests => {
                self.loading = true;
                let params = ApPaymentRequestQueryParams {
                    supplier_id: None,
                    approval_status: if self.filter_status == "全部" { None } else { Some(self.filter_status.clone()) },
                    payment_type: if self.filter_type == "全部" { None } else { Some(self.filter_type.clone()) },
                    start_date: None,
                    end_date: None,
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApPaymentRequestService::list_with_query(&params).await {
                        Ok(response) => link.send_message(Msg::RequestsLoaded(response.items, response.total)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::RequestsLoaded(requests, total) => {
                self.requests = requests;
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
                ctx.link().send_message(Msg::LoadRequests);
                false
            }
            Msg::SetFilterType(tp) => {
                self.filter_type = tp;
                self.page = 1;
                ctx.link().send_message(Msg::LoadRequests);
                false
            }
            Msg::ViewRequest(id) => {
                self.viewing_item = self.requests.iter().find(|i| i.id == id).cloned();
                true
            }
            Msg::DeleteRequest(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApPaymentRequestService::delete(id).await {
                        Ok(_) => link.send_message(Msg::LoadRequests),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::SubmitRequest(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApPaymentRequestService::submit_request(id).await {
                        Ok(_) => link.send_message(Msg::LoadRequests),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ApproveRequest(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApPaymentRequestService::approve_request(id).await {
                        Ok(_) => link.send_message(Msg::LoadRequests),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::RejectRequest(id, reason) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApPaymentRequestService::reject_request(id, reason).await {
                        Ok(_) => link.send_message(Msg::LoadRequests),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ChangePage(page) => {
                self.page = page;
                ctx.link().send_message(Msg::LoadRequests);
                false
            }
            Msg::Refresh => {
                ctx.link().send_message(Msg::LoadRequests);
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
            <div class="ap-payment-request-page">
                <div class="page-header">
                    <h1>{"付款申请管理"}</h1>
                </div>

                <div class="filter-bar">
                    <div class="filter-item">
                        <label>{"审批状态："}</label>
                        <select value={self.filter_status.clone()} onchange={on_status_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="DRAFT">{"草稿"}</option>
                            <option value="APPROVING">{"审批中"}</option>
                            <option value="APPROVED">{"已审批"}</option>
                            <option value="REJECTED">{"已拒绝"}</option>
                        </select>
                    </div>
                    <div class="filter-item">
                        <label>{"付款类型："}</label>
                        <select value={self.filter_type.clone()} onchange={on_type_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="PREPAYMENT">{"预付款"}</option>
                            <option value="PROGRESS">{"进度款"}</option>
                            <option value="FINAL">{"尾款"}</option>
                            <option value="WARRANTY">{"质保金"}</option>
                        </select>
                    </div>
                    <button class="btn-refresh" onclick={ctx.link().callback(|_| Msg::Refresh)}>
                        {"刷新"}
                    </button>
                </div>

                {self.render_content(ctx)}
                {self.render_detail_modal(ctx)}
            </div>
        }
    }
}

impl ApPaymentRequestPage {
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
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadRequests)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        if self.requests.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"!"}</div>
                    <p>{"暂无付款申请"}</p>
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
                                <th>{"申请单号"}</th>
                                <th>{"供应商"}</th>
                                <th>{"申请日期"}</th>
                                <th>{"付款类型"}</th>
                                <th>{"付款方式"}</th>
                                <th>{"申请金额"}</th>
                                <th>{"审批状态"}</th>
                                <th>{"期望付款日期"}</th>
                                <th>{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {for self.requests.iter().map(|request| {
                                let request_id = request.id;
                                let status = request.approval_status.clone();
                                let status_text = match status.as_str() {
                                    "DRAFT" => "草稿",
                                    "APPROVING" => "审批中",
                                    "APPROVED" => "已审批",
                                    "REJECTED" => "已拒绝",
                                    _ => &status,
                                };
                                let status_class = match status.as_str() {
                                    "DRAFT" => "status-draft",
                                    "APPROVING" => "status-pending",
                                    "APPROVED" => "status-approved",
                                    "REJECTED" => "status-rejected",
                                    _ => "",
                                };
                                let payment_type_text = match request.payment_type.as_str() {
                                    "PREPAYMENT" => "预付款",
                                    "PROGRESS" => "进度款",
                                    "FINAL" => "尾款",
                                    "WARRANTY" => "质保金",
                                    _ => &request.payment_type,
                                };
                                let payment_method_text = match request.payment_method.as_str() {
                                    "TT" => "电汇",
                                    "LC" => "信用证",
                                    "DP" => "付款交单",
                                    "DA" => "承兑交单",
                                    "CHECK" => "支票",
                                    "CASH" => "现金",
                                    _ => &request.payment_method,
                                };
                                html! {
                                    <tr>
                                        <td>{&request.request_no}</td>
                                        <td>{request.supplier_name.as_deref().unwrap_or("-")}</td>
                                        <td>{&request.request_date}</td>
                                        <td>{payment_type_text}</td>
                                        <td>{payment_method_text}</td>
                                        <td class="numeric">{&request.request_amount.to_string()}</td>
                                        <td>
                                            <span class={format!("status-badge {}", status_class)}>{status_text}</span>
                                        </td>
                                        <td>{request.expected_payment_date.as_deref().unwrap_or("-")}</td>
                                        <td>
                                            <div class="action-buttons">
                                                <button class="btn-action" onclick={ctx.link().callback(move |_| Msg::ViewRequest(request_id))}>
                                                    {"查看"}
                                                </button>
                                                if request.approval_status == "DRAFT" {
                                                    <button class="btn-action btn-primary" onclick={ctx.link().callback(move |_| Msg::SubmitRequest(request_id))}>
                                                        {"提交"}
                                                    </button>
                                                    <button class="btn-action btn-danger" onclick={ctx.link().callback(move |_| Msg::DeleteRequest(request_id))}>
                                                        {"删除"}
                                                    </button>
                                                }
                                                if request.approval_status == "APPROVING" {
                                                    <button class="btn-action btn-success" onclick={ctx.link().callback(move |_| Msg::ApproveRequest(request_id))}>
                                                        {"审批"}
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
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Request No: "}</span>
                                    <span class="detail-value">{&item.request_no}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Request Date: "}</span>
                                    <span class="detail-value">{&item.request_date}</span>
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
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Payment Type: "}</span>
                                    <span class="detail-value">{&item.payment_type}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Payment Method: "}</span>
                                    <span class="detail-value">{&item.payment_method}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Request Amount: "}</span>
                                    <span class="detail-value">{&item.request_amount}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Approval Status: "}</span>
                                    <span class="detail-value">{&item.approval_status}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Currency: "}</span>
                                    <span class="detail-value">{&item.currency}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Exchange Rate: "}</span>
                                    <span class="detail-value">{&item.exchange_rate}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Request Amount Foreign: "}</span>
                                    <span class="detail-value">{item.request_amount_foreign.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Expected Payment Date: "}</span>
                                    <span class="detail-value">{item.expected_payment_date.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Bank Name: "}</span>
                                    <span class="detail-value">{item.bank_name.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Bank Account: "}</span>
                                    <span class="detail-value">{item.bank_account.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Bank Account Name: "}</span>
                                    <span class="detail-value">{item.bank_account_name.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Notes: "}</span>
                                    <span class="detail-value">{item.notes.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Attachment Urls: "}</span>
                                    <span class="detail-value">{item.attachment_urls.as_ref().map_or("-".to_string(), |v| v.join(", "))}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Created By: "}</span>
                                    <span class="detail-value">{item.created_by.to_string()}</span>
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
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Updated By: "}</span>
                                    <span class="detail-value">{item.updated_by.map_or("-".to_string(), |v| v.to_string())}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Updated At: "}</span>
                                    <span class="detail-value">{&item.updated_at}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Submitted By: "}</span>
                                    <span class="detail-value">{item.submitted_by.map_or("-".to_string(), |v| v.to_string())}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Submitter Name: "}</span>
                                    <span class="detail-value">{item.submitter_name.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Submitted At: "}</span>
                                    <span class="detail-value">{item.submitted_at.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Approved By: "}</span>
                                    <span class="detail-value">{item.approved_by.map_or("-".to_string(), |v| v.to_string())}</span>
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
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Rejected By: "}</span>
                                    <span class="detail-value">{item.rejected_by.map_or("-".to_string(), |v| v.to_string())}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Rejecter Name: "}</span>
                                    <span class="detail-value">{item.rejecter_name.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Rejected At: "}</span>
                                    <span class="detail-value">{item.rejected_at.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Rejected Reason: "}</span>
                                    <span class="detail-value">{item.rejected_reason.as_deref().unwrap_or("-")}</span>
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
