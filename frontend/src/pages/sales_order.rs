// 销售订单管理页面

use yew::prelude::*;
use crate::components::permission_guard::PermissionGuard;
use crate::utils::permissions;
use wasm_bindgen_futures::spawn_local;
use crate::models::sales::{SalesOrder, ShipOrderRequest, ShipOrderItemRequest};
use crate::services::sales_service::SalesService;
use crate::services::crud_service::CrudService;
use crate::models::warehouse::Warehouse;
use crate::services::warehouse_service::WarehouseService;
use std::str::FromStr;
use rust_decimal::Decimal;

#[derive(Clone, Debug, PartialEq)]
pub struct ShipItemData {
    pub order_item_id: i32,
    pub product_id: i32,
    pub product_name: String,
    pub quantity: f64,
    pub warehouse_id: Option<i32>,
    pub batch_no: String,
}

pub struct SalesOrderPage {
    orders: Vec<SalesOrder>,
    loading: bool,
    error: Option<String>,
    page: u64,
    page_size: u64,
    filter_status: String,
    printing_order: Option<SalesOrder>,
    print_trigger: bool,
    
    // 发货相关状态
    warehouses: Vec<Warehouse>,
    shipping_order: Option<SalesOrder>,
    ship_items: Vec<ShipItemData>,
    submitting_ship: bool,
    
    // 物流与扫码
    logistics_carrier: String,
    tracking_number: String,
    barcode_input: String,
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
    SubmitShip,
    ShipSuccess,
    ShipError(String),
    
    FastShip(i32),
    UpdateLogisticsCarrier(String),
    UpdateTrackingNumber(String),
    UpdateBarcodeInput(String),
    ProcessBarcode,
    Ignore,
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
            filter_status: String::from("全部"),
            printing_order: None,
            print_trigger: false,
            warehouses: Vec::new(),
            shipping_order: None,
            ship_items: Vec::new(),
            submitting_ship: false,
            
            logistics_carrier: String::new(),
            tracking_number: String::new(),
            barcode_input: String::new(),
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
                #[derive(serde::Serialize)]
                struct SalesOrderQuery {
                    page: Option<u64>,
                    page_size: Option<u64>,
                    status: Option<String>,
                    customer_id: Option<i32>,
                }
                let query = SalesOrderQuery {
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                    status: if self.filter_status == "全部" { None } else { Some(self.filter_status.clone()) },
                    customer_id: None,
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match SalesService::list_with_query(&query).await {
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
                    match SalesService::get(id).await {
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
                    if let Ok(res) = WarehouseService::list().await {
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
                    match SalesService::get(id).await {
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
            Msg::SubmitShip => {
                if let Some(order) = &self.shipping_order {
                    let mut req_items = Vec::new();
                    // 校验并收集数据
                    for item in &self.ship_items {
                        if item.warehouse_id.is_none() {
                            ctx.link().send_message(Msg::ShipError("请选择发货仓库".into()));
                            return false;
                        }
                        if item.batch_no.trim().is_empty() {
                            ctx.link().send_message(Msg::ShipError("请输入批次号".into()));
                            return false;
                        }
                        
                        let quantity_dec = Decimal::from_f64_retain(item.quantity).unwrap_or_default();
                        
                        req_items.push(ShipOrderItemRequest {
                            order_item_id: item.order_item_id,
                            product_id: item.product_id,
                            quantity: quantity_dec,
                            warehouse_id: item.warehouse_id.unwrap_or(0),
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
                if let Some(win) = web_sys::window() { win.alert_with_message(&e).ok(); }
                true
            }
            _ => false,
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
                                        if permissions::has_permission("sales_order", "read") {
                                            <button class="btn-secondary" onclick={ctx.link().callback(move |_| Msg::PreparePrint(id))}>
                                                {"打印"}
                                            </button>
                                        }
                                        if (order.status == "draft" || order.status == "rejected") && permissions::has_permission("sales_order", "update") {
                                            <button class="btn-primary" style="margin-left: 8px;" onclick={ctx.link().callback(move |_| Msg::SubmitOrder(id))}>
                                                {"提交审批"}
                                            </button>
                                        }
                                        if order.status == "approved" && permissions::has_permission("sales_order", "update") {
                                            <button class="btn-primary" style="margin-left: 8px;" onclick={ctx.link().callback(move |_| Msg::PrepareShip(id))}>
                                                {"发货"}
                                            </button>
                                        }
                                    </td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
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
                            <div class="ship-extra-info" style="margin-bottom: 20px; padding: 15px; background: #f8f9fa; border-radius: 4px; border: 1px solid #e5e6eb;">
                                <h4 style="margin-top: 0; margin-bottom: 15px; font-size: 14px; color: #1d2129;">{"发货追踪与扫码 (选填)"}</h4>
                                <div style="display: flex; gap: 15px; margin-bottom: 10px;">
                                    <div style="flex: 1;">
                                        <label style="display: block; margin-bottom: 5px; font-size: 12px; color: #4e5969;">{"物流承运商"}</label>
                                        <input 
                                            type="text" 
                                            class="form-control" 
                                            placeholder="如：顺丰、跨越速运"
                                            value={self.logistics_carrier.clone()}
                                            oninput={ctx.link().batch_callback(|e: InputEvent| {
                                                use wasm_bindgen::JsCast;
                                                use web_sys::HtmlInputElement;
                                                let target = e.target()?;
                                                let input = target.unchecked_into::<HtmlInputElement>();
                                                Some(Msg::UpdateLogisticsCarrier(input.value()))
                                            })}
                                        />
                                    </div>
                                    <div style="flex: 1;">
                                        <label style="display: block; margin-bottom: 5px; font-size: 12px; color: #4e5969;">{"物流运单号"}</label>
                                        <input 
                                            type="text" 
                                            class="form-control" 
                                            placeholder="请扫码或输入运单号"
                                            value={self.tracking_number.clone()}
                                            oninput={ctx.link().batch_callback(|e: InputEvent| {
                                                use wasm_bindgen::JsCast;
                                                use web_sys::HtmlInputElement;
                                                let target = e.target()?;
                                                let input = target.unchecked_into::<HtmlInputElement>();
                                                Some(Msg::UpdateTrackingNumber(input.value()))
                                            })}
                                        />
                                    </div>
                                </div>
                                <div style="display: flex; gap: 15px;">
                                    <div style="flex: 1;">
                                        <label style="display: block; margin-bottom: 5px; font-size: 12px; color: #4e5969;">{"条码枪录入 (布卷条码 -> 批次)"}</label>
                                        <div style="display: flex; gap: 8px;">
                                            <input 
                                                type="text" 
                                                class="form-control" 
                                                placeholder="请用 PDA 扫码枪扫描布卷条码..."
                                                value={self.barcode_input.clone()}
                                                oninput={ctx.link().batch_callback(|e: InputEvent| {
                                                    use wasm_bindgen::JsCast;
                                                    use web_sys::HtmlInputElement;
                                                    let target = e.target()?;
                                                    let input = target.unchecked_into::<HtmlInputElement>();
                                                    Some(Msg::UpdateBarcodeInput(input.value()))
                                                })}
                                                onkeyup={ctx.link().callback(|e: KeyboardEvent| {
                                                    if e.key() == "Enter" {
                                                        Msg::ProcessBarcode
                                                    } else {
                                                        Msg::Ignore
                                                    }
                                                })}
                                            />
                                            <button type="button" class="btn-secondary" onclick={ctx.link().callback(|_| Msg::ProcessBarcode)}>
                                                {"识别"}
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            </div>
                            
                            <table class="data-table">
                                <thead>
                                    <tr>
                                        <th>{"商品名称"}</th>
                                        <th>{"数量"}</th>
                                        <th>{"发货仓库"}</th>
                                        <th>{"批次号"}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {for self.ship_items.iter().enumerate().map(|(idx, item)| {
                                        let on_warehouse_change = ctx.link().batch_callback(move |e: Event| {
                                            use wasm_bindgen::JsCast;
                                            use web_sys::HtmlSelectElement;
                                            let target = e.target()?;
                                            let select = target.unchecked_into::<HtmlSelectElement>();
                                            if let Ok(wid) = select.value().parse::<i32>() {
                                                Some(Msg::UpdateShipItemWarehouse(idx, wid))
                                            } else {
                                                Some(Msg::UpdateShipItemWarehouse(idx, 0))
                                            }
                                        });
                                        
                                        let on_batch_change = ctx.link().batch_callback(move |e: Event| {
                                            use wasm_bindgen::JsCast;
                                            use web_sys::HtmlInputElement;
                                            let target = e.target()?;
                                            let input = target.unchecked_into::<HtmlInputElement>();
                                            Some(Msg::UpdateShipItemBatch(idx, input.value()))
                                        });

                                        html! {
                                            <tr>
                                                <td>{&item.product_name}</td>
                                                <td>{item.quantity}</td>
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
                        <div class="modal-footer">
                            <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseShipModal)}>
                                {"取消"}
                            </button>
                            <PermissionGuard resource="sales_order" action="create">
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
</PermissionGuard>
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
