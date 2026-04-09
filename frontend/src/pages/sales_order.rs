//! 销售订单管理页面

use crate::components::main_layout::MainLayout;
use crate::models::sales::{SalesOrder, ShipOrderItemRequest, ShipOrderRequest};
use crate::models::warehouse::Warehouse;
use crate::services::sales_service::SalesService;
use crate::services::warehouse_service::WarehouseService;
use rust_decimal::Decimal;
use std::str::FromStr;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct ShipItemData {
    pub order_item_id: i32,
    pub product_id: i32,
    pub product_name: String,
    pub quantity: f64,
    pub warehouse_id: Option<i32>,
    pub batch_no: String,
    // 前端附加字段
    pub rolls: String,
    pub total_meters: String,
    pub weight_width: String,
    pub color_code: String,
}

pub struct SalesOrderPage {
    orders: Vec<SalesOrder>,
    loading: bool,
    error: Option<String>,
    page: u64,
    page_size: u64,
    printing_order: Option<SalesOrder>,
    print_trigger: bool,

    // 发货相关状态
    warehouses: Vec<Warehouse>,
    shipping_order: Option<SalesOrder>,
    ship_items: Vec<ShipItemData>,
    submitting_ship: bool,
}

pub enum Msg {
    LoadOrders,
    OrdersLoaded(Vec<SalesOrder>),
    LoadError(String),
    PreparePrint(i32),
    PrintReady(SalesOrder),

    // 发货相关消息
    LoadWarehouses,
    WarehousesLoaded(Vec<Warehouse>),
    PrepareShip(i32),
    ShipReady(SalesOrder),
    CloseShipModal,
    SubmitOrder(i32),
    UpdateShipItemWarehouse(usize, i32),
    UpdateShipItemBatch(usize, String),
    UpdateShipItemRolls(usize, String),
    UpdateShipItemTotalMers(usize, String),
    UpdateShipItemWeightWidth(usize, String),
    UpdateShipItemColorCode(usize, String),
    SubmitShip,
    ShipSuccess,
    ShipError(String),
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
            warehouses: Vec::new(),
            shipping_order: None,
            ship_items: Vec::new(),
            submitting_ship: false,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadOrders);
            ctx.link().send_message(Msg::LoadWarehouses);
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
            Msg::LoadWarehouses => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    if let Ok(res) = WarehouseService::list_warehouses().await {
                        link.send_message(Msg::WarehousesLoaded(res.warehouses));
                    }
                });
                false
            }
            Msg::WarehousesLoaded(warehouses) => {
                self.warehouses = warehouses;
                true
            }
            Msg::PrepareShip(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match SalesService::get_order(id).await {
                        Ok(order) => {
                            link.send_message(Msg::ShipReady(order));
                        }
                        Err(e) => {
                            link.send_message(Msg::ShipError(format!("加载订单数据失败: {}", e)));
                        }
                    }
                });
                false
            }
            Msg::ShipReady(order) => {
                let mut items = Vec::new();
                if let Some(order_items) = &order.items {
                    for item in order_items {
                        items.push(ShipItemData {
                            order_item_id: item.id,
                            product_id: item.product_id,
                            product_name: item.product_name.clone().unwrap_or_default(),
                            quantity: item.quantity,
                            warehouse_id: None,
                            batch_no: String::new(),
                            rolls: String::new(),
                            total_meters: String::new(),
                            weight_width: String::new(),
                            color_code: String::new(),
                        });
                    }
                }
                self.ship_items = items;
                self.shipping_order = Some(order);
                true
            }

            Msg::SubmitOrder(id) => {
                let link = ctx.link().clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = crate::services::sales_service::SalesService::submit_order(id).await;
                    link.send_message(Msg::LoadOrders);
                });
                true
            }
            Msg::CloseShipModal => {
                self.shipping_order = None;
                self.ship_items.clear();
                self.submitting_ship = false;
                true
            }
            Msg::UpdateShipItemWarehouse(idx, warehouse_id) => {
                if let Some(item) = self.ship_items.get_mut(idx) {
                    if warehouse_id > 0 {
                        item.warehouse_id = Some(warehouse_id);
                    } else {
                        item.warehouse_id = None;
                    }
                }
                true
            }
            Msg::UpdateShipItemBatch(idx, batch_no) => {
                if let Some(item) = self.ship_items.get_mut(idx) {
                    item.batch_no = batch_no;
                }
                true
            }
            Msg::UpdateShipItemRolls(idx, rolls) => {
                if let Some(item) = self.ship_items.get_mut(idx) {
                    item.rolls = rolls;
                }
                true
            }
            Msg::UpdateShipItemTotalMers(idx, total_meters) => {
                if let Some(item) = self.ship_items.get_mut(idx) {
                    item.total_meters = total_meters;
                }
                true
            }
            Msg::UpdateShipItemWeightWidth(idx, weight_width) => {
                if let Some(item) = self.ship_items.get_mut(idx) {
                    item.weight_width = weight_width;
                }
                true
            }
            Msg::UpdateShipItemColorCode(idx, color_code) => {
                if let Some(item) = self.ship_items.get_mut(idx) {
                    item.color_code = color_code;
                }
                true
            }
            Msg::SubmitShip => {
                if let Some(order) = &self.shipping_order {
                    let mut req_items = Vec::new();
                    // 校验并收集数据
                    for item in &self.ship_items {
                        if item.warehouse_id.is_none() {
                            ctx.link()
                                .send_message(Msg::ShipError("请选择发货仓库".into()));
                            return false;
                        }
                        if item.batch_no.trim().is_empty() {
                            ctx.link()
                                .send_message(Msg::ShipError("请输入批次号".into()));
                            return false;
                        }

                        let quantity_dec =
                            Decimal::from_f64_retain(item.quantity).unwrap_or_default();

                        req_items.push(ShipOrderItemRequest {
                            order_item_id: item.order_item_id,
                            product_id: item.product_id,
                            quantity: quantity_dec,
                            warehouse_id: item.warehouse_id.unwrap(),
                            batch_no: item.batch_no.clone(),
                        });
                    }

                    self.submitting_ship = true;
                    let order_id = order.id;
                    let req = ShipOrderRequest { items: req_items };
                    let link = ctx.link().clone();

                    spawn_local(async move {
                        match SalesService::ship_order(order_id, req).await {
                            Ok(_) => link.send_message(Msg::ShipSuccess),
                            Err(e) => link.send_message(Msg::ShipError(e)),
                        }
                    });
                    return true;
                }
                false
            }
            Msg::ShipSuccess => {
                self.shipping_order = None;
                self.submitting_ship = false;
                ctx.link().send_message(Msg::LoadOrders);
                true
            }
            Msg::ShipError(e) => {
                self.submitting_ship = false;
                web_sys::window().unwrap().alert_with_message(&e).unwrap();
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <MainLayout current_page={"sales_order"}>
<div class="sales-order-page">
                <div class="page-header">
                    <h1>{"📦 销售订单管理"}</h1>
                </div>

                {self.render_content(ctx)}
            </div>
        
</MainLayout>}
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
                <div class="overflow-x-auto w-full pb-4">
<table class="data-table w-full">
                    <thead>
                        <tr>
                            <th>{"订单号"}</th>
                            <th>{"色卡编号"}</th>
                            <th>{"花型"}</th>
                            <th>{"客户"}</th>
                            <th class="numeric-cell text-right">{"总金额"}</th>
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
                                    <td>{"-"}</td>
                                    <td>{"🎨"}</td>
                                    <td>{order.customer_name.as_deref().unwrap_or("-")}</td>
                                    <td class="numeric-cell text-right">{&order.total_amount}</td>
                                    <td><span class="status-badge">{&order.status}</span></td>
                                    <td>{&order.created_at}</td>
                                    <td>
                                        <button class="btn-secondary" onclick={ctx.link().callback(move |_| Msg::PreparePrint(id))}>
                                            {"打印"}
                                        </button>
                                        <button class="btn-primary" style="margin-left: 8px;" onclick={ctx.link().callback(move |_| Msg::PrepareShip(id))}>
                                            {"发货"}
                                        </button>
                                    </td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
</div>
            </div>
            {self.render_print_view()}
            {self.render_ship_modal(ctx)}
            </>
        }
    }

    fn render_ship_modal(&self, ctx: &Context<Self>) -> Html {
        if let Some(order) = &self.shipping_order {
            html! {
                <div class="modal-overlay">
                    <div class="modal-content" style="width: 800px; max-width: 90vw;">
                        <div class="modal-header">
                            <h2>{"订单发货 - "}{&order.order_no}</h2>
                            <button class="close-btn" onclick={ctx.link().callback(|_| Msg::CloseShipModal)}>{"×"}</button>
                        </div>
                        <div class="modal-body">
                            <div class="overflow-x-auto w-full pb-4">
<table class="data-table w-full">
                                <thead>
                                    <tr>
                                        <th>{"商品名称"}</th>
                                        <th>{"色卡编号"}</th>
                                        <th>{"花型"}</th>
                                        <th class="numeric-cell text-right">{"数量"}</th>
                                        <th>{"匹数(rolls)"}</th>
                                        <th>{"总米数"}</th>
                                        <th>{"克重/门幅"}</th>
                                        <th>{"预估重量(公斤)"}</th>
                                        <th>{"发货仓库"}</th>
                                        <th>{"批次号"}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {for self.ship_items.iter().enumerate().map(|(idx, item)| {
                                        let on_warehouse_change = ctx.link().callback(move |e: Event| {
                                            use wasm_bindgen::JsCast;
                                            use web_sys::HtmlSelectElement;
                                            let target = e.target().unwrap();
                                            let select = target.unchecked_into::<HtmlSelectElement>();
                                            if let Ok(wid) = select.value().parse::<i32>() {
                                                Msg::UpdateShipItemWarehouse(idx, wid)
                                            } else {
                                                Msg::UpdateShipItemWarehouse(idx, 0)
                                            }
                                        });

                                        let on_batch_change = ctx.link().callback(move |e: Event| {
                                            use wasm_bindgen::JsCast;
                                            use web_sys::HtmlInputElement;
                                            let target = e.target().unwrap();
                                            let input = target.unchecked_into::<HtmlInputElement>();
                                            Msg::UpdateShipItemBatch(idx, input.value())
                                        });

                                        let on_rolls_change = ctx.link().callback(move |e: Event| {
                                            use wasm_bindgen::JsCast;
                                            use web_sys::HtmlInputElement;
                                            let target = e.target().unwrap();
                                            let input = target.unchecked_into::<HtmlInputElement>();
                                            Msg::UpdateShipItemRolls(idx, input.value())
                                        });

                                        let on_meters_change = ctx.link().callback(move |e: Event| {
                                            use wasm_bindgen::JsCast;
                                            use web_sys::HtmlInputElement;
                                            let target = e.target().unwrap();
                                            let input = target.unchecked_into::<HtmlInputElement>();
                                            Msg::UpdateShipItemTotalMers(idx, input.value())
                                        });

                                        let on_weight_width_change = ctx.link().callback(move |e: Event| {
                                            use wasm_bindgen::JsCast;
                                            use web_sys::HtmlInputElement;
                                            let target = e.target().unwrap();
                                            let input = target.unchecked_into::<HtmlInputElement>();
                                            Msg::UpdateShipItemWeightWidth(idx, input.value())
                                        });

                                        let on_color_change = ctx.link().callback(move |e: Event| {
                                            use wasm_bindgen::JsCast;
                                            use web_sys::HtmlInputElement;
                                            let target = e.target().unwrap();
                                            let input = target.unchecked_into::<HtmlInputElement>();
                                            Msg::UpdateShipItemColorCode(idx, input.value())
                                        });

                                        // Calculate estimated weight
                                        let estimated_weight = if let (Ok(meters), Some((weight_str, width_str))) = (
                                            item.total_meters.parse::<f64>(),
                                            item.weight_width.split_once('/')
                                        ) {
                                            if let (Ok(weight), Ok(width)) = (weight_str.parse::<f64>(), width_str.parse::<f64>()) {
                                                // 重量(kg) = (克重(g/m2) * 门幅(m) * 米数) / 1000
                                                // 假设门幅单位是cm，如果是m不需要除以100。这里假设克重是g/m2，门幅是cm
                                                // 重量 = 米数 * 克重 * (门幅 / 100) / 1000 = (米数 * 克重 * 门幅) / 100000
                                                (meters * weight * width) / 100000.0
                                            } else {
                                                0.0
                                            }
                                        } else {
                                            0.0
                                        };

                                        html! {
                                            <tr>
                                                <td>{&item.product_name}</td>
                                                <td>
                                                    <input
                                                        type="text"
                                                        class="form-control"
                                                        style="width: 80px"
                                                        value={item.color_code.clone()}
                                                        onchange={on_color_change}
                                                        placeholder="色卡"
                                                    />
                                                </td>
                                                <td>{"🎨"}</td>
                                                <td class="numeric-cell text-right">{item.quantity}</td>
                                                <td>
                                                    <input
                                                        type="number"
                                                        class="form-control"
                                                        style="width: 80px"
                                                        value={item.rolls.clone()}
                                                        onchange={on_rolls_change}
                                                        placeholder="匹数"
                                                    />
                                                </td>
                                                <td>
                                                    <input
                                                        type="number"
                                                        class="form-control"
                                                        style="width: 80px"
                                                        value={item.total_meters.clone()}
                                                        onchange={on_meters_change}
                                                        placeholder="总米数"
                                                    />
                                                </td>
                                                <td>
                                                    <input
                                                        type="text"
                                                        class="form-control"
                                                        style="width: 80px"
                                                        value={item.weight_width.clone()}
                                                        onchange={on_weight_width_change}
                                                        placeholder="200/150"
                                                    />
                                                </td>
                                                <td>
                                                    {if estimated_weight > 0.0 {
                                                        format!("{:.2} kg", estimated_weight)
                                                    } else {
                                                        "-".to_string()
                                                    }}
                                                </td>
                                                <td>
                                                    <select
                                                        class="form-control"
                                                        onchange={on_warehouse_change}
                                                        value={item.warehouse_id.map(|id| id.to_string()).unwrap_or_default()}
                                                    >
                                                        <option value="">{"请选择仓库"}</option>
                                                        {for self.warehouses.iter().map(|w| {
                                                            html! { <option value={w.id.to_string()}>{&w.name}</option> }
                                                        })}
                                                    </select>
                                                </td>
                                                <td>
                                                    <input
                                                        type="text"
                                                        class="form-control"
                                                        value={item.batch_no.clone()}
                                                        onchange={on_batch_change}
                                                        placeholder="请输入批次号"
                                                    />
                                                </td>
                                            </tr>
                                        }
                                    })}
                                </tbody>
                            </table>
</div>
                        </div>
                        <div class="modal-footer">
                            <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseShipModal)}>
                                {"取消"}
                            </button>
                            <button
                                class="btn-primary"
                                onclick={ctx.link().callback(|_| Msg::SubmitShip)}
                                disabled={self.submitting_ship}
                            >
                                if self.submitting_ship {
                                    {"提交中..."}
                                } else {
                                    {"确认发货"}
                                }
                            </button>
                        </div>
                    </div>
                </div>
            }
        } else {
            html! {}
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
                        <p>{"订单状态: "}<span class="status-badge">{&order.status}</span></p>
                        <p>{"创建时间: "}{&order.created_at}</p>
                    </div>
                    <div class="overflow-x-auto w-full pb-4">
<table class="data-table w-full print-table">
                        <thead>
                            <tr>
                                <th>{"商品名称"}</th>
                                <th>{"色卡编号"}</th>
                                <th>{"花型"}</th>
                                <th class="numeric-cell text-right">{"数量"}</th>
                                <th class="numeric-cell text-right">{"单价"}</th>
                                <th class="numeric-cell text-right">{"折扣(%)"}</th>
                                <th class="numeric-cell text-right">{"总价"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {for items.iter().map(|item| {
                                html! {
                                    <tr>
                                        <td>{item.product_name.as_deref().unwrap_or("-")}</td>
                                        <td>{"-"}</td>
                                        <td>{"🎨"}</td>
                                        <td class="numeric-cell text-right">{item.quantity}</td>
                                        <td class="numeric-cell text-right">{item.unit_price}</td>
                                        <td class="numeric-cell text-right">{item.discount_percent}</td>
                                        <td class="numeric-cell text-right">{item.total_amount}</td>
                                    </tr>
                                }
                            })}
                        </tbody>
                    </table>
</div>
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
