//! 采购订单管理页面

use crate::components::main_layout::MainLayout;
use crate::models::purchase_order::{PurchaseOrder, PurchaseOrderQuery};
use crate::services::purchase_order_service::PurchaseOrderService;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

pub struct PurchaseOrderPage {
    printing_order: Option<crate::models::purchase_order::PurchaseOrder>,
    print_trigger: bool,
    orders: Vec<PurchaseOrder>,
    loading: bool,
    error: Option<String>,
    filter_status: String,
    page: u64,
    page_size: u64,
}

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    View,
    Create,
    Edit,
}

pub enum Msg {
    LoadOrders,
    OrdersLoaded(Vec<PurchaseOrder>),
    LoadError(String),
    SetFilterStatus(String),
    ViewOrder(i32),
    DeleteOrder(i32),
    SubmitOrder(i32),
    PrintOrder(crate::models::purchase_order::PurchaseOrder),
    ClearPrint,
    ApproveOrder(i32),
    RejectOrder(i32),
    CloseOrder(i32),
    ChangePage(u64),
}

impl Component for PurchaseOrderPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            orders: Vec::new(),
            loading: true,
            printing_order: None,
            print_trigger: false,
            error: None,
            filter_status: String::from("全部"),
            page: 1,
            page_size: 20,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadOrders);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadOrders => {
                self.loading = true;
                let query = PurchaseOrderQuery {
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                    status: if self.filter_status == "全部" {
                        None
                    } else {
                        Some(self.filter_status.clone())
                    },
                    supplier_id: None,
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseOrderService::list(query).await {
                        Ok(orders) => link.send_message(Msg::OrdersLoaded(orders)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::OrdersLoaded(orders) => {
                self.orders = orders;
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
                ctx.link().send_message(Msg::LoadOrders);
                false
            }
            Msg::ViewOrder(id) => {
                web_sys::window()
                    .unwrap()
                    .location()
                    .set_href(&format!("/purchase-orders/{}", id))
                    .ok();
                false
            }
            Msg::DeleteOrder(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseOrderService::delete(id).await {
                        Ok(_) => link.send_message(Msg::LoadOrders),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }

            Msg::PrintOrder(order) => {
                self.printing_order = Some(order);
                self.print_trigger = true;
                true
            }
            Msg::ClearPrint => {
                self.printing_order = None;
                self.print_trigger = false;
                true
            }
            Msg::SubmitOrder(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseOrderService::submit(id).await {
                        Ok(_) => link.send_message(Msg::LoadOrders),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ApproveOrder(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseOrderService::approve(id).await {
                        Ok(_) => link.send_message(Msg::LoadOrders),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::RejectOrder(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseOrderService::reject(id, "不符合要求".to_string()).await {
                        Ok(_) => link.send_message(Msg::LoadOrders),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::CloseOrder(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseOrderService::close(id).await {
                        Ok(_) => link.send_message(Msg::LoadOrders),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ChangePage(page) => {
                self.page = page;
                ctx.link().send_message(Msg::LoadOrders);
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
            <MainLayout current_page={"/purchase-orders"}>
<div class="purchase-order-page">
                <div class="page-header">
                    <h1>{"📦 采购订单管理"}</h1>
                </div>

                <div class="filter-bar">
                    <div class="filter-item">
                        <label>{"订单状态："}</label>
                        <select value={self.filter_status.clone()} onchange={on_status_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="草稿">{"草稿"}</option>
                            <option value="待审批">{"待审批"}</option>
                            <option value="已审批">{"已审批"}</option>
                            <option value="已关闭">{"已关闭"}</option>
                            <option value="已拒绝">{"已拒绝"}</option>
                        </select>
                    </div>
                </div>

                {self.render_content(ctx)}
                {self.render_print_view()}
            </div>
        
</MainLayout>}
    }
}

impl PurchaseOrderPage {
    fn render_print_view(&self) -> Html {
        if let Some(order) = &self.printing_order {
            html! {
                <div class="print-view" style="display: none;">
                    <div class="print-header">
                        <h2>{"秉羲管理系统 - 采购订单"}</h2>
                    </div>
                    <div class="print-info-grid">
                        <div><strong>{"订单编号："}</strong> {&order.order_no}</div>
                        <div><strong>{"订单日期："}</strong> {&order.order_date}</div>
                        <div><strong>{"供应商："}</strong> {order.supplier_name.as_deref().unwrap_or("-")}</div>
                        <div><strong>{"要求交货期："}</strong> {order.expected_delivery_date.as_deref().unwrap_or("-")}</div>
                        <div><strong>{"采购总金额："}</strong> {&order.total_amount} {order.currency.as_deref().unwrap_or("")}</div>
                        <div><strong>{"采购员："}</strong> {order.purchaser_id.unwrap_or(0)}</div>
                    </div>
                    <table class="print-table">
                        <thead>
                            <tr>
                                <th>{"序号"}</th>
                                <th>{"产品名称"}</th>
                                <th>{"规格"}</th>
                                <th>{"数量"}</th>
                                <th>{"单价"}</th>
                                <th>{"小计"}</th>
                                <th>{"备注"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            // 实际项目中这里应该渲染 items，但目前 purchase_order.rs 的列表中没有展开 items
                            // 所以留出空行或仅打印主表信息
                            <tr>
                                <td colspan="7" style="text-align: center; padding: 20px;">{"【订单明细请在详情页查看并打印】"}</td>
                            </tr>
                        </tbody>
                    </table>
                    <div class="print-footer">
                        <div class="print-signature">{"制单人签字"}</div>
                        <div class="print-signature">{"审批人签字"}</div>
                        <div class="print-signature">{"供应商确认盖章"}</div>
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
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadOrders)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        if self.orders.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"📦"}</div>
                    <p>{"暂无采购订单"}</p>
                </div>
            };
        }

        html! {
            <div class="table-responsive">
                <table class="data-table w-full">
                    <thead>
                        <tr>
                            <th>{"订单编号"}</th>
                            <th>{"供应商"}</th>
                            <th>{"订单日期"}</th>
                            <th>{"要求交货日期"}</th>
                            <th>{"订单状态"}</th>
                            <th class="numeric-cell text-right">{"总金额"}</th>
                            <th>{"仓库"}</th>
                            <th>{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.orders.iter().map(|order| {
                            let status = order.status.clone();
                            let order_id = order.id;
                            let status_check = status.clone();
                            html! {
                                <tr>
                                    <td>{&order.order_no}</td>
                                    <td>{order.supplier_name.as_deref().unwrap_or("-")}</td>
                                    <td>{&order.order_date}</td>
                                    <td>{order.expected_delivery_date.as_deref().unwrap_or("-")}</td>
                                    <td><span class="status-badge">{status}</span></td>
                                    <td class="numeric-cell text-right">{&order.total_amount}</td>
                                    <td>{order.warehouse_name.as_deref().unwrap_or("-")}</td>
                                    <td>
                                        {if status_check == "REJECTED" || status_check == "DRAFT" {
                                            html! {
                                                <button class="px-3 py-1 bg-indigo-600 text-white rounded text-xs" onclick={ctx.link().callback(move |_| Msg::SubmitOrder(order_id))}>{"提交审批"}</button>
                                            }
                                        } else if status_check == "PENDING" {
                                            html! {
                                                <>
                                                    <button class="px-3 py-1 bg-green-600 text-white rounded text-xs ml-2" onclick={ctx.link().callback(move |_| Msg::ApproveOrder(order_id))}>{"审批通过"}</button>
                                                    <button class="px-3 py-1 bg-red-600 text-white rounded text-xs ml-2" onclick={ctx.link().callback(move |_| Msg::RejectOrder(order_id))}>{"驳回"}</button>
                                                </>
                                            }
                                        } else if status_check == "APPROVED" {
                                            html! {
                                                <button class="px-3 py-1 bg-yellow-600 text-white rounded text-xs ml-2" onclick={ctx.link().callback(move |_| Msg::CloseOrder(order_id))}>{"关闭订单"}</button>
                                            }
                                        } else {
                                            html! {}
                                        }}
                                        <button class="px-3 py-1 bg-gray-500 text-white rounded text-xs ml-2" onclick={let order_print = order.clone(); ctx.link().callback(move |_| Msg::PrintOrder(order_print.clone()))}>{"打印"}</button>
                                    </td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
            </div>
        }
    }
}
