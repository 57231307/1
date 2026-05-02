use gloo_dialogs;
// 采购退货管理页面

use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::models::purchase_return::{CreatePurchaseReturnRequest, CreatePurchaseReturnItemRequest, 
    PurchaseReturn, PurchaseReturnQuery,
};
use crate::services::purchase_return_service::PurchaseReturnService;

/// 采购退货页面状态管理
pub struct PurchaseReturnPage {
    printing_return: Option<crate::models::purchase_return::PurchaseReturn>,
    print_trigger: bool,
    show_modal: bool,
    new_return_no: String,
    new_supplier_id: String,
    new_product_id: String,
    new_quantity: String,
    new_reason: String,
    returns: Vec<PurchaseReturn>,
    loading: bool,
    error: Option<String>,
    filter_status: String,
    page: u64,
    page_size: u64,

    viewing_item: Option<PurchaseReturn>,}

/// 消息枚举
pub enum Msg {
    LoadReturns,
    ReturnsLoaded(Vec<PurchaseReturn>),
    LoadError(String),
    SetFilterStatus(String),
    ViewReturn(i32),
    DeleteReturn(i32),
    SubmitReturn(i32),
    ApproveReturn(i32),
    RejectReturn(i32),
    ChangePage(u64),
    OpenModal,
    CloseModal,
    UpdateInput(String, String),
    SubmitCreate,

    CloseDetailModal,}

impl Component for PurchaseReturnPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            viewing_item: None,
            returns: Vec::new(),
            loading: true,
            printing_return: None,
            print_trigger: false,
            show_modal: false,
            new_return_no: String::new(),
            new_supplier_id: String::new(),
            new_product_id: String::new(),
            new_quantity: String::new(),
            new_reason: String::new(),
            error: None,
            filter_status: String::from("全部"),
            page: 1,
            page_size: 20,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadReturns);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::CloseDetailModal => {
                self.viewing_item = None;
                true
            }
            Msg::LoadReturns => {
                self.loading = true;
                let query = PurchaseReturnQuery {
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                    status: if self.filter_status == "全部" { None } else { Some(self.filter_status.clone()) },
                    supplier_id: None,
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseReturnService::list_with_query(&query).await {
                        Ok(returns) => link.send_message(Msg::ReturnsLoaded(returns.items)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ReturnsLoaded(returns) => {
                self.returns = returns;
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
                ctx.link().send_message(Msg::LoadReturns);
                false
            }
            Msg::ViewReturn(id) => {
                self.viewing_item = self.returns.iter().find(|i| i.id == id).cloned();
                true
            }
            Msg::DeleteReturn(_id) => {
                // 删除功能暂未实现
                false
            }
            Msg::SubmitReturn(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseReturnService::submit(id).await {
                        Ok(_) => link.send_message(Msg::LoadReturns),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ApproveReturn(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseReturnService::approve(id).await {
                        Ok(_) => link.send_message(Msg::LoadReturns),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::RejectReturn(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseReturnService::reject(id, "不符合要求".to_string()).await {
                        Ok(_) => link.send_message(Msg::LoadReturns),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }

            Msg::OpenModal => {
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                true
            }
            Msg::UpdateInput(field, value) => {
                match field.as_str() {
                    "return_no" => self.new_return_no = value,
                    "supplier_id" => self.new_supplier_id = value,
                    "product_id" => self.new_product_id = value,
                    "quantity" => self.new_quantity = value,
                    "reason" => self.new_reason = value,
                    _ => {}
                }
                true
            }
            Msg::SubmitCreate => {
                use std::str::FromStr;
                let req = CreatePurchaseReturnRequest {
                    return_no: self.new_return_no.clone(),
                    supplier_id: i32::from_str(&self.new_supplier_id).unwrap_or(0),
                    order_id: None,
                    return_date: Some(String::new()),
                    reason_type: "质量问题".to_string(),
                    reason_detail: Some(self.new_reason.clone()),
                    remarks: None,
                    items: vec![CreatePurchaseReturnItemRequest {
                        product_id: i32::from_str(&self.new_product_id).unwrap_or(0),
                        quantity: rust_decimal::Decimal::from_str(&self.new_quantity).unwrap_or_default(),
                        unit_price: None,
                    }],
                };
                let link = ctx.link().clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = crate::services::purchase_return_service::PurchaseReturnService::create(req).await;
                    link.send_message(Msg::LoadReturns);
                    link.send_message(Msg::CloseModal);
                });
                true
            }
            Msg::ChangePage(page) => {
                self.page = page;
                ctx.link().send_message(Msg::LoadReturns);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_status_change = ctx.link().batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlSelectElement>().ok()?;
            Some(Msg::SetFilterStatus(target.value()))
        });

        html! {
            <div class="purchase-return-page">
                <div class="page-header">
                    <h1>{"退货管理"}</h1>
                    <button class="bg-indigo-600 text-white px-4 py-2 rounded shadow hover:bg-indigo-700" onclick={ctx.link().callback(|_| Msg::OpenModal)}>
                        {"新建退货单"}
                    </button>
                </div>

                <div class="filter-bar">
                    <div class="filter-item">
                        <label>{"退货状态："}</label>
                        <select value={self.filter_status.clone()} onchange={on_status_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="草稿">{"草稿"}</option>
                            <option value="待审批">{"待审批"}</option>
                            <option value="已审批">{"已审批"}</option>
                            <option value="已拒绝">{"已拒绝"}</option>
                        </select>
                    </div>
                </div>

                {self.render_content(ctx)}
                {self.render_detail_modal(ctx)}
                {self.render_print_view()}
                {self.render_modal(ctx)}
            </div>
        }
    }
}

impl PurchaseReturnPage {
    
    fn render_modal(&self, ctx: &Context<Self>) -> Html {
        if !self.show_modal { return html! {}; }
        let on_input = |field: &'static str| {
            ctx.link().callback(move |e: InputEvent| {
                let input = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                Msg::UpdateInput(field.to_string(), input.value())
            })
        };
        html! {
            <div class="fixed inset-0 z-50 flex items-center justify-center overflow-x-hidden overflow-y-auto outline-none focus:outline-none">
                <div class="fixed inset-0 bg-gray-900 bg-opacity-50 transition-opacity" onclick={ctx.link().callback(|_| Msg::CloseModal)}></div>
                <div class="relative w-full max-w-lg mx-auto my-6 z-50">
                    <div class="relative flex flex-col w-full bg-white border-0 rounded-xl shadow-2xl outline-none focus:outline-none p-6">
                        <h3 class="text-2xl font-semibold mb-4">{"新建采购退货单"}</h3>
                        <div class="grid grid-cols-1 gap-4 mb-4">
                            <input placeholder="退单号 (例如: PR20260401)" class="w-full px-3 py-2 border rounded" oninput={on_input("return_no")} value={self.new_return_no.clone()} />
                            <input placeholder="退货原因说明" class="w-full px-3 py-2 border rounded" oninput={on_input("reason")} value={self.new_reason.clone()} />
                            <h4 class="font-semibold">{"退回商品明细"}</h4>
                            <input placeholder="产品 ID" class="w-full px-3 py-2 border rounded" oninput={on_input("product_id")} value={self.new_product_id.clone()} />
                            <input placeholder="退回数量" class="w-full px-3 py-2 border rounded" oninput={on_input("quantity")} value={self.new_quantity.clone()} />
                        </div>
                        <div class="flex justify-end gap-2">
                            <button class="px-4 py-2 text-gray-500 hover:bg-gray-100 rounded" onclick={ctx.link().callback(|_| Msg::CloseModal)}>{"取消"}</button>
                            <button class="px-4 py-2 bg-indigo-600 text-white rounded hover:bg-indigo-700" onclick={ctx.link().callback(|_| Msg::SubmitCreate)}>{"确认创建"}</button>
                        </div>
                    </div>
                </div>
            </div>
        }
    }

    
    fn render_print_view(&self) -> Html {
        if let Some(ret) = &self.printing_return {
            html! {
                <div class="print-view" style="display: none;">
                    <div class="print-header">
                        <h2>{"秉羲面料管理 - 采购退货单"}</h2>
                    </div>
                    <div class="print-info-grid">
                        <div><strong>{"退货单号："}</strong> {&ret.return_no}</div>
                        <div><strong>{"供应商 ID："}</strong> {&ret.supplier_id}</div>
                        <div><strong>{"原订单 ID："}</strong> {ret.order_id}</div>
                        <div><strong>{"退货总额："}</strong> {&ret.total_amount}</div>
                        <div><strong>{"退货原因："}</strong> {ret.reason.as_deref().unwrap_or("-")}</div>
                        <div><strong>{"备注："}</strong> {ret.notes.as_deref().unwrap_or("-")}</div>
                        <div><strong>{"状态："}</strong> {&ret.status}</div>
                    </div>
                    <table class="print-table">
                        <thead>
                            <tr>
                                <th>{"产品 ID"}</th>
                                <th>{"退货数量"}</th>
                                <th>{"单价"}</th>
                                <th>{"金额"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr>
                                <td colspan="4" style="text-align: center; padding: 20px;">{"【明细项请在详情页查看并打印】"}</td>
                            </tr>
                        </tbody>
                    </table>
                    <div class="print-footer">
                        <div class="print-signature">{"仓库退货员"}</div>
                        <div class="print-signature">{"审批人"}</div>
                        <div class="print-signature">{"供应商确认"}</div>
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
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadReturns)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        if self.returns.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"📦"}</div>
                    <p>{"暂无采购退货单"}</p>
                </div>
            };
        }

        html! {
            <div class="table-responsive">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>{"退货单号"}</th>
                            <th>{"关联订单"}</th>
                            <th>{"供应商"}</th>
                            <th>{"退货日期"}</th>
                            <th>{"退货状态"}</th>
                            <th>{"退货数量"}</th>
                            <th>{"退货金额"}</th>
                            <th>{"仓库"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.returns.iter().map(|ret| {
                            let status = ret.status.clone();
                            html! {
                                <tr>
                                    <td>{&ret.return_no}</td>
                                    <td>{ret.order_no.as_deref().unwrap_or("-")}</td>
                                    <td>{ret.supplier_name.as_deref().unwrap_or("-")}</td>
                                    <td>{&ret.return_date}</td>
                                    <td>{status}</td>
                                    <td class="numeric">{&ret.total_quantity}</td>
                                    <td class="numeric">{&ret.total_amount}</td>
                                    <td>{ret.warehouse_name.as_deref().unwrap_or("-")}</td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
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
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Return No: "}</span>
                                    <span class="detail-value">{&item.return_no}</span>
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
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Return Date: "}</span>
                                    <span class="detail-value">{&item.return_date}</span>
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
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"Reason: "}</span>
                                    <span class="detail-value">{item.reason.as_deref().unwrap_or("-")}</span>
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
