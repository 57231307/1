//! 销售订单管理页面

use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use crate::models::sales::SalesOrder;
use crate::services::sales_service::SalesService;

pub struct SalesOrderPage {
    orders: Vec<SalesOrder>,
    loading: bool,
    error: Option<String>,
    page: u64,
    page_size: u64,
    printing_order: Option<SalesOrder>,
    print_trigger: bool,
}

pub enum Msg {
    LoadOrders,
    OrdersLoaded(Vec<SalesOrder>),
    LoadError(String),
    PreparePrint(i32),
    PrintReady(SalesOrder),
}

impl Component for SalesOrderPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            orders: Vec::new(),
            loading: true,
            error: None,
            page: 1,
            page_size: 20,
            printing_order: None,
            print_trigger: false,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadOrders);
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
            Msg::LoadOrders => {
                self.loading = true;
                let link = ctx.link().clone();
                spawn_local(async move {
                    match SalesService::list_orders().await {
                        Ok(res) => link.send_message(Msg::OrdersLoaded(res.orders)),
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
            Msg::PreparePrint(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match SalesService::get_order(id).await {
                        Ok(order) => {
                            link.send_message(Msg::PrintReady(order));
                        }
                        Err(_) => {
                            link.send_message(Msg::LoadError("加载订单打印数据失败".into()));
                        }
                    }
                });
                false
            }
            Msg::PrintReady(order) => {
                self.printing_order = Some(order);
                self.print_trigger = true;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="sales-order-page">
                <div class="page-header">
                    <h1>{"📦 销售订单管理"}</h1>
                </div>

                {self.render_content(ctx)}
            </div>
        }
    }
}

impl SalesOrderPage {
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
                    <p>{"暂无销售订单"}</p>
                </div>
            };
        }

        html! {
            <>
            <div class="table-responsive">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>{"订单号"}</th>
                            <th>{"客户"}</th>
                            <th>{"总金额"}</th>
                            <th>{"状态"}</th>
                            <th>{"创建时间"}</th>
                            <th>{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.orders.iter().map(|order| {
                            let id = order.id;
                            html! {
                                <tr>
                                    <td>{&order.order_no}</td>
                                    <td>{order.customer_name.as_deref().unwrap_or("-")}</td>
                                    <td class="numeric">{&order.total_amount}</td>
                                    <td>{&order.status}</td>
                                    <td>{&order.created_at}</td>
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
        if let Some(order) = &self.printing_order {
            let items = order.items.clone().unwrap_or_default();
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
                        <h2>{"销售订单"}</h2>
                        <p>{"订单号: "}{&order.order_no}</p>
                    </div>
                    <div class="print-info" style="margin-bottom: 20px;">
                        <p>{"客户: "}{order.customer_name.as_deref().unwrap_or("-")}</p>
                        <p>{"订单状态: "}{&order.status}</p>
                        <p>{"创建时间: "}{&order.created_at}</p>
                    </div>
                    <table class="print-table">
                        <thead>
                            <tr>
                                <th>{"商品名称"}</th>
                                <th>{"数量"}</th>
                                <th>{"单价"}</th>
                                <th>{"折扣(%)"}</th>
                                <th>{"总价"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {for items.iter().map(|item| {
                                html! {
                                    <tr>
                                        <td>{item.product_name.as_deref().unwrap_or("-")}</td>
                                        <td>{item.quantity}</td>
                                        <td>{item.unit_price}</td>
                                        <td>{item.discount_percent}</td>
                                        <td>{item.total_amount}</td>
                                    </tr>
                                }
                            })}
                        </tbody>
                    </table>
                    <div style="margin-top: 20px; text-align: right;">
                        <h3>{"总金额: "}{&order.total_amount}</h3>
                    </div>
                </div>
            }
        } else {
            html! {}
        }
    }
}
