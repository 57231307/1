//! 面料订单管理页面

use crate::components::main_layout::MainLayout;
use crate::models::fabric_order::{
    CreateFabricOrderRequest, FabricOrder, FabricOrderQuery, UpdateFabricOrderRequest,
};
use crate::services::fabric_order_service::FabricOrderService;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use web_sys::window;

pub struct FabricOrderPage {
    orders: Vec<FabricOrder>,
    loading: bool,
    error: Option<String>,
    show_modal: bool,
    modal_mode: ModalMode,
    current_order: Option<FabricOrder>,
    filter_status: String,
    page: u64,
    page_size: u64,
    // 前端附加字段
    new_rolls: String,
    new_total_meters: String,
    new_weight_width: String,
    new_color_code: String,
}

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    View,
    Create,
    Edit,
}

pub enum Msg {
    LoadOrders,
    OrdersLoaded(Vec<FabricOrder>),
    LoadError(String),
    SetFilterStatus(String),
    OpenModal(ModalMode, Option<FabricOrder>),
    CloseModal,
    CreateOrder,
    UpdateOrder(i32, UpdateFabricOrderRequest),
    DeleteOrder(i32),
    ApproveOrder(i32),
    ChangePage(u64),
    UpdateNewRolls(String),
    UpdateNewTotalMeters(String),
    UpdateNewWeightWidth(String),
    UpdateNewColorCode(String),
}

impl Component for FabricOrderPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            orders: Vec::new(),
            loading: true,
            error: None,
            show_modal: false,
            modal_mode: ModalMode::View,
            current_order: None,
            filter_status: String::from("全部"),
            page: 1,
            page_size: 20,
            new_rolls: String::new(),
            new_total_meters: String::new(),
            new_weight_width: String::new(),
            new_color_code: String::new(),
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
                let query = FabricOrderQuery {
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                    customer_id: None,
                    order_no: None,
                    status: if self.filter_status == "全部" {
                        None
                    } else {
                        Some(self.filter_status.clone())
                    },
                    batch_no: None,
                    color_no: None,
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match FabricOrderService::list(query).await {
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
            Msg::OpenModal(mode, order) => {
                self.modal_mode = mode;
                self.current_order = order;
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.current_order = None;
                true
            }
            Msg::CreateOrder => {
                let req = CreateFabricOrderRequest {
                    customer_id: 1,
                    order_date: chrono::Utc::now().to_rfc3339(),
                    required_date: chrono::Utc::now().to_rfc3339(),
                    items: vec![],
                    shipping_address: None,
                    delivery_address: None,
                    payment_terms: None,
                    remarks: None,
                    batch_no: None,
                    color_no: if self.new_color_code.is_empty() { None } else { Some(self.new_color_code.clone()) },
                    dye_lot_no: None,
                    grade: None,
                    packaging_requirement: None,
                    quality_standard: None,
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match FabricOrderService::create(req).await {
                        Ok(_) => link.send_message(Msg::LoadOrders),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                self.show_modal = false;
                self.new_rolls.clear();
                self.new_total_meters.clear();
                self.new_weight_width.clear();
                self.new_color_code.clear();
                false
            }
            Msg::UpdateOrder(id, req) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match FabricOrderService::update(id, req).await {
                        Ok(_) => link.send_message(Msg::LoadOrders),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                self.show_modal = false;
                false
            }
            Msg::DeleteOrder(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match FabricOrderService::delete(id).await {
                        Ok(_) => link.send_message(Msg::LoadOrders),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ApproveOrder(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match FabricOrderService::approve(id).await {
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
            Msg::UpdateNewRolls(val) => {
                self.new_rolls = val;
                true
            }
            Msg::UpdateNewTotalMeters(val) => {
                self.new_total_meters = val;
                true
            }
            Msg::UpdateNewWeightWidth(val) => {
                self.new_weight_width = val;
                true
            }
            Msg::UpdateNewColorCode(val) => {
                self.new_color_code = val;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_filter_change = ctx.link().callback(|e: Event| {
            let target = e
                .target()
                .unwrap()
                .dyn_into::<web_sys::HtmlSelectElement>()
                .unwrap();
            Msg::SetFilterStatus(target.value())
        });

        html! {
            <MainLayout current_page={"fabric_order"}>
<div class="fabric-order-page">
                <div class="page-header">
                    <h1>{"📋 面料订单管理"}</h1>
                    <div class="header-actions">
                        <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::OpenModal(ModalMode::Create, None))}>
                            {"新建订单"}
                        </button>
                    </div>
                </div>

                <div class="filter-bar">
                    <div class="filter-item">
                        <label>{"订单状态："}</label>
                        <select value={self.filter_status.clone()} onchange={on_filter_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="待审批">{"待审批"}</option>
                            <option value="已审批">{"已审批"}</option>
                            <option value="已完成">{"已完成"}</option>
                            <option value="已取消">{"已取消"}</option>
                        </select>
                    </div>
                </div>
                {self.render_content(ctx)}
                {self.render_modal(ctx)}
            </div>
        
</MainLayout>}
    }
}

impl FabricOrderPage {
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
                    <div class="empty-icon">{"📋"}</div>
                    <p>{"暂无面料订单"}</p>
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::OpenModal(ModalMode::Create, None))}>
                        {"创建第一个订单"}
                    </button>
                </div>
            };
        }

        html! {
            <div class="table-responsive">
                <div class="overflow-x-auto w-full pb-4">
<table class="data-table w-full">
                    <thead>
                        <tr>
                            <th>{"订单编号"}</th>
                            <th>{"色卡编号"}</th>
                            <th>{"花型"}</th>
                            <th>{"客户名称"}</th>
                            <th>{"订单日期"}</th>
                            <th>{"要求交货日期"}</th>
                            <th>{"订单状态"}</th>
                            <th class="numeric-cell text-right">{"总金额"}</th>
                            <th>{"批次号"}</th>
                            <th>{"色号"}</th>
                            <th>{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.orders.iter().map(|order| {
                            let order_clone = order.clone();
                            let order_id = order.id;
                            let order_status = order.status.clone();
                            html! {
                                <tr>
                                    <td>{&order.order_no}</td>
                                    <td>{"-"}</td>
                                    <td>{"🎨"}</td>
                                    <td>{order.customer_name.as_deref().unwrap_or("-")}</td>
                                    <td>{&order.order_date}</td>
                                    <td>{&order.required_date}</td>
                                    <td>
                                        <span class={format!("status-badge status-{}", self.get_status_class(&order.status))}>
                                            {&order.status}
                                        </span>
                                    </td>
                                    <td class="numeric-cell text-right">{&order.total_amount}</td>
                                    <td>{order.batch_no.as_deref().unwrap_or("-")}</td>
                                    <td>{order.color_no.as_deref().unwrap_or("-")}</td>
                                    <td>
                                        <div class="action-buttons">
                                            <button class="btn-sm btn-info" onclick={ctx.link().callback(move |_| Msg::OpenModal(ModalMode::View, Some(order_clone.clone())))}>
                                                {"查看"}
                                            </button>
                                            {if order_status == "待审批" {
                                                html! {
                                                    <button class="btn-sm btn-success" onclick={ctx.link().callback(move |_| Msg::ApproveOrder(order_id))}>
                                                        {"审批"}
                                                    </button>
                                                }
                                            } else {
                                                html! {}
                                            }}
                                            <button class="btn-sm btn-danger" onclick={ctx.link().callback(move |_| Msg::DeleteOrder(order_id))}>
                                                {"删除"}
                                            </button>
                                        </div>
                                    </td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
</div>
            </div>
        }
    }

    fn render_modal(&self, ctx: &Context<Self>) -> Html {
        if !self.show_modal {
            return html! {};
        }
        let is_create = self.modal_mode == ModalMode::Create;
        let title = match self.modal_mode {
            ModalMode::Create => "新建订单",
            ModalMode::Edit => "编辑订单",
            ModalMode::View => "查看订单",
        };

        let on_rolls_change = ctx.link().callback(move |e: Event| {
            use wasm_bindgen::JsCast;
            let target = e.target().unwrap();
            let input = target.unchecked_into::<web_sys::HtmlInputElement>();
            Msg::UpdateNewRolls(input.value())
        });
        let on_meters_change = ctx.link().callback(move |e: Event| {
            use wasm_bindgen::JsCast;
            let target = e.target().unwrap();
            let input = target.unchecked_into::<web_sys::HtmlInputElement>();
            Msg::UpdateNewTotalMeters(input.value())
        });
        let on_weight_width_change = ctx.link().callback(move |e: Event| {
            use wasm_bindgen::JsCast;
            let target = e.target().unwrap();
            let input = target.unchecked_into::<web_sys::HtmlInputElement>();
            Msg::UpdateNewWeightWidth(input.value())
        });
        let on_color_change = ctx.link().callback(move |e: Event| {
            use wasm_bindgen::JsCast;
            let target = e.target().unwrap();
            let input = target.unchecked_into::<web_sys::HtmlInputElement>();
            Msg::UpdateNewColorCode(input.value())
        });

        // Calculate estimated weight
        let estimated_weight = if let (Ok(meters), Some((weight_str, width_str))) = (
            self.new_total_meters.parse::<f64>(),
            self.new_weight_width.split_once('/')
        ) {
            if let (Ok(weight), Ok(width)) = (weight_str.parse::<f64>(), width_str.parse::<f64>()) {
                (meters * weight * width) / 100000.0
            } else {
                0.0
            }
        } else {
            0.0
        };

        html! {
            <div class="modal-overlay">
                <div class="modal-content" style="width: 600px;">
                    <div class="modal-header">
                        <h2>{title}</h2>
                        <button class="close-btn" onclick={ctx.link().callback(|_| Msg::CloseModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        // Existing form fields would go here...
                        <div class="form-group">
                            <label>{"匹数(rolls)"}</label>
                            <input type="number" class="form-control" value={self.new_rolls.clone()} onchange={on_rolls_change} placeholder="请输入匹数" disabled={!is_create} />
                        </div>
                        <div class="form-group">
                            <label>{"总米数"}</label>
                            <input type="number" class="form-control" value={self.new_total_meters.clone()} onchange={on_meters_change} placeholder="请输入总米数" disabled={!is_create} />
                        </div>
                        <div class="form-group">
                            <label>{"克重/门幅"}</label>
                            <input type="text" class="form-control" value={self.new_weight_width.clone()} onchange={on_weight_width_change} placeholder="例如: 200/150" disabled={!is_create} />
                        </div>
                        <div class="form-group">
                            <label>{"预估重量(公斤)"}</label>
                            <div class="form-control" style="background: #f5f5f5;">
                                {if estimated_weight > 0.0 {
                                    format!("{:.2} kg", estimated_weight)
                                } else {
                                    "-".to_string()
                                }}
                            </div>
                        </div>
                        <div class="form-group">
                            <label>{"色卡编号"}</label>
                            <input type="text" class="form-control" value={self.new_color_code.clone()} onchange={on_color_change} placeholder="请输入色卡编号" disabled={!is_create} />
                        </div>
                        <div class="form-group">
                            <label>{"花型"}</label>
                            <div class="form-control" style="background: #f5f5f5;">
                                {"🎨"}
                            </div>
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseModal)}>{"取消"}</button>
                        {if is_create {
                            html! {
                                <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::CreateOrder)}>{"提交"}</button>
                            }
                        } else {
                            html! {}
                        }}
                    </div>
                </div>
            </div>
        }
    }

    fn get_status_class(&self, status: &str) -> &str {
        match status {
            "待审批" => "warning",
            "已审批" => "info",
            "已完成" => "success",
            "已取消" => "danger",
            _ => "default",
        }
    }
}
