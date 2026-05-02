use gloo_dialogs;
// 采购收货单管理页面

use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::models::purchase_receipt::{
    PurchaseReceipt, PurchaseReceiptQuery, PurchaseReceiptItem
};
use crate::services::purchase_receipt_service::PurchaseReceiptService;
use crate::services::crud_service::CrudService;

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

    viewing_item: Option<PurchaseReceipt>,}

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

    CloseDetailModal,}

impl Component for PurchaseReceiptPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            viewing_item: None,
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
            Msg::CloseDetailModal => {
                self.viewing_item = None;
                true
            }
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
                    match PurchaseReceiptService::list_with_query(&query).await {
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
                self.viewing_item = self.receipts.iter().find(|i| i.id == id).cloned();
                true
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
        let on_status_change = ctx.link().batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlSelectElement>().ok()?;
            Some(Msg::SetFilterStatus(target.value()))
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
                {self.render_detail_modal(ctx)}
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
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Receipt No: "}</span>
                                    <span class="detail-value">{&item.receipt_no}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Order Id: "}</span>
                                    <span class="detail-value">{item.order_id.to_string()}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Order No: "}</span>
                                    <span class="detail-value">{item.order_no.as_deref().unwrap_or("-")}</span>
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
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Receipt Date: "}</span>
                                    <span class="detail-value">{&item.receipt_date}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Status: "}</span>
                                    <span class="detail-value">{&item.status}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Total Quantity: "}</span>
                                    <span class="detail-value">{&item.total_quantity}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Total Amount: "}</span>
                                    <span class="detail-value">{&item.total_amount}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Warehouse Id: "}</span>
                                    <span class="detail-value">{item.warehouse_id.to_string()}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Warehouse Name: "}</span>
                                    <span class="detail-value">{item.warehouse_name.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Department Id: "}</span>
                                    <span class="detail-value">{item.department_id.to_string()}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Department Name: "}</span>
                                    <span class="detail-value">{item.department_name.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Inspector: "}</span>
                                    <span class="detail-value">{item.inspector.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Inspection Date: "}</span>
                                    <span class="detail-value">{item.inspection_date.as_deref().unwrap_or("-")}</span>
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
