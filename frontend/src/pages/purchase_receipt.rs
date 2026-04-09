//! 采购收货单管理页面

use crate::components::main_layout::MainLayout;
use crate::models::purchase_receipt::{PurchaseReceipt, PurchaseReceiptItem, PurchaseReceiptQuery};
use crate::services::purchase_receipt_service::PurchaseReceiptService;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

/// 采购收货单页面状态
pub struct PurchaseReceiptPage {
    receipts: Vec<PurchaseReceipt>,
    loading: bool,
    error: Option<String>,
    filter_status: String,
    page: u64,
    page_size: u64,
    printing_receipt: Option<(PurchaseReceipt, Vec<PurchaseReceiptItem>)>,
    print_trigger: bool,
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
    PreparePrint(i32),
    PrintReady(PurchaseReceipt, Vec<PurchaseReceiptItem>),
    ClearPrint,
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
            printing_receipt: None,
            print_trigger: false,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadReceipts);
        }
        if self.print_trigger {
            self.print_trigger = false;
            if let Some(window) = web_sys::window() {
                let _ = window.print();
            }
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadReceipts => {
                self.loading = true;
                let query = PurchaseReceiptQuery {
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                    status: if self.filter_status == "全部" {
                        None
                    } else {
                        Some(self.filter_status.clone())
                    },
                    supplier_id: None,
                    order_id: None,
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseReceiptService::list(query).await {
                        Ok(receipts) => link.send_message(Msg::ReceiptsLoaded(receipts.items)),
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
                web_sys::window()
                    .unwrap()
                    .location()
                    .set_href(&format!("/purchase-receipts/{}", id))
                    .ok();
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
            Msg::PreparePrint(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    let receipt_res = PurchaseReceiptService::get(id).await;
                    let items_res = PurchaseReceiptService::list_items(id).await;
                    match (receipt_res, items_res) {
                        (Ok(receipt), Ok(items)) => {
                            link.send_message(Msg::PrintReady(receipt, items));
                        }
                        _ => {
                            link.send_message(Msg::LoadError("加载打印数据失败".into()));
                        }
                    }
                });
                false
            }
            Msg::PrintReady(receipt, items) => {
                self.printing_receipt = Some((receipt, items));
                self.print_trigger = true;
                true
            }
            Msg::ClearPrint => {
                self.printing_receipt = None;
                true
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
            <MainLayout current_page={""}>
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
        
</MainLayout>}
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
            <>
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
                            <th>{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.receipts.iter().map(|receipt| {
                            let status = receipt.status.clone();
                            let id = receipt.id;
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
                                    <td>
                                        <button class="btn-secondary" onclick={ctx.link().callback(move |_| Msg::PreparePrint(id))}>
                                            {"打印"}
                                        </button>
                                    </td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
            </div>
            {self.render_print_view()}
            </>
        }
    }

    fn render_print_view(&self) -> Html {
        if let Some((receipt, items)) = &self.printing_receipt {
            html! {
                <div class="print-view" style="display: none;">
                    <style>
                        {"
                        @media print {
                            body * {
                                visibility: hidden;
                            }
                            .print-view, .print-view * {
                                visibility: visible;
                            }
                            .print-view {
                                position: absolute;
                                left: 0;
                                top: 0;
                                width: 100%;
                                display: block !important;
                                padding: 20px;
                            }
                            .print-header {
                                text-align: center;
                                margin-bottom: 20px;
                            }
                            .print-table {
                                width: 100%;
                                border-collapse: collapse;
                            }
                            .print-table th, .print-table td {
                                border: 1px solid #000;
                                padding: 8px;
                                text-align: left;
                            }
                        }
                        "}
                    </style>
                    <div class="print-header">
                        <h2>{"采购收货单"}</h2>
                        <p>{"单号: "}{&receipt.receipt_no}</p>
                    </div>
                    <div class="print-info" style="margin-bottom: 20px;">
                        <p>{"供应商: "}{receipt.supplier_name.as_deref().unwrap_or("-")}</p>
                        <p>{"收货日期: "}{&receipt.receipt_date}</p>
                        <p>{"仓库: "}{receipt.warehouse_name.as_deref().unwrap_or("-")}</p>
                    </div>
                    <table class="print-table">
                        <thead>
                            <tr>
                                <th>{"序号"}</th>
                                <th>{"物料名称"}</th>
                                <th>{"规格"}</th>
                                <th>{"数量"}</th>
                                <th>{"单价"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {for items.iter().map(|item| {
                                html! {
                                    <tr>
                                        <td>{item.line_no}</td>
                                        <td>{&item.material_name}</td>
                                        <td>{item.specification.as_deref().unwrap_or("-")}</td>
                                        <td>{&item.quantity_received}</td>
                                        <td>{&item.unit_price}</td>
                                    </tr>
                                }
                            })}
                        </tbody>
                    </table>
                </div>
            }
        } else {
            html! {}
        }
    }
}
