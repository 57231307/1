// 面料订单管理页面

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
use crate::models::fabric_order::{
    FabricOrder, FabricOrderQuery,
    CreateFabricOrderRequest, UpdateFabricOrderRequest,
};
use crate::services::fabric_order_service::FabricOrderService;
use crate::services::crud_service::CrudService;

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    View,
    Create,
    Edit,
}

pub struct FabricOrderPage {
    orders: Vec<FabricOrder>,
    filtered_orders: Vec<FabricOrder>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_order: Option<FabricOrder>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    // 表单字段
    form_customer_id: String,
    form_order_date: String,
    form_required_date: String,
    form_batch_no: String,
    form_color_no: String,
    form_remarks: String,
    form_error: Option<String>,
}

pub enum Msg {
    LoadOrders,
    OrdersLoaded(Vec<FabricOrder>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    OpenEditModal(FabricOrder),
    OpenViewModal(FabricOrder),
    CloseModal,
    SubmitForm,
    FormSubmitted,
    ApproveOrder(i32),
    DeleteOrder(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    // 表单字段变更
    FormCustomerIdChanged(String),
    FormOrderDateChanged(String),
    FormRequiredDateChanged(String),
    FormBatchNoChanged(String),
    FormColorNoChanged(String),
    FormRemarksChanged(String),
}

impl Component for FabricOrderPage {
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
            modal_mode: ModalMode::View,
            editing_order: None,
            show_delete_confirm: false,
            deleting_id: None,
            form_customer_id: String::new(),
            form_order_date: String::new(),
            form_required_date: String::new(),
            form_batch_no: String::new(),
            form_color_no: String::new(),
            form_remarks: String::new(),
            form_error: None,
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
                self.error = None;
                let query = FabricOrderQuery {
                    page: Some(1),
                    page_size: Some(1000),
                    customer_id: None,
                    order_no: None,
                    status: None,
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
                self.loading = false;
                self.orders = orders;
                self.apply_filter();
                true
            }
            Msg::LoadError(e) => {
                self.error = Some(e);
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
            Msg::OpenCreateModal => {
                self.reset_form();
                self.editing_order = None;
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(order) => {
                self.form_customer_id = order.customer_id.to_string();
                self.form_order_date = order.order_date.clone();
                self.form_required_date = order.required_date.clone();
                self.form_batch_no = order.batch_no.clone().unwrap_or_default();
                self.form_color_no = order.color_no.clone().unwrap_or_default();
                self.form_remarks = order.remarks.clone().unwrap_or_default();
                self.form_error = None;
                self.editing_order = Some(order);
                self.modal_mode = ModalMode::Edit;
                self.show_modal = true;
                true
            }
            Msg::OpenViewModal(order) => {
                self.editing_order = Some(order);
                self.modal_mode = ModalMode::View;
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
                // 表单验证
                if self.form_customer_id.is_empty() {
                    self.form_error = Some("客户ID不能为空".to_string());
                    return true;
                }
                if self.form_order_date.is_empty() {
                    self.form_error = Some("订单日期不能为空".to_string());
                    return true;
                }
                if self.form_required_date.is_empty() {
                    self.form_error = Some("要求交货日期不能为空".to_string());
                    return true;
                }

                self.form_error = None;

                if self.modal_mode == ModalMode::Edit {
                    if let Some(order) = &self.editing_order {
                        let id = order.id;
                        let req = UpdateFabricOrderRequest {
                            required_date: Some(self.form_required_date.clone()),
                            status: None,
                            shipping_address: None,
                            delivery_address: None,
                            payment_terms: None,
                            remarks: if self.form_remarks.is_empty() { None } else { Some(self.form_remarks.clone()) },
                            items: None,
                            batch_no: if self.form_batch_no.is_empty() { None } else { Some(self.form_batch_no.clone()) },
                            color_no: if self.form_color_no.is_empty() { None } else { Some(self.form_color_no.clone()) },
                            packaging_requirement: None,
                            quality_standard: None,
                        };
                        let link = ctx.link().clone();
                        spawn_local(async move {
                            match FabricOrderService::update(id, req).await {
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
                    let customer_id = self.form_customer_id.parse().unwrap_or(0);
                    let req = CreateFabricOrderRequest {
                        customer_id,
                        order_date: self.form_order_date.clone(),
                        required_date: self.form_required_date.clone(),
                        items: vec![],
                        shipping_address: None,
                        delivery_address: None,
                        payment_terms: None,
                        remarks: if self.form_remarks.is_empty() { None } else { Some(self.form_remarks.clone()) },
                        batch_no: if self.form_batch_no.is_empty() { None } else { Some(self.form_batch_no.clone()) },
                        color_no: if self.form_color_no.is_empty() { None } else { Some(self.form_color_no.clone()) },
                        dye_lot_no: None,
                        grade: None,
                        packaging_requirement: None,
                        quality_standard: None,
                    };
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match FabricOrderService::create(req).await {
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
                ctx.link().send_message(Msg::LoadOrders);
                false
            }
            Msg::ApproveOrder(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match FabricOrderService::approve(id).await {
                        Ok(_) => {
                            toast_helper::show_success("审批成功");
                            link.send_message(Msg::LoadOrders);
                        }
                        Err(e) => {
                            toast_helper::show_error(&format!("审批失败: {}", e));
                        }
                    }
                });
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
                        match FabricOrderService::delete(id).await {
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
                ctx.link().send_message(Msg::LoadOrders);
                false
            }
            Msg::FormCustomerIdChanged(v) => { self.form_customer_id = v; true }
            Msg::FormOrderDateChanged(v) => { self.form_order_date = v; true }
            Msg::FormRequiredDateChanged(v) => { self.form_required_date = v; true }
            Msg::FormBatchNoChanged(v) => { self.form_batch_no = v; true }
            Msg::FormColorNoChanged(v) => { self.form_color_no = v; true }
            Msg::FormRemarksChanged(v) => { self.form_remarks = v; true }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="fabric-order-page">
                <PageHeader title={"面料订单管理".to_string()} subtitle={Some("管理面料销售订单".to_string())}>
                    <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                        {"+ 新建订单"}
                    </button>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索订单编号或客户名称...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载面料订单数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadOrders)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_orders.is_empty() {
                    <EmptyState
                        icon={"📋".to_string()}
                        title={"暂无面料订单数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一个订单".to_string()
                        } else {
                            "没有匹配搜索条件的订单".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"订单编号"}</th>
                                    <th>{"客户名称"}</th>
                                    <th>{"订单日期"}</th>
                                    <th>{"要求交货日期"}</th>
                                    <th>{"订单状态"}</th>
                                    <th class="numeric">{"总金额"}</th>
                                    <th>{"批次号"}</th>
                                    <th>{"色号"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_orders().iter().map(|order| {
                                    let order_clone = order.clone();
                                    let order_clone2 = order.clone();
                                    let order_clone3 = order.clone();
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
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-info"
                                                        onclick={link.callback(move |_| Msg::OpenViewModal(order_clone.clone()))}
                                                    >
                                                        {"查看"}
                                                    </button>
                                                    <button
                                                        class="btn btn-sm btn-secondary"
                                                        onclick={link.callback(move |_| Msg::OpenEditModal(order_clone2.clone()))}
                                                    >
                                                        {"编辑"}
                                                    </button>
                                                    if order_status == "待审批" {
                                                        <PermissionGuard resource="fabric_order" action="approve">
                                                            <button
                                                                class="btn btn-sm btn-success"
                                                                onclick={link.callback(move |_| Msg::ApproveOrder(order_id))}
                                                            >
                                                                {"审批"}
                                                            </button>
                                                        </PermissionGuard>
                                                    }
                                                    <PermissionGuard resource="fabric_order" action="delete">
                                                        <button
                                                            class="btn btn-sm btn-danger"
                                                            onclick={link.callback(move |_| Msg::DeleteOrder(order_id))}
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

                // 弹窗
                if self.show_modal {
                    {self.render_modal(ctx)}
                }

                // 删除确认对话框
                <ConfirmDialog
                    title={"确认删除".to_string()}
                    message={"确定要删除这个面料订单吗？此操作不可撤销。".to_string()}
                    confirm_text={"删除".to_string()}
                    cancel_text={"取消".to_string()}
                    confirm_class={"btn-danger".to_string()}
                    on_confirm={link.callback(|_| Msg::ConfirmDelete)}
                    on_cancel={link.callback(|_| Msg::CancelDelete)}
                    visible={self.show_delete_confirm}
                />
            </div>
        }
    }
}

impl FabricOrderPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_orders = self.orders.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_orders = self.orders.iter()
                .filter(|o| {
                    o.order_no.to_lowercase().contains(&keyword) ||
                    o.customer_name.as_ref().map(|n| n.to_lowercase().contains(&keyword)).unwrap_or(false) ||
                    o.batch_no.as_ref().map(|b| b.to_lowercase().contains(&keyword)).unwrap_or(false) ||
                    o.color_no.as_ref().map(|c| c.to_lowercase().contains(&keyword)).unwrap_or(false)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_orders(&self) -> Vec<FabricOrder> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_orders[start..end.min(self.filtered_orders.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_customer_id = String::new();
        self.form_order_date = String::new();
        self.form_required_date = String::new();
        self.form_batch_no = String::new();
        self.form_color_no = String::new();
        self.form_remarks = String::new();
        self.form_error = None;
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
        match self.modal_mode {
            ModalMode::View => self.render_view_modal(ctx),
            _ => self.render_form_modal(ctx),
        }
    }

    fn render_view_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        if let Some(order) = &self.editing_order {
            html! {
                <div class="modal-overlay" onclick={link.callback(|_| Msg::CloseModal)}>
                    <div class="modal-content modal-lg" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                        <div class="modal-header">
                            <h3>{"订单详情"}</h3>
                            <button class="close-btn" onclick={link.callback(|_| Msg::CloseModal)}>{"×"}</button>
                        </div>
                        <div class="modal-body">
                            <div class="detail-grid">
                                <div class="detail-item">
                                    <label>{"订单编号："}</label>
                                    <span>{&order.order_no}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"客户名称："}</label>
                                    <span>{order.customer_name.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"订单日期："}</label>
                                    <span>{&order.order_date}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"要求交货日期："}</label>
                                    <span>{&order.required_date}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"订单状态："}</label>
                                    <span>{&order.status}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"总金额："}</label>
                                    <span>{&order.total_amount}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"已付金额："}</label>
                                    <span>{&order.paid_amount}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"批次号："}</label>
                                    <span>{order.batch_no.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"色号："}</label>
                                    <span>{order.color_no.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"缸号："}</label>
                                    <span>{order.dye_lot_no.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"等级："}</label>
                                    <span>{order.grade.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"包装要求："}</label>
                                    <span>{order.packaging_requirement.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"质量标准："}</label>
                                    <span>{order.quality_standard.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"发货地址："}</label>
                                    <span>{order.shipping_address.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"交货地址："}</label>
                                    <span>{order.delivery_address.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"付款条件："}</label>
                                    <span>{order.payment_terms.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"备注："}</label>
                                    <span>{order.remarks.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"创建时间："}</label>
                                    <span>{&order.created_at}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"更新时间："}</label>
                                    <span>{&order.updated_at}</span>
                                </div>
                            </div>
                        </div>
                        <div class="modal-footer">
                            <button class="btn btn-primary" onclick={link.callback(|_| Msg::CloseModal)}>{"关闭"}</button>
                        </div>
                    </div>
                </div>
            }
        } else {
            html! {}
        }
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let is_edit = self.modal_mode == ModalMode::Edit;
        let title = if is_edit { "编辑面料订单" } else { "新建面料订单" };

        let on_customer_id_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormCustomerIdChanged(input.value()))
        });
        let on_order_date_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormOrderDateChanged(input.value()))
        });
        let on_required_date_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormRequiredDateChanged(input.value()))
        });
        let on_batch_no_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormBatchNoChanged(input.value()))
        });
        let on_color_no_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormColorNoChanged(input.value()))
        });
        let on_remarks_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormRemarksChanged(input.value()))
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
                                type="text"
                                class="form-input"
                                value={self.form_customer_id.clone()}
                                oninput={on_customer_id_change}
                                placeholder="请输入客户ID"
                                disabled={is_edit}
                            />
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"订单日期 *"}</label>
                                <input
                                    type="date"
                                    class="form-input"
                                    value={self.form_order_date.clone()}
                                    oninput={on_order_date_change}
                                    disabled={is_edit}
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
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"批次号"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_batch_no.clone()}
                                    oninput={on_batch_no_change}
                                    placeholder="请输入批次号"
                                />
                            </div>
                            <div class="form-group">
                                <label>{"色号"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_color_no.clone()}
                                    oninput={on_color_no_change}
                                    placeholder="请输入色号"
                                />
                            </div>
                        </div>
                        <div class="form-group">
                            <label>{"备注"}</label>
                            <textarea
                                class="form-input"
                                value={self.form_remarks.clone()}
                                oninput={on_remarks_change}
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
}
