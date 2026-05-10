// 销售订单管理页面

use crate::utils::permissions;
use crate::utils::toast_helper;
use yew::prelude::*;
use crate::components::permission_guard::PermissionGuard;
use crate::components::{
    confirm_dialog::ConfirmDialog,
    search_bar::SearchBar,
    pagination::Pagination,
    page_header::PageHeader,
    empty_state::EmptyState,
    loading_state::LoadingState,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use crate::services::sales_service::SalesService;
use crate::services::crud_service::CrudService;
use crate::models::sales::{
    SalesOrder, CreateSalesOrderRequest, UpdateSalesOrderRequest, SalesOrderItemRequest,
    ShipOrderRequest, ShipOrderItemRequest,
};
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
    filtered_orders: Vec<SalesOrder>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_order: Option<SalesOrder>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
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

    // 表单字段
    form_customer_id: String,
    form_required_date: String,
    form_shipping_address: String,
    form_billing_address: String,
    form_notes: String,
    form_payment_terms: String,
    form_error: Option<String>,
}

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    Create,
    Edit,
}

pub enum Msg {
    LoadData,
    DataLoaded(Vec<SalesOrder>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    OpenEditModal(SalesOrder),
    CloseModal,
    SubmitForm,
    FormSubmitted,
    DeleteOrder(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    SetFilterStatus(String),

    // 表单字段变更
    FormCustomerIdChanged(String),
    FormRequiredDateChanged(String),
    FormShippingAddressChanged(String),
    FormBillingAddressChanged(String),
    FormNotesChanged(String),
    FormPaymentTermsChanged(String),

    // 原有发货相关消息
    PreparePrint(i32),
    PrintReady(SalesOrder),
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
            filtered_orders: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            show_modal: false,
            modal_mode: ModalMode::Create,
            editing_order: None,
            show_delete_confirm: false,
            deleting_id: None,
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
            form_customer_id: String::new(),
            form_required_date: String::new(),
            form_shipping_address: String::new(),
            form_billing_address: String::new(),
            form_notes: String::new(),
            form_payment_terms: String::new(),
            form_error: None,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadData);
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
            Msg::LoadData => {
                self.loading = true;
                self.error = None;
                let link = ctx.link().clone();
                spawn_local(async move {
                    match SalesService::list_orders().await {
                        Ok(res) => link.send_message(Msg::DataLoaded(res.orders)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::DataLoaded(data) => {
                self.loading = false;
                self.orders = data;
                self.apply_filter();
                true
            }
            Msg::LoadError(err) => {
                self.error = Some(err);
                self.loading = false;
                true
            }
            Msg::Search(keyword) => {
                self.search_keyword = keyword;
                self.page = 0;
                self.apply_filter();
                true
            }
            Msg::ResetSearch => {
                self.search_keyword = String::new();
                self.page = 0;
                self.apply_filter();
                true
            }
            Msg::PageChanged(page) => {
                self.page = page;
                true
            }
            Msg::SetFilterStatus(status) => {
                self.filter_status = status;
                self.page = 0;
                self.apply_filter();
                true
            }
            Msg::OpenCreateModal => {
                self.reset_form();
                self.editing_order = None;
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(order) => {
                self.form_customer_id = order.customer_id.to_string();
                self.form_required_date = String::new();
                self.form_shipping_address = order.items.as_ref().and_then(|_| Some(String::new())).unwrap_or_default();
                self.form_billing_address = String::new();
                self.form_notes = String::new();
                self.form_payment_terms = String::new();
                self.form_error = None;
                self.editing_order = Some(order);
                self.modal_mode = ModalMode::Edit;
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.editing_order = None;
                self.form_error = None;
                true
            }
            Msg::SubmitForm => {
                if self.form_customer_id.is_empty() {
                    self.form_error = Some("客户ID不能为空".to_string());
                    return true;
                }
                if self.form_required_date.is_empty() {
                    self.form_error = Some("要求交货日期不能为空".to_string());
                    return true;
                }

                self.form_error = None;

                let customer_id = self.form_customer_id.parse::<i32>().unwrap_or(0);
                let req = CreateSalesOrderRequest {
                    customer_id,
                    required_date: self.form_required_date.clone(),
                    status: "draft".to_string(),
                    shipping_address: if self.form_shipping_address.is_empty() { None } else { Some(self.form_shipping_address.clone()) },
                    billing_address: if self.form_billing_address.is_empty() { None } else { Some(self.form_billing_address.clone()) },
                    notes: if self.form_notes.is_empty() { None } else { Some(self.form_notes.clone()) },
                    items: vec![],
                    payment_terms: if self.form_payment_terms.is_empty() { None } else { Some(self.form_payment_terms.clone()) },
                    remarks: None,
                    batch_no: None,
                    color_no: None,
                    dye_lot_no: None,
                    grade: None,
                    packaging_requirement: None,
                    quality_standard: None,
                };

                let link = ctx.link().clone();

                if self.modal_mode == ModalMode::Edit {
                    if let Some(order) = &self.editing_order {
                        let id = order.id;
                        let update_req = UpdateSalesOrderRequest {
                            required_date: Some(self.form_required_date.clone()),
                            status: None,
                            shipping_address: if self.form_shipping_address.is_empty() { None } else { Some(self.form_shipping_address.clone()) },
                            billing_address: if self.form_billing_address.is_empty() { None } else { Some(self.form_billing_address.clone()) },
                            notes: if self.form_notes.is_empty() { None } else { Some(self.form_notes.clone()) },
                            items: None,
                        };
                        spawn_local(async move {
                            match SalesService::update_order(id, update_req).await {
                                Ok(_) => {
                                    toast_helper::show_success("更新成功");
                                    link.send_message(Msg::FormSubmitted);
                                }
                                Err(e) => {
                                    toast_helper::show_error(&format!("更新失败: {}", e));
                                }
                            }
                        });
                    }
                } else {
                    spawn_local(async move {
                        match SalesService::create_order(req).await {
                            Ok(_) => {
                                toast_helper::show_success("创建成功");
                                link.send_message(Msg::FormSubmitted);
                            }
                            Err(e) => {
                                toast_helper::show_error(&format!("创建失败: {}", e));
                            }
                        }
                    });
                }
                false
            }
            Msg::FormSubmitted => {
                self.show_modal = false;
                self.editing_order = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::DeleteOrder(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match SalesService::delete_order(id).await {
                            Ok(_) => {
                                toast_helper::show_success("删除成功");
                                link.send_message(Msg::Deleted);
                            }
                            Err(e) => {
                                toast_helper::show_error(&format!("删除失败: {}", e));
                                link.send_message(Msg::CancelDelete);
                            }
                        }
                    });
                }
                false
            }
            Msg::CancelDelete => {
                self.show_delete_confirm = false;
                self.deleting_id = None;
                true
            }
            Msg::Deleted => {
                self.show_delete_confirm = false;
                self.deleting_id = None;
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::FormCustomerIdChanged(v) => { self.form_customer_id = v; true }
            Msg::FormRequiredDateChanged(v) => { self.form_required_date = v; true }
            Msg::FormShippingAddressChanged(v) => { self.form_shipping_address = v; true }
            Msg::FormBillingAddressChanged(v) => { self.form_billing_address = v; true }
            Msg::FormNotesChanged(v) => { self.form_notes = v; true }
            Msg::FormPaymentTermsChanged(v) => { self.form_payment_terms = v; true }

            // 原有功能
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
                    if let Ok(res) = WarehouseService::list().await {
                        link.send_message(Msg::WarehousesLoaded(res.data));
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
                        });
                    }
                }
                self.ship_items = items;
                self.shipping_order = Some(order);
                true
            }
            Msg::CloseShipModal => {
                self.shipping_order = None;
                self.ship_items.clear();
                self.submitting_ship = false;
                true
            }
            Msg::SubmitOrder(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    let _ = SalesService::submit_order(id).await;
                    link.send_message(Msg::LoadData);
                });
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
                ctx.link().send_message(Msg::LoadData);
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
        let link = ctx.link();

        html! {
            <div class="sales-order-page">
                <PageHeader title={"销售订单管理".to_string()} subtitle={Some("管理所有销售订单信息".to_string())}>
                    <PermissionGuard resource="sales_order" action="create">
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                            {"+ 新建销售订单"}
                        </button>
                    </PermissionGuard>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索订单号或客户名称...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                    <div class="filter-group">
                        <label>{"状态："}</label>
                        <select
                            class="form-control"
                            value={self.filter_status.clone()}
                            onchange={link.batch_callback(|e: Event| {
                                let target = e.target()?;
                                let select = target.unchecked_into::<web_sys::HtmlSelectElement>();
                                Some(Msg::SetFilterStatus(select.value()))
                            })}
                        >
                            <option value="全部">{"全部"}</option>
                            <option value="draft">{"草稿"}</option>
                            <option value="submitted">{"已提交"}</option>
                            <option value="approved">{"已审核"}</option>
                            <option value="shipped">{"已发货"}</option>
                            <option value="completed">{"已完成"}</option>
                        </select>
                    </div>
                </div>

                if self.loading {
                    <LoadingState message={"正在加载销售订单数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadData)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_orders.is_empty() {
                    <EmptyState
                        icon={"📦".to_string()}
                        title={"暂无销售订单数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一个销售订单".to_string()
                        } else {
                            "没有匹配搜索条件的订单".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"ID"}</th>
                                    <th>{"订单号"}</th>
                                    <th>{"客户"}</th>
                                    <th class="numeric">{"总金额"}</th>
                                    <th>{"状态"}</th>
                                    <th>{"创建时间"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_orders().iter().map(|order| {
                                    let order_clone = order.clone();
                                    let id = order.id;
                                    let id2 = order.id;
                                    let id3 = order.id;
                                    let id4 = order.id;
                                    html! {
                                        <tr>
                                            <td>{order.id}</td>
                                            <td>{&order.order_no}</td>
                                            <td>{order.customer_name.as_deref().unwrap_or("-")}</td>
                                            <td class="numeric">{&order.total_amount}</td>
                                            <td>{&order.status}</td>
                                            <td>{&order.created_at}</td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-secondary"
                                                        onclick={link.callback(move |_| Msg::PreparePrint(id))}
                                                    >
                                                        {"打印"}
                                                    </button>
                                                    if permissions::has_permission("sales_order", "update") {
                                                        <button
                                                            class="btn btn-sm btn-secondary"
                                                            onclick={link.callback(move |_| Msg::OpenEditModal(order_clone.clone()))}
                                                        >
                                                            {"编辑"}
                                                        </button>
                                                    }
                                                    if (order.status == "draft" || order.status == "rejected") && permissions::has_permission("sales_order", "update") {
                                                        <button
                                                            class="btn btn-sm btn-primary"
                                                            onclick={link.callback(move |_| Msg::SubmitOrder(id2))}
                                                        >
                                                            {"提交审批"}
                                                        </button>
                                                    }
                                                    if order.status == "approved" && permissions::has_permission("sales_order", "update") {
                                                        <button
                                                            class="btn btn-sm btn-primary"
                                                            onclick={link.callback(move |_| Msg::PrepareShip(id3))}
                                                        >
                                                            {"发货"}
                                                        </button>
                                                    }
                                                    <PermissionGuard resource="sales_order" action="delete">
                                                        <button
                                                            class="btn btn-sm btn-danger"
                                                            onclick={link.callback(move |_| Msg::DeleteOrder(id4))}
                                                        >
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

                        <Pagination
                            current_page={self.page}
                            page_size={self.page_size}
                            total={self.filtered_orders.len() as u64}
                            on_page_change={link.callback(|page| Msg::PageChanged(page))}
                        />
                    </div>
                }

                // 新建/编辑弹窗
                if self.show_modal {
                    {self.render_form_modal(ctx)}
                }

                // 删除确认对话框
                <ConfirmDialog
                    title={"确认删除".to_string()}
                    message={"确定要删除这个销售订单吗？此操作不可撤销。".to_string()}
                    confirm_text={"删除".to_string()}
                    cancel_text={"取消".to_string()}
                    confirm_class={"btn-danger".to_string()}
                    on_confirm={link.callback(|_| Msg::ConfirmDelete)}
                    on_cancel={link.callback(|_| Msg::CancelDelete)}
                    visible={self.show_delete_confirm}
                />

                {self.render_print_view()}
                {self.render_ship_modal(ctx)}
            </div>
        }
    }
}

impl SalesOrderPage {
    fn apply_filter(&mut self) {
        let mut result = self.orders.clone();

        if self.filter_status != "全部" {
            result = result.into_iter()
                .filter(|o| o.status == self.filter_status)
                .collect();
        }

        if self.search_keyword.is_empty() {
            self.filtered_orders = result;
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_orders = result.iter()
                .filter(|o| {
                    o.order_no.to_lowercase().contains(&keyword) ||
                    o.customer_name.as_ref().map(|n| n.to_lowercase().contains(&keyword)).unwrap_or(false)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_orders(&self) -> Vec<SalesOrder> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_orders[start..end.min(self.filtered_orders.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_customer_id = String::new();
        self.form_required_date = String::new();
        self.form_shipping_address = String::new();
        self.form_billing_address = String::new();
        self.form_notes = String::new();
        self.form_payment_terms = String::new();
        self.form_error = None;
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let is_edit = self.modal_mode == ModalMode::Edit;
        let title = if is_edit { "编辑销售订单" } else { "新建销售订单" };

        let on_customer_id_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormCustomerIdChanged(input.value()))
        });
        let on_required_date_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormRequiredDateChanged(input.value()))
        });
        let on_shipping_address_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormShippingAddressChanged(input.value()))
        });
        let on_billing_address_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormBillingAddressChanged(input.value()))
        });
        let on_notes_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormNotesChanged(input.value()))
        });
        let on_payment_terms_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormPaymentTermsChanged(input.value()))
        });

        html! {
            <div class="modal-overlay" onclick={link.callback(|_| Msg::CloseModal)}>
                <div class="modal-content" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <div class="modal-header">
                        <h3>{title}</h3>
                        <button class="close-btn" onclick={link.callback(|_| Msg::CloseModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        if let Some(err) = &self.form_error {
                            <div class="form-error">{err}</div>
                        }
                        <div class="form-group">
                            <label>{"客户ID *"}</label>
                            <input
                                type="number"
                                class="form-input"
                                value={self.form_customer_id.clone()}
                                oninput={on_customer_id_change}
                                placeholder="请输入客户ID"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"要求交货日期 *"}</label>
                            <input
                                type="date"
                                class="form-input"
                                value={self.form_required_date.clone()}
                                oninput={on_required_date_change}
                            />
                        </div>
                        <div class="form-group">
                            <label>{"送货地址"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_shipping_address.clone()}
                                oninput={on_shipping_address_change}
                                placeholder="请输入送货地址"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"账单地址"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_billing_address.clone()}
                                oninput={on_billing_address_change}
                                placeholder="请输入账单地址"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"付款条款"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_payment_terms.clone()}
                                oninput={on_payment_terms_change}
                                placeholder="如：月结30天"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"备注"}</label>
                            <textarea
                                class="form-input"
                                value={self.form_notes.clone()}
                                oninput={on_notes_change}
                                placeholder="请输入备注信息"
                                rows="3"
                            />
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={link.callback(|_| Msg::CloseModal)}>
                            {"取消"}
                        </button>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::SubmitForm)}>
                            {if is_edit { "保存修改" } else { "创建订单" }}
                        </button>
                    </div>
                </div>
            </div>
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
                                            let target = e.target()?;
                                            let select = target.unchecked_into::<web_sys::HtmlSelectElement>();
                                            if let Ok(wid) = select.value().parse::<i32>() {
                                                Some(Msg::UpdateShipItemWarehouse(idx, wid))
                                            } else {
                                                Some(Msg::UpdateShipItemWarehouse(idx, 0))
                                            }
                                        });

                                        let on_batch_change = ctx.link().batch_callback(move |e: Event| {
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
