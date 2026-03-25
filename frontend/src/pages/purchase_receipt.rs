//! 采购收货单管理页面

use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::services::purchase_receipt_service::{
    PurchaseReceiptService, PurchaseReceipt, PurchaseReceiptQuery,
};

/// 采购收货单页面状态
pub struct PurchaseReceiptPage {
    receipts: Vec<PurchaseReceipt>,
    loading: bool,
    error: Option<String>,
    filter_status: String,
    page: u64,
    page_size: u64,
}

/// 模态框模式
#[derive(Clone, PartialEq)]
pub enum ModalMode {
    View,
    Create,
    Edit,
}

pub enum Msg {
    LoadReceipts,
    ReceiptsLoaded(Vec<PurchaseReceipt>),
    LoadError(String),
    SetFilterStatus(String),
    ViewReceipt(i32),
    ConfirmReceipt(i32),
    ChangePage(u64),
}

impl Component for PurchaseReceiptPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            receipts: Vec::new(),
            loading: true,
            error: None,
            filter_status: String::from("全部"),
            page: 1,
            page_size: 20,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadReceipts);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadReceipts => {
                self.loading = true;
                let query = PurchaseReceiptQuery {
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                    status: if self.filter_status == "全部" { None } else { Some(self.filter_status.clone()) },
                    supplier_id: None,
                    order_id: None,
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseReceiptService::list(query).await {
                        Ok(receipts) => link.send_message(Msg::ReceiptsLoaded(receipts)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ReceiptsLoaded(receipts) => {
                self.receipts = receipts;
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
                ctx.link().send_message(Msg::LoadReceipts);
                false
            }
            Msg::ViewReceipt(id) => {
                web_sys::window().unwrap().location().set_href(&format!("/purchase-receipts/{}", id)).ok();
                false
            }
            Msg::ConfirmReceipt(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseReceiptService::confirm(id).await {
                        Ok(_) => link.send_message(Msg::LoadReceipts),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ChangePage(page) => {
                self.page = page;
                ctx.link().send_message(Msg::LoadReceipts);
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
            <div class="purchase-receipt-page">
                <div class="page-header">
                    <h1>{"📥 采购收货单管理"}</h1>
                </div>

                <div class="filter-bar">
                    <div class="filter-item">
                        <label>{"收货单状态："}</label>
                        <select value={self.filter_status.clone()} onchange={on_status_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="草稿">{"草稿"}</option>
                            <option value="已确认">{"已确认"}</option>
                            <option value="已入库">{"已入库"}</option>
                        </select>
                    </div>
                </div>

                {self.render_content(ctx)}
            </div>
        }
    }
}

impl PurchaseReceiptPage {
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
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadReceipts)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        if self.receipts.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"📥"}</div>
                    <p>{"暂无采购收货单"}</p>
                </div>
            };
        }

        html! {
            <div class="table-responsive">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>{"收货单编号"}</th>
                            <th>{"关联订单号"}</th>
                            <th>{"供应商"}</th>
                            <th>{"收货日期"}</th>
                            <th>{"收货单状态"}</th>
                            <th>{"总数量"}</th>
                            <th>{"总金额"}</th>
                            <th>{"仓库"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.receipts.iter().map(|receipt| {
                            let status = receipt.status.clone();
                            html! {
                                <tr>
                                    <td>{&receipt.receipt_no}</td>
                                    <td>{receipt.order_no.as_deref().unwrap_or("-")}</td>
                                    <td>{receipt.supplier_name.as_deref().unwrap_or("-")}</td>
                                    <td>{&receipt.receipt_date}</td>
                                    <td>{status}</td>
                                    <td class="numeric">{&receipt.total_quantity}</td>
                                    <td class="numeric">{&receipt.total_amount}</td>
                                    <td>{receipt.warehouse_name.as_deref().unwrap_or("-")}</td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
            </div>
        }
    }
}