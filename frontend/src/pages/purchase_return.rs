use crate::utils::permissions;
use crate::utils::toast_helper;
// 采购退货管理页面

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
use crate::models::purchase_return::{
    CreatePurchaseReturnRequest, CreatePurchaseReturnItemRequest,
    PurchaseReturn, PurchaseReturnQuery,
};
use crate::services::purchase_return_service::PurchaseReturnService;
use crate::services::crud_service::CrudService;

pub struct PurchaseReturnPage {
    returns: Vec<PurchaseReturn>,
    filtered_returns: Vec<PurchaseReturn>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_return: Option<PurchaseReturn>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    viewing_item: Option<PurchaseReturn>,
    // 表单字段
    form_return_no: String,
    form_supplier_id: String,
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
    DataLoaded(Vec<PurchaseReturn>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    OpenEditModal(PurchaseReturn),
    CloseModal,
    SubmitForm,
    FormSubmitted,
    DeleteReturn(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    ViewReturn(i32),
    CloseDetailModal,
    SubmitReturn(i32),
    ApproveReturn(i32),
    RejectReturn(i32),
    // 表单字段变更
    FormReturnNoChanged(String),
    FormSupplierIdChanged(String),
    FormProductIdChanged(String),
    FormQuantityChanged(String),
    FormReasonChanged(String),
}

impl Component for PurchaseReturnPage {
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
            viewing_item: None,
            form_return_no: String::new(),
            form_supplier_id: String::new(),
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
                let link = ctx.link().clone();
                spawn_local(async move {
                    let query = PurchaseReturnQuery {
                        page: Some(1),
                        page_size: Some(1000),
                        status: None,
                        supplier_id: None,
                    };
                    match PurchaseReturnService::list(query).await {
                        Ok(res) => link.send_message(Msg::DataLoaded(res.items)),
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
            Msg::OpenCreateModal => {
                self.reset_form();
                self.editing_return = None;
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(ret) => {
                self.form_return_no = ret.return_no.clone();
                self.form_supplier_id = ret.supplier_id.to_string();
                self.form_reason = ret.reason.clone().unwrap_or_default();
                self.form_error = None;
                self.editing_return = Some(ret);
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
                if self.form_supplier_id.is_empty() {
                    self.form_error = Some("供应商不能为空".to_string());
                    return true;
                }

                self.form_error = None;

                let supplier_id = self.form_supplier_id.parse::<i32>().unwrap_or(0);
                let req = CreatePurchaseReturnRequest {
                    return_no: self.form_return_no.clone(),
                    supplier_id,
                    order_id: None,
                    return_date: Some(String::new()),
                    reason_type: "质量问题".to_string(),
                    reason_detail: if self.form_reason.is_empty() { None } else { Some(self.form_reason.clone()) },
                    remarks: None,
                    items: vec![CreatePurchaseReturnItemRequest {
                        product_id: self.form_product_id.parse::<i32>().unwrap_or(0),
                        quantity: self.form_quantity.parse::<rust_decimal::Decimal>().unwrap_or_default(),
                        unit_price: None,
                    }],
                };

                let link = ctx.link().clone();

                if self.modal_mode == ModalMode::Edit {
                    if let Some(ret) = &self.editing_return {
                        let id = ret.id;
                        let update_req = crate::models::purchase_return::UpdatePurchaseReturnRequest {
                            return_date: None,
                            warehouse_id: None,
                            department_id: None,
                            reason: if self.form_reason.is_empty() { None } else { Some(self.form_reason.clone()) },
                            notes: None,
                        };
                        spawn_local(async move {
                            match PurchaseReturnService::update(id, update_req).await {
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
                        match PurchaseReturnService::create(req).await {
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
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match PurchaseReturnService::delete(id).await {
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
                    match PurchaseReturnService::submit(id).await {
                        Ok(_) => {
                            toast_helper::show_success("提交成功");
                            link.send_message(Msg::LoadData);
                        }
                        Err(e) => toast_helper::show_error(&format!("提交失败: {}", e)),
                    }
                });
                false
            }
            Msg::ApproveReturn(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseReturnService::approve(id).await {
                        Ok(_) => {
                            toast_helper::show_success("审批通过");
                            link.send_message(Msg::LoadData);
                        }
                        Err(e) => toast_helper::show_error(&format!("审批失败: {}", e)),
                    }
                });
                false
            }
            Msg::RejectReturn(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseReturnService::reject(id, "不符合要求".to_string()).await {
                        Ok(_) => {
                            toast_helper::show_success("已驳回");
                            link.send_message(Msg::LoadData);
                        }
                        Err(e) => toast_helper::show_error(&format!("驳回失败: {}", e)),
                    }
                });
                false
            }
            Msg::FormReturnNoChanged(v) => { self.form_return_no = v; true }
            Msg::FormSupplierIdChanged(v) => { self.form_supplier_id = v; true }
            Msg::FormProductIdChanged(v) => { self.form_product_id = v; true }
            Msg::FormQuantityChanged(v) => { self.form_quantity = v; true }
            Msg::FormReasonChanged(v) => { self.form_reason = v; true }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="purchase-return-page">
                <PageHeader title={"采购退货管理".to_string()} subtitle={Some("管理所有采购退货单信息".to_string())}>
                    <PermissionGuard resource="purchase_return" action="create">
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                            {"+ 新建退货单"}
                        </button>
                    </PermissionGuard>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索退货单号或供应商...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载采购退货数据...".to_string()} />
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
                        icon={"📦".to_string()}
                        title={"暂无采购退货单数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一个采购退货单".to_string()
                        } else {
                            "没有匹配搜索条件的退货单".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"退货单号"}</th>
                                    <th>{"关联订单"}</th>
                                    <th>{"供应商"}</th>
                                    <th>{"退货日期"}</th>
                                    <th>{"状态"}</th>
                                    <th class="numeric">{"退货数量"}</th>
                                    <th class="numeric">{"退货金额"}</th>
                                    <th>{"仓库"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_returns().iter().map(|ret| {
                                    let ret_clone = ret.clone();
                                    let id = ret.id;
                                    let status = ret.status.clone();
                                    html! {
                                        <tr>
                                            <td>{&ret.return_no}</td>
                                            <td>{ret.order_no.as_deref().unwrap_or("-")}</td>
                                            <td>{ret.supplier_name.as_deref().unwrap_or("-")}</td>
                                            <td>{&ret.return_date}</td>
                                            <td>{&status}</td>
                                            <td class="numeric">{&ret.total_quantity}</td>
                                            <td class="numeric">{&ret.total_amount}</td>
                                            <td>{ret.warehouse_name.as_deref().unwrap_or("-")}</td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-secondary"
                                                        onclick={link.callback(move |_| Msg::ViewReturn(id))}
                                                    >
                                                        {"查看"}
                                                    </button>
                                                    if status == "DRAFT" {
                                                        <PermissionGuard resource="purchase_return" action="update">
                                                            <button
                                                                class="btn btn-sm btn-secondary"
                                                                onclick={link.callback(move |_| Msg::OpenEditModal(ret_clone.clone()))}
                                                            >
                                                                {"编辑"}
                                                            </button>
                                                        </PermissionGuard>
                                                        <PermissionGuard resource="purchase_return" action="update">
                                                            <button
                                                                class="btn btn-sm btn-primary"
                                                                onclick={link.callback(move |_| Msg::SubmitReturn(id))}
                                                            >
                                                                {"提交"}
                                                            </button>
                                                        </PermissionGuard>
                                                        <PermissionGuard resource="purchase_return" action="delete">
                                                            <button
                                                                class="btn btn-sm btn-danger"
                                                                onclick={link.callback(move |_| Msg::DeleteReturn(id))}
                                                            >
                                                                {"删除"}
                                                            </button>
                                                        </PermissionGuard>
                                                    }
                                                    if status == "PENDING_APPROVAL" || status == "SUBMITTED" {
                                                        <PermissionGuard resource="purchase_return" action="approve">
                                                            <button
                                                                class="btn btn-sm btn-success"
                                                                onclick={link.callback(move |_| Msg::ApproveReturn(id))}
                                                            >
                                                                {"通过"}
                                                            </button>
                                                        </PermissionGuard>
                                                        <PermissionGuard resource="purchase_return" action="approve">
                                                            <button
                                                                class="btn btn-sm btn-warning"
                                                                onclick={link.callback(move |_| Msg::RejectReturn(id))}
                                                            >
                                                                {"驳回"}
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

                if self.show_modal {
                    {self.render_form_modal(ctx)}
                }

                if let Some(item) = &self.viewing_item {
                    {self.render_detail_modal(ctx, item)}
                }

                <ConfirmDialog
                    title={"确认删除".to_string()}
                    message={"确定要删除这个采购退货单吗？此操作不可撤销。".to_string()}
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

impl PurchaseReturnPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_returns = self.returns.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_returns = self.returns.iter()
                .filter(|r| {
                    r.return_no.to_lowercase().contains(&keyword) ||
                    r.supplier_name.as_ref().map(|s| s.to_lowercase().contains(&keyword)).unwrap_or(false) ||
                    r.status.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_returns(&self) -> Vec<PurchaseReturn> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_returns[start..end.min(self.filtered_returns.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_return_no = String::new();
        self.form_supplier_id = String::new();
        self.form_product_id = String::new();
        self.form_quantity = String::new();
        self.form_reason = String::new();
        self.form_error = None;
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let is_edit = self.modal_mode == ModalMode::Edit;
        let title = if is_edit { "编辑采购退货单" } else { "新建采购退货单" };

        let on_return_no_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormReturnNoChanged(input.value()))
        });
        let on_supplier_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormSupplierIdChanged(input.value()))
        });
        let on_product_change = link.batch_callback(|e: InputEvent| {
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
                            <label>{"供应商ID *"}</label>
                            <input
                                type="number"
                                class="form-input"
                                value={self.form_supplier_id.clone()}
                                oninput={on_supplier_change}
                                placeholder="请输入供应商ID"
                            />
                        </div>
                        if !is_edit {
                            <div class="form-group">
                                <label>{"产品ID"}</label>
                                <input
                                    type="number"
                                    class="form-input"
                                    value={self.form_product_id.clone()}
                                    oninput={on_product_change}
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
                            <label>{"退货原因"}</label>
                            <textarea
                                class="form-input"
                                value={self.form_reason.clone()}
                                oninput={on_reason_change}
                                placeholder="请输入退货原因"
                                rows="3"
                            />
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={link.callback(|_| Msg::CloseModal)}>
                            {"取消"}
                        </button>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::SubmitForm)}>
                            {if is_edit { "保存修改" } else { "创建退货单" }}
                        </button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_detail_modal(&self, ctx: &Context<Self>, item: &PurchaseReturn) -> Html {
        html! {
            <div class="modal-overlay" onclick={ctx.link().callback(|_| Msg::CloseDetailModal)}>
                <div class="modal-content" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())} style="width: 800px; max-width: 90vw;">
                    <div class="modal-header">
                        <h2>{"采购退货单详情"}</h2>
                        <button class="close-btn" onclick={ctx.link().callback(|_| Msg::CloseDetailModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        <div class="detail-grid" style="display: grid; grid-template-columns: 1fr 1fr; gap: 1rem;">
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"退货单号: "}</span>
                                <span class="detail-value">{&item.return_no}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"关联订单: "}</span>
                                <span class="detail-value">{item.order_no.as_deref().unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"供应商: "}</span>
                                <span class="detail-value">{item.supplier_name.as_deref().unwrap_or("-")}</span>
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
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"退货数量: "}</span>
                                <span class="detail-value">{&item.total_quantity}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"退货金额: "}</span>
                                <span class="detail-value">{&item.total_amount}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"仓库: "}</span>
                                <span class="detail-value">{item.warehouse_name.as_deref().unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"部门: "}</span>
                                <span class="detail-value">{item.department_name.as_deref().unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"原因: "}</span>
                                <span class="detail-value">{item.reason.as_deref().unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"备注: "}</span>
                                <span class="detail-value">{item.notes.as_deref().unwrap_or("-")}</span>
                            </div>
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-primary" onclick={ctx.link().callback(|_| Msg::CloseDetailModal)}>{"关闭"}</button>
                    </div>
                </div>
            </div>
        }
    }
}
