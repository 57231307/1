// 销售退货管理页面

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
use crate::models::sales_return::{
    CreateSalesReturnRequest, CreateSalesReturnItemRequest, SalesReturn, SalesReturnQuery,
};
use crate::services::sales_return_service::SalesReturnService;
use crate::services::crud_service::CrudService;

pub struct SalesReturnPage {
    returns: Vec<SalesReturn>,
    filtered_returns: Vec<SalesReturn>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_return: Option<SalesReturn>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    filter_status: String,
    viewing_item: Option<SalesReturn>,
    // 表单字段
    form_return_no: String,
    form_customer_id: String,
    form_product_id: String,
    form_quantity: String,
    form_reason: String,
    form_error: Option<String>,
}

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    Create,
    Edit,
}

pub enum Msg {
    LoadData,
    DataLoaded(Vec<SalesReturn>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    OpenEditModal(SalesReturn),
    CloseModal,
    SubmitForm,
    FormSubmitted,
    DeleteReturn(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    SetFilterStatus(String),
    ViewReturn(i32),
    CloseDetailModal,
    SubmitReturn(i32),
    ApproveReturn(i32),
    // 表单字段变更
    FormReturnNoChanged(String),
    FormCustomerIdChanged(String),
    FormProductIdChanged(String),
    FormQuantityChanged(String),
    FormReasonChanged(String),
}

impl Component for SalesReturnPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            returns: Vec::new(),
            filtered_returns: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            show_modal: false,
            modal_mode: ModalMode::Create,
            editing_return: None,
            show_delete_confirm: false,
            deleting_id: None,
            filter_status: String::from("全部"),
            viewing_item: None,
            form_return_no: String::new(),
            form_customer_id: String::new(),
            form_product_id: String::new(),
            form_quantity: String::new(),
            form_reason: String::new(),
            form_error: None,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadData);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadData => {
                self.loading = true;
                self.error = None;
                let query = SalesReturnQuery {
                    page: Some(1),
                    page_size: Some(1000),
                    status: None,
                    customer_id: None,
                    return_no: None,
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match SalesReturnService::list(query).await {
                        Ok(res) => link.send_message(Msg::DataLoaded(res.data)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::DataLoaded(data) => {
                self.loading = false;
                self.returns = data;
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
                self.editing_return = None;
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(return_order) => {
                self.form_return_no = return_order.return_no.clone();
                self.form_customer_id = return_order.customer_id.to_string();
                self.form_reason = return_order.reason.clone();
                self.form_error = None;
                self.editing_return = Some(return_order);
                self.modal_mode = ModalMode::Edit;
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.editing_return = None;
                self.form_error = None;
                true
            }
            Msg::SubmitForm => {
                if self.form_return_no.is_empty() {
                    self.form_error = Some("退货单号不能为空".to_string());
                    return true;
                }
                if self.form_customer_id.is_empty() {
                    self.form_error = Some("客户ID不能为空".to_string());
                    return true;
                }
                if self.form_reason.is_empty() {
                    self.form_error = Some("退货原因不能为空".to_string());
                    return true;
                }

                self.form_error = None;

                let customer_id = self.form_customer_id.parse::<i32>().unwrap_or(0);
                let product_id = self.form_product_id.parse::<i32>().unwrap_or(0);
                let quantity = self.form_quantity.parse().unwrap_or_default();

                let req = CreateSalesReturnRequest {
                    return_no: self.form_return_no.clone(),
                    sales_order_id: None,
                    customer_id,
                    return_date: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
                    warehouse_id: 1,
                    reason: self.form_reason.clone(),
                    remarks: None,
                    items: vec![
                        CreateSalesReturnItemRequest {
                            product_id,
                            quantity,
                            unit_price: None,
                        }
                    ],
                };

                let link = ctx.link().clone();

                if self.modal_mode == ModalMode::Edit {
                    if let Some(return_order) = &self.editing_return {
                        let _id = return_order.id;
                        spawn_local(async move {
                            match SalesReturnService::create(req).await {
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
                        match SalesReturnService::create(req).await {
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
                self.editing_return = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::DeleteReturn(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(_id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        toast_helper::show_error("删除功能暂不可用");
                        link.send_message(Msg::CancelDelete);
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
            Msg::ViewReturn(id) => {
                self.viewing_item = self.returns.iter().find(|i| i.id == id).cloned();
                true
            }
            Msg::CloseDetailModal => {
                self.viewing_item = None;
                true
            }
            Msg::SubmitReturn(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    if let Err(e) = SalesReturnService::submit(id).await {
                        toast_helper::show_error(&format!("提交失败: {}", e));
                    } else {
                        toast_helper::show_success("提交成功");
                        link.send_message(Msg::LoadData);
                    }
                });
                false
            }
            Msg::ApproveReturn(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    if let Err(e) = SalesReturnService::approve(id).await {
                        toast_helper::show_error(&format!("审批失败: {}", e));
                    } else {
                        toast_helper::show_success("审批成功");
                        link.send_message(Msg::LoadData);
                    }
                });
                false
            }
            Msg::FormReturnNoChanged(v) => { self.form_return_no = v; true }
            Msg::FormCustomerIdChanged(v) => { self.form_customer_id = v; true }
            Msg::FormProductIdChanged(v) => { self.form_product_id = v; true }
            Msg::FormQuantityChanged(v) => { self.form_quantity = v; true }
            Msg::FormReasonChanged(v) => { self.form_reason = v; true }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="sales-return-page">
                <PageHeader title={"销售退货管理".to_string()} subtitle={Some("管理所有销售退货信息".to_string())}>
                    <PermissionGuard resource="sales_return" action="create">
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                            {"+ 新建退货单"}
                        </button>
                    </PermissionGuard>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索退货单号或客户...".to_string()}
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
                            <option value="DRAFT">{"草稿"}</option>
                            <option value="SUBMITTED">{"已提交"}</option>
                            <option value="APPROVED">{"已审批"}</option>
                            <option value="REJECTED">{"已拒绝"}</option>
                            <option value="COMPLETED">{"已完成"}</option>
                        </select>
                    </div>
                </div>

                if self.loading {
                    <LoadingState message={"正在加载销售退货数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadData)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_returns.is_empty() {
                    <EmptyState
                        icon={"🔄".to_string()}
                        title={"暂无销售退货数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一个退货单".to_string()
                        } else {
                            "没有匹配搜索条件的退货单".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"ID"}</th>
                                    <th>{"退货单号"}</th>
                                    <th>{"客户ID"}</th>
                                    <th>{"退货日期"}</th>
                                    <th>{"状态"}</th>
                                    <th class="numeric">{"退货金额"}</th>
                                    <th>{"退货原因"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_returns().iter().map(|return_order| {
                                    let return_clone = return_order.clone();
                                    let id = return_order.id;
                                    let id2 = return_order.id;
                                    let id3 = return_order.id;
                                    html! {
                                        <tr>
                                            <td>{return_order.id}</td>
                                            <td>{&return_order.return_no}</td>
                                            <td>{return_order.customer_id}</td>
                                            <td>{&return_order.return_date}</td>
                                            <td>
                                                <span class={format!("status-badge status-{}", return_order.status.to_lowercase())}>
                                                    {&return_order.status}
                                                </span>
                                            </td>
                                            <td class="numeric">{format!("¥{:.2}", return_order.total_amount)}</td>
                                            <td>{&return_order.reason}</td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-info"
                                                        onclick={link.callback(move |_| Msg::ViewReturn(id))}
                                                    >
                                                        {"查看"}
                                                    </button>
                                                    if permissions::has_permission("sales_return", "update") {
                                                        <button
                                                            class="btn btn-sm btn-secondary"
                                                            onclick={link.callback(move |_| Msg::OpenEditModal(return_clone.clone()))}
                                                        >
                                                            {"编辑"}
                                                        </button>
                                                    }
                                                    if return_order.status == "DRAFT" {
                                                        <PermissionGuard resource="sales_return" action="create">
                                                            <button
                                                                class="btn btn-sm btn-primary"
                                                                onclick={link.callback(move |_| Msg::SubmitReturn(id2))}
                                                            >
                                                                {"提交"}
                                                            </button>
                                                        </PermissionGuard>
                                                    }
                                                    if return_order.status == "SUBMITTED" {
                                                        <PermissionGuard resource="sales_return" action="approve">
                                                            <button
                                                                class="btn btn-sm btn-success"
                                                                onclick={link.callback(move |_| Msg::ApproveReturn(id3))}
                                                            >
                                                                {"审批"}
                                                            </button>
                                                        </PermissionGuard>
                                                    }
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
                            total={self.filtered_returns.len() as u64}
                            on_page_change={link.callback(|page| Msg::PageChanged(page))}
                        />
                    </div>
                }

                // 新建/编辑弹窗
                if self.show_modal {
                    {self.render_form_modal(ctx)}
                }

                // 详情弹窗
                if self.viewing_item.is_some() {
                    {self.render_detail_modal(ctx)}
                }

                // 删除确认对话框
                <ConfirmDialog
                    title={"确认删除".to_string()}
                    message={"确定要删除这个退货单吗？此操作不可撤销。".to_string()}
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

impl SalesReturnPage {
    fn apply_filter(&mut self) {
        let mut result = self.returns.clone();

        if self.filter_status != "全部" {
            result = result.into_iter()
                .filter(|r| r.status == self.filter_status)
                .collect();
        }

        if self.search_keyword.is_empty() {
            self.filtered_returns = result;
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_returns = result.iter()
                .filter(|r| {
                    r.return_no.to_lowercase().contains(&keyword) ||
                    r.reason.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_returns(&self) -> Vec<SalesReturn> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_returns[start..end.min(self.filtered_returns.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_return_no = String::new();
        self.form_customer_id = String::new();
        self.form_product_id = String::new();
        self.form_quantity = String::new();
        self.form_reason = String::new();
        self.form_error = None;
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let is_edit = self.modal_mode == ModalMode::Edit;
        let title = if is_edit { "编辑退货单" } else { "新建退货单" };

        let on_return_no_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormReturnNoChanged(input.value()))
        });
        let on_customer_id_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormCustomerIdChanged(input.value()))
        });
        let on_product_id_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormProductIdChanged(input.value()))
        });
        let on_quantity_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormQuantityChanged(input.value()))
        });
        let on_reason_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormReasonChanged(input.value()))
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
                            <label>{"退货单号 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_return_no.clone()}
                                oninput={on_return_no_change}
                                placeholder="请输入退货单号"
                            />
                        </div>
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
                        if !is_edit {
                            <div class="form-group">
                                <label>{"产品ID"}</label>
                                <input
                                    type="number"
                                    class="form-input"
                                    value={self.form_product_id.clone()}
                                    oninput={on_product_id_change}
                                    placeholder="请输入产品ID"
                                />
                            </div>
                            <div class="form-group">
                                <label>{"退货数量"}</label>
                                <input
                                    type="number"
                                    class="form-input"
                                    value={self.form_quantity.clone()}
                                    oninput={on_quantity_change}
                                    placeholder="请输入退货数量"
                                />
                            </div>
                        }
                        <div class="form-group">
                            <label>{"退货原因 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_reason.clone()}
                                oninput={on_reason_change}
                                placeholder="请输入退货原因"
                            />
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={link.callback(|_| Msg::CloseModal)}>
                            {"取消"}
                        </button>
                        <PermissionGuard resource="sales_return" action="create">
                            <button class="btn btn-primary" onclick={link.callback(|_| Msg::SubmitForm)}>
                                {if is_edit { "保存修改" } else { "创建退货单" }}
                            </button>
                        </PermissionGuard>
                    </div>
                </div>
            </div>
        }
    }

    fn render_detail_modal(&self, ctx: &Context<Self>) -> Html {
        if let Some(item) = &self.viewing_item {
            html! {
                <div class="modal-overlay" onclick={ctx.link().callback(|_| Msg::CloseDetailModal)}>
                    <div class="modal-content" style="width: 800px; max-width: 90vw;" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                        <div class="modal-header">
                            <h3>{"退货单详情"}</h3>
                            <button class="close-btn" onclick={ctx.link().callback(|_| Msg::CloseDetailModal)}>{"×"}</button>
                        </div>
                        <div class="modal-body">
                            <div class="detail-grid" style="display: grid; grid-template-columns: 1fr 1fr; gap: 1rem;">
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"ID: "}</span>
                                    <span class="detail-value">{item.id.to_string()}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"退货单号: "}</span>
                                    <span class="detail-value">{&item.return_no}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"销售订单ID: "}</span>
                                    <span class="detail-value">{item.sales_order_id.map_or("-".to_string(), |v| v.to_string())}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"客户ID: "}</span>
                                    <span class="detail-value">{item.customer_id.to_string()}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"退货日期: "}</span>
                                    <span class="detail-value">{&item.return_date}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"状态: "}</span>
                                    <span class="detail-value">{&item.status}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"退货金额: "}</span>
                                    <span class="detail-value">{item.total_amount.to_string()}</span>
                                </div>
                                <div class="detail-item">
                                    <span class="detail-label" style="font-weight: bold; color: #666;">{"退货原因: "}</span>
                                    <span class="detail-value">{&item.reason}</span>
                                </div>
                            </div>
                        </div>
                        <div class="modal-footer">
                            <button class="btn btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseDetailModal)}>{"关闭"}</button>
                        </div>
                    </div>
                </div>
            }
        } else {
            html! {}
        }
    }
}
