// 采购检验页面

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
use crate::models::purchase_inspection::{
    PurchaseInspection, PurchaseInspectionQuery, CreatePurchaseInspectionRequest, CompleteInspectionRequest,
};
use crate::services::purchase_inspection_service::PurchaseInspectionService;
use crate::services::crud_service::CrudService;

pub struct PurchaseInspectionPage {
    inspections: Vec<PurchaseInspection>,
    filtered_inspections: Vec<PurchaseInspection>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_inspection: Option<PurchaseInspection>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    show_complete_modal: bool,
    completing_id: Option<i32>,
    viewing_item: Option<PurchaseInspection>,
    // 表单字段
    form_receipt_id: String,
    form_supplier_id: String,
    form_inspection_date: String,
    form_notes: String,
    form_error: Option<String>,
    // 完成检验表单
    form_pass_quantity: String,
    form_reject_quantity: String,
    form_inspection_result: String,
}

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    Create,
    Edit,
}

pub enum Msg {
    LoadData,
    DataLoaded(Vec<PurchaseInspection>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    OpenEditModal(PurchaseInspection),
    CloseModal,
    SubmitForm,
    FormSubmitted,
    DeleteInspection(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    ViewInspection(i32),
    CloseDetailModal,
    ShowCompleteModal(i32),
    HideCompleteModal,
    SubmitComplete,
    // 表单字段变更
    FormReceiptIdChanged(String),
    FormSupplierIdChanged(String),
    FormInspectionDateChanged(String),
    FormNotesChanged(String),
    FormPassQuantityChanged(String),
    FormRejectQuantityChanged(String),
    FormInspectionResultChanged(String),
}

impl Component for PurchaseInspectionPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            inspections: Vec::new(),
            filtered_inspections: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            show_modal: false,
            modal_mode: ModalMode::Create,
            editing_inspection: None,
            show_delete_confirm: false,
            deleting_id: None,
            show_complete_modal: false,
            completing_id: None,
            viewing_item: None,
            form_receipt_id: String::new(),
            form_supplier_id: String::new(),
            form_inspection_date: String::new(),
            form_notes: String::new(),
            form_error: None,
            form_pass_quantity: String::new(),
            form_reject_quantity: String::new(),
            form_inspection_result: "PASSED".to_string(),
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
                    let query = PurchaseInspectionQuery {
                        page: Some(1),
                        page_size: Some(1000),
                        status: None,
                        supplier_id: None,
                    };
                    match PurchaseInspectionService::list(query).await {
                        Ok(res) => link.send_message(Msg::DataLoaded(res.items)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::DataLoaded(data) => {
                self.loading = false;
                self.inspections = data;
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
                self.editing_inspection = None;
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(inspection) => {
                self.form_receipt_id = inspection.id.to_string();
                self.form_supplier_id = inspection.supplier_id.to_string();
                self.form_inspection_date = inspection.inspection_date.clone();
                self.form_notes = inspection.remarks.clone().unwrap_or_default();
                self.form_error = None;
                self.editing_inspection = Some(inspection);
                self.modal_mode = ModalMode::Edit;
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.editing_inspection = None;
                self.form_error = None;
                true
            }
            Msg::SubmitForm => {
                if self.form_receipt_id.is_empty() {
                    self.form_error = Some("入库单ID不能为空".to_string());
                    return true;
                }
                if self.form_supplier_id.is_empty() {
                    self.form_error = Some("供应商不能为空".to_string());
                    return true;
                }
                if self.form_inspection_date.is_empty() {
                    self.form_error = Some("检验日期不能为空".to_string());
                    return true;
                }

                self.form_error = None;

                let receipt_id = self.form_receipt_id.parse::<i32>().unwrap_or(0);
                let supplier_id = self.form_supplier_id.parse::<i32>().unwrap_or(0);
                let req = CreatePurchaseInspectionRequest {
                    receipt_id,
                    order_id: None,
                    supplier_id,
                    inspection_date: self.form_inspection_date.clone(),
                    inspector_id: None,
                    inspection_type: None,
                    notes: if self.form_notes.is_empty() { None } else { Some(self.form_notes.clone()) },
                };

                let link = ctx.link().clone();

                if self.modal_mode == ModalMode::Edit {
                    if let Some(inspection) = &self.editing_inspection {
                        let id = inspection.id;
                        let update_req = crate::models::purchase_inspection::UpdatePurchaseInspectionRequest {
                            sample_size: None,
                            defect_description: None,
                            notes: if self.form_notes.is_empty() { None } else { Some(self.form_notes.clone()) },
                        };
                        spawn_local(async move {
                            match PurchaseInspectionService::update(id, update_req).await {
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
                        match PurchaseInspectionService::create(req).await {
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
                self.editing_inspection = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::DeleteInspection(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match PurchaseInspectionService::delete(id).await {
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
            Msg::ViewInspection(id) => {
                self.viewing_item = self.inspections.iter().find(|i| i.id == id).cloned();
                true
            }
            Msg::CloseDetailModal => {
                self.viewing_item = None;
                true
            }
            Msg::ShowCompleteModal(id) => {
                self.completing_id = Some(id);
                self.show_complete_modal = true;
                self.form_pass_quantity = String::new();
                self.form_reject_quantity = String::new();
                self.form_inspection_result = "PASSED".to_string();
                true
            }
            Msg::HideCompleteModal => {
                self.show_complete_modal = false;
                self.completing_id = None;
                true
            }
            Msg::SubmitComplete => {
                if let Some(id) = self.completing_id {
                    let req = CompleteInspectionRequest {
                        pass_quantity: if self.form_pass_quantity.is_empty() { "0".to_string() } else { self.form_pass_quantity.clone() },
                        reject_quantity: if self.form_reject_quantity.is_empty() { "0".to_string() } else { self.form_reject_quantity.clone() },
                        inspection_result: self.form_inspection_result.clone(),
                    };
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match PurchaseInspectionService::complete(id, req).await {
                            Ok(_) => {
                                toast_helper::show_success("检验完成");
                                link.send_message(Msg::HideCompleteModal);
                                link.send_message(Msg::LoadData);
                            }
                            Err(e) => toast_helper::show_error(&format!("检验失败: {}", e)),
                        }
                    });
                }
                false
            }
            Msg::FormReceiptIdChanged(v) => { self.form_receipt_id = v; true }
            Msg::FormSupplierIdChanged(v) => { self.form_supplier_id = v; true }
            Msg::FormInspectionDateChanged(v) => { self.form_inspection_date = v; true }
            Msg::FormNotesChanged(v) => { self.form_notes = v; true }
            Msg::FormPassQuantityChanged(v) => { self.form_pass_quantity = v; true }
            Msg::FormRejectQuantityChanged(v) => { self.form_reject_quantity = v; true }
            Msg::FormInspectionResultChanged(v) => { self.form_inspection_result = v; true }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="purchase-inspection-page">
                <PageHeader title={"采购检验管理".to_string()} subtitle={Some("管理所有采购检验单信息".to_string())}>
                    <PermissionGuard resource="purchase_inspection" action="create">
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                            {"+ 新建检验单"}
                        </button>
                    </PermissionGuard>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索检验单号或供应商...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载采购检验数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadData)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_inspections.is_empty() {
                    <EmptyState
                        icon={"🔍".to_string()}
                        title={"暂无采购检验单数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一个采购检验单".to_string()
                        } else {
                            "没有匹配搜索条件的检验单".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"检验单号"}</th>
                                    <th>{"供应商"}</th>
                                    <th>{"检验日期"}</th>
                                    <th>{"检验结果"}</th>
                                    <th class="numeric">{"合格数量"}</th>
                                    <th class="numeric">{"不合格数量"}</th>
                                    <th>{"备注"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_inspections().iter().map(|inspection| {
                                    let inspection_clone = inspection.clone();
                                    let id = inspection.id;
                                    let result_text = match inspection.result.as_str() {
                                        "PENDING" => "待检验",
                                        "PASSED" => "合格",
                                        "FAILED" => "不合格",
                                        _ => &inspection.result,
                                    };
                                    let result_class = match inspection.result.as_str() {
                                        "PENDING" => "status-pending",
                                        "PASSED" => "status-passed",
                                        "FAILED" => "status-failed",
                                        _ => "",
                                    };
                                    html! {
                                        <tr>
                                            <td>{&inspection.inspection_no}</td>
                                            <td>{inspection.supplier_name.as_deref().unwrap_or("-")}</td>
                                            <td>{&inspection.inspection_date}</td>
                                            <td class={result_class}>{result_text}</td>
                                            <td class="numeric">{&inspection.qualified_quantity}</td>
                                            <td class="numeric">{&inspection.unqualified_quantity}</td>
                                            <td>{inspection.remarks.as_deref().unwrap_or("-")}</td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-secondary"
                                                        onclick={link.callback(move |_| Msg::ViewInspection(id))}
                                                    >
                                                        {"查看"}
                                                    </button>
                                                    if inspection.result == "PENDING" {
                                                        <PermissionGuard resource="purchase_inspection" action="update">
                                                            <button
                                                                class="btn btn-sm btn-secondary"
                                                                onclick={link.callback(move |_| Msg::OpenEditModal(inspection_clone.clone()))}
                                                            >
                                                                {"编辑"}
                                                            </button>
                                                        </PermissionGuard>
                                                        <button
                                                            class="btn btn-sm btn-primary"
                                                            onclick={link.callback(move |_| Msg::ShowCompleteModal(id))}
                                                        >
                                                            {"完成检验"}
                                                        </button>
                                                        <PermissionGuard resource="purchase_inspection" action="delete">
                                                            <button
                                                                class="btn btn-sm btn-danger"
                                                                onclick={link.callback(move |_| Msg::DeleteInspection(id))}
                                                            >
                                                                {"删除"}
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
                            total={self.filtered_inspections.len() as u64}
                            on_page_change={link.callback(|page| Msg::PageChanged(page))}
                        />
                    </div>
                }

                if self.show_modal {
                    {self.render_form_modal(ctx)}
                }

                if self.show_complete_modal {
                    {self.render_complete_modal(ctx)}
                }

                if let Some(item) = &self.viewing_item {
                    {self.render_detail_modal(ctx, item)}
                }

                <ConfirmDialog
                    title={"确认删除".to_string()}
                    message={"确定要删除这个采购检验单吗？此操作不可撤销。".to_string()}
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

impl PurchaseInspectionPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_inspections = self.inspections.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_inspections = self.inspections.iter()
                .filter(|i| {
                    i.inspection_no.to_lowercase().contains(&keyword) ||
                    i.supplier_name.as_ref().map(|s| s.to_lowercase().contains(&keyword)).unwrap_or(false) ||
                    i.result.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_inspections(&self) -> Vec<PurchaseInspection> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_inspections[start..end.min(self.filtered_inspections.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_receipt_id = String::new();
        self.form_supplier_id = String::new();
        self.form_inspection_date = String::new();
        self.form_notes = String::new();
        self.form_error = None;
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let is_edit = self.modal_mode == ModalMode::Edit;
        let title = if is_edit { "编辑采购检验单" } else { "新建采购检验单" };

        let on_receipt_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormReceiptIdChanged(input.value()))
        });
        let on_supplier_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormSupplierIdChanged(input.value()))
        });
        let on_date_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormInspectionDateChanged(input.value()))
        });
        let on_notes_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormNotesChanged(input.value()))
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
                            <label>{"入库单ID *"}</label>
                            <input
                                type="number"
                                class="form-input"
                                value={self.form_receipt_id.clone()}
                                oninput={on_receipt_change}
                                placeholder="请输入入库单ID"
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
                        <div class="form-group">
                            <label>{"检验日期 *"}</label>
                            <input
                                type="date"
                                class="form-input"
                                value={self.form_inspection_date.clone()}
                                oninput={on_date_change}
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
                            {if is_edit { "保存修改" } else { "创建检验单" }}
                        </button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_complete_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        let on_pass_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormPassQuantityChanged(input.value()))
        });
        let on_reject_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormRejectQuantityChanged(input.value()))
        });
        let on_result_change = link.callback(|e: Event| {
            let select = e.target_unchecked_into::<web_sys::HtmlSelectElement>();
            Msg::FormInspectionResultChanged(select.value())
        });

        html! {
            <div class="modal-overlay" onclick={link.callback(|_| Msg::HideCompleteModal)}>
                <div class="modal-content" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <div class="modal-header">
                        <h3>{"完成检验"}</h3>
                        <button class="close-btn" onclick={link.callback(|_| Msg::HideCompleteModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        <div class="form-group">
                            <label>{"合格数量"}</label>
                            <input
                                type="number"
                                class="form-input"
                                value={self.form_pass_quantity.clone()}
                                oninput={on_pass_change}
                                placeholder="请输入合格数量"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"不合格数量"}</label>
                            <input
                                type="number"
                                class="form-input"
                                value={self.form_reject_quantity.clone()}
                                oninput={on_reject_change}
                                placeholder="请输入不合格数量"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"检验结果"}</label>
                            <select class="form-input" value={self.form_inspection_result.clone()} onchange={on_result_change}>
                                <option value="PASSED">{"合格"}</option>
                                <option value="FAILED">{"不合格"}</option>
                            </select>
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={link.callback(|_| Msg::HideCompleteModal)}>
                            {"取消"}
                        </button>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::SubmitComplete)}>
                            {"确认完成"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_detail_modal(&self, ctx: &Context<Self>, item: &PurchaseInspection) -> Html {
        html! {
            <div class="modal-overlay" onclick={ctx.link().callback(|_| Msg::CloseDetailModal)}>
                <div class="modal-content" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())} style="width: 800px; max-width: 90vw;">
                    <div class="modal-header">
                        <h2>{"采购检验单详情"}</h2>
                        <button class="close-btn" onclick={ctx.link().callback(|_| Msg::CloseDetailModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        <div class="detail-grid" style="display: grid; grid-template-columns: 1fr 1fr; gap: 1rem;">
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"检验单号: "}</span>
                                <span class="detail-value">{&item.inspection_no}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"供应商: "}</span>
                                <span class="detail-value">{item.supplier_name.as_deref().unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"检验日期: "}</span>
                                <span class="detail-value">{&item.inspection_date}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"检验结果: "}</span>
                                <span class="detail-value">{&item.result}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"合格数量: "}</span>
                                <span class="detail-value">{&item.qualified_quantity}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"不合格数量: "}</span>
                                <span class="detail-value">{&item.unqualified_quantity}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"不合格原因: "}</span>
                                <span class="detail-value">{item.unqualified_reason.as_deref().unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="detail-label" style="font-weight: bold; color: #666;">{"备注: "}</span>
                                <span class="detail-value">{item.remarks.as_deref().unwrap_or("-")}</span>
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
