// 面料订单管理页面

use crate::utils::permissions;
use yew::prelude::*;
use crate::components::permission_guard::PermissionGuard;
use crate::utils::dom_helper;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::models::fabric_order::{
    FabricOrder, FabricOrderQuery,
    CreateFabricOrderRequest, UpdateFabricOrderRequest,
};
use crate::services::fabric_order_service::FabricOrderService;
use crate::services::crud_service::CrudService;

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
                    status: if self.filter_status == "全部" { None } else { Some(self.filter_status.clone()) },
                    batch_no: None,
                    color_no: None,
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match FabricOrderService::list_with_query(&query).await {
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
                let get_val = |id: &str| -> String { dom_helper::get_input_value(id).or_else(|| dom_helper::get_textarea_value(id)).unwrap_or_default() };
                let get_opt = |id: &str| -> Option<String> { let v = get_val(id); if v.is_empty() { None } else { Some(v) } };
                
                if self.modal_mode == ModalMode::Create {
                    let req = CreateFabricOrderRequest {
                        customer_id: get_val("customer-id").parse().unwrap_or(0),
                        order_date: get_val("order-date"),
                        required_date: get_val("required-date"),
                        items: vec![],
                        shipping_address: None,
                        delivery_address: None,
                        payment_terms: None,
                        remarks: get_opt("remarks"),
                        batch_no: get_opt("batch-no"),
                        color_no: get_opt("color-no"),
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
                } else if self.modal_mode == ModalMode::Edit {
                    let id = self.current_order.as_ref().map(|o| o.id).unwrap_or(0);
                    let req = UpdateFabricOrderRequest {
                        required_date: Some(get_val("required-date")),
                        status: None,
                        shipping_address: None,
                        delivery_address: None,
                        payment_terms: None,
                        remarks: get_opt("remarks"),
                        items: None,
                        batch_no: get_opt("batch-no"),
                        color_no: get_opt("color-no"),
                        packaging_requirement: None,
                        quality_standard: None,
                    };
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match FabricOrderService::update(id, req).await {
                            Ok(_) => link.send_message(Msg::LoadOrders),
                            Err(e) => link.send_message(Msg::LoadError(e)),
                        }
                    });
                }
                self.show_modal = false;
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
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_filter_change = ctx.link().batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlSelectElement>().ok()?;
            Some(Msg::SetFilterStatus(target.value()))
        });

        html! {
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
        }
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
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>{"订单编号"}</th>
                            <th>{"客户名称"}</th>
                            <th>{"订单日期"}</th>
                            <th>{"要求交货日期"}</th>
                            <th>{"订单状态"}</th>
                            <th>{"总金额"}</th>
                            <th>{"批次号"}</th>
                            <th>{"色号"}</th>
                            <th>{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.orders.iter().map(|order| {
                            let order_clone = order.clone();
                            let order_clone2 = order.clone();
                            let order_id = order.id;
                            let order_status = order.status.clone();
                            html! {
                                <tr>
                                    <td>{&order.order_no}</td>
                                    <td>{order.customer_name.as_deref().unwrap_or("-")}</td>
                                    <td>{&order.order_date}</td>
                                    <td>{&order.required_date}</td>
                                    <td>
                                        <span class={format!("status-badge status-{}", self.get_status_class(&order.status))}>
                                            {&order.status}
                                        </span>
                                    </td>
                                    <td class="numeric">{&order.total_amount}</td>
                                    <td>{order.batch_no.as_deref().unwrap_or("-")}</td>
                                    <td>{order.color_no.as_deref().unwrap_or("-")}</td>
                                    <td>
                                        <div class="action-buttons">
                                            <button class="btn-sm btn-info" onclick={ctx.link().callback(move |_| Msg::OpenModal(ModalMode::View, Some(order_clone.clone())))}>
                                                {"查看"}
                                            </button>
                                                                                        <button class="btn-sm btn-primary" onclick={ctx.link().callback(move |_| Msg::OpenModal(ModalMode::Edit, Some(order_clone2.clone())))}>
                                                {"编辑"}
                                            </button>
                                            {if order_status == "待审批" {
                                                html! {
                                                    <PermissionGuard resource="fabric_order" action="approve">
<button class="btn-sm btn-success" onclick={ctx.link().callback(move |_| Msg::ApproveOrder(order_id))}>
                                                        {"审批"}
                                                    </button>
</PermissionGuard>
                                                }
                                            } else {
                                                html! {}
                                            }}
                                            <PermissionGuard resource="fabric_order" action="delete">
<button class="btn-sm btn-danger" onclick={ctx.link().callback(move |_| Msg::DeleteOrder(order_id))}>
                                                {"删除"}
                                            </button>
</PermissionGuard>
                                        </div>
                                    </td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
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

    fn render_modal(&self, ctx: &Context<Self>) -> Html {
        if !self.show_modal {
            return html! {};
        }
        
        let title = match self.modal_mode {
            ModalMode::Create => "新建面料订单",
            ModalMode::Edit => "编辑面料订单",
            ModalMode::View => "查看面料订单",
        };

        html! {
            <div class="modal-overlay">
                <div class="modal-content">
                    <div class="modal-header">
                        <h3>{title}</h3>
                        <button class="close-btn" onclick={ctx.link().callback(|_| Msg::CloseModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                                                {if self.modal_mode == ModalMode::View {
                            if let Some(item) = &self.current_order {
                                html! {
                                    <div class="detail-grid" style="display: grid; grid-template-columns: 1fr 1fr; gap: 1rem;">
                                        <div class="detail-item">
                                            <span class="detail-label" style="font-weight: bold; color: #666;">{"Id: "}</span>
                                            <span class="detail-value">{item.id.to_string()}</span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label" style="font-weight: bold; color: #666;">{"Order No: "}</span>
                                            <span class="detail-value">{&item.order_no}</span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label" style="font-weight: bold; color: #666;">{"Customer Id: "}</span>
                                            <span class="detail-value">{item.customer_id.to_string()}</span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label" style="font-weight: bold; color: #666;">{"Customer Name: "}</span>
                                            <span class="detail-value">{item.customer_name.as_deref().unwrap_or("-")}</span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label" style="font-weight: bold; color: #666;">{"Order Date: "}</span>
                                            <span class="detail-value">{&item.order_date}</span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label" style="font-weight: bold; color: #666;">{"Required Date: "}</span>
                                            <span class="detail-value">{&item.required_date}</span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label" style="font-weight: bold; color: #666;">{"Status: "}</span>
                                            <span class="detail-value">{&item.status}</span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label" style="font-weight: bold; color: #666;">{"Total Amount: "}</span>
                                            <span class="detail-value">{&item.total_amount}</span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label" style="font-weight: bold; color: #666;">{"Paid Amount: "}</span>
                                            <span class="detail-value">{&item.paid_amount}</span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label" style="font-weight: bold; color: #666;">{"Shipping Address: "}</span>
                                            <span class="detail-value">{item.shipping_address.as_deref().unwrap_or("-")}</span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label" style="font-weight: bold; color: #666;">{"Delivery Address: "}</span>
                                            <span class="detail-value">{item.delivery_address.as_deref().unwrap_or("-")}</span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label" style="font-weight: bold; color: #666;">{"Payment Terms: "}</span>
                                            <span class="detail-value">{item.payment_terms.as_deref().unwrap_or("-")}</span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label" style="font-weight: bold; color: #666;">{"Remarks: "}</span>
                                            <span class="detail-value">{item.remarks.as_deref().unwrap_or("-")}</span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label" style="font-weight: bold; color: #666;">{"Batch No: "}</span>
                                            <span class="detail-value">{item.batch_no.as_deref().unwrap_or("-")}</span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label" style="font-weight: bold; color: #666;">{"Color No: "}</span>
                                            <span class="detail-value">{item.color_no.as_deref().unwrap_or("-")}</span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label" style="font-weight: bold; color: #666;">{"Dye Lot No: "}</span>
                                            <span class="detail-value">{item.dye_lot_no.as_deref().unwrap_or("-")}</span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label" style="font-weight: bold; color: #666;">{"Grade: "}</span>
                                            <span class="detail-value">{item.grade.as_deref().unwrap_or("-")}</span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label" style="font-weight: bold; color: #666;">{"Packaging Requirement: "}</span>
                                            <span class="detail-value">{item.packaging_requirement.as_deref().unwrap_or("-")}</span>
                                        </div>
                                        <div class="detail-item">
                                            <span class="detail-label" style="font-weight: bold; color: #666;">{"Quality Standard: "}</span>
                                            <span class="detail-value">{item.quality_standard.as_deref().unwrap_or("-")}</span>
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
                                }
                            } else {
                                html! { <p>{"No data"}</p> }
                            }
                                                } else {
                            let is_edit = self.modal_mode == ModalMode::Edit;
                            let order = self.current_order.clone().unwrap_or_else(|| FabricOrder {
                                id: 0,
                                order_no: String::new(),
                                customer_id: 0,
                                customer_name: None,
                                order_date: String::new(),
                                required_date: String::new(),
                                status: "待审批".to_string(),
                                total_amount: "0".to_string(),
                                paid_amount: "0".to_string(),
                                shipping_address: None,
                                delivery_address: None,
                                payment_terms: None,
                                remarks: None,
                                batch_no: None,
                                color_no: None,
                                dye_lot_no: None,
                                grade: None,
                                packaging_requirement: None,
                                quality_standard: None,
                                created_at: String::new(),
                                updated_at: String::new(),
                            });
                            
                            html! {
                                <div>
                                    <div class="form-group">
                                        <label>{"客户 ID"}</label>
                                        <input type="number" id="customer-id" value={order.customer_id.to_string()} />
                                    </div>
                                    <div class="form-group">
                                        <label>{"订单日期"}</label>
                                        <input type="date" id="order-date" value={order.order_date.clone()} />
                                    </div>
                                    <div class="form-group">
                                        <label>{"要求交货日期"}</label>
                                        <input type="date" id="required-date" value={order.required_date.clone()} />
                                    </div>
                                    <div class="form-group">
                                        <label>{"批次号"}</label>
                                        <input type="text" id="batch-no" value={order.batch_no.clone().unwrap_or_default()} />
                                    </div>
                                    <div class="form-group">
                                        <label>{"色号"}</label>
                                        <input type="text" id="color-no" value={order.color_no.clone().unwrap_or_default()} />
                                    </div>
                                    <div class="form-group">
                                        <label>{"备注"}</label>
                                        <textarea id="remarks" value={order.remarks.clone().unwrap_or_default()}></textarea>
                                    </div>
                                </div>
                            }
                        }}
                    </div>
                    <div class="modal-footer">
                        <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseModal)}>{"关闭"}</button>
                        {if self.modal_mode == ModalMode::Create {
                            html! {
                                <PermissionGuard resource="fabric_order" action="create">
<button class="btn-primary" onclick={ctx.link().callback(|_| Msg::CreateOrder)}>{"保存"}</button>
</PermissionGuard>
                            }
                        } else {
                            html! {}
                        }}
                    </div>
                </div>
            </div>
        }
    }
}