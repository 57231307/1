// 染色批次管理页面

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
use crate::models::dye_batch::{
    DyeBatch, DyeBatchQuery, CreateDyeBatchRequest, UpdateDyeBatchRequest, CompleteDyeBatchRequest,
};
use crate::services::dye_batch_service::DyeBatchService;
use crate::services::crud_service::CrudService;

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    Create,
    Edit,
    Complete,
}

pub struct DyeBatchPage {
    batches: Vec<DyeBatch>,
    filtered_batches: Vec<DyeBatch>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_batch: Option<DyeBatch>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    // 表单字段
    form_batch_no: String,
    form_color_code: String,
    form_color_name: String,
    form_fabric_type: String,
    form_weight_kg: String,
    form_quality_grade: String,
    form_remarks: String,
    // 完成表单字段
    form_complete_quality_grade: String,
    form_complete_remarks: String,
    form_error: Option<String>,
}

pub enum Msg {
    LoadBatches,
    BatchesLoaded(Vec<DyeBatch>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    OpenEditModal(DyeBatch),
    OpenCompleteModal(DyeBatch),
    CloseModal,
    SubmitForm,
    SubmitComplete,
    FormSubmitted,
    DeleteBatch(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    // 表单字段变更
    FormBatchNoChanged(String),
    FormColorCodeChanged(String),
    FormColorNameChanged(String),
    FormFabricTypeChanged(String),
    FormWeightKgChanged(String),
    FormQualityGradeChanged(String),
    FormRemarksChanged(String),
    FormCompleteQualityGradeChanged(String),
    FormCompleteRemarksChanged(String),
}

impl Component for DyeBatchPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            batches: Vec::new(),
            filtered_batches: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            show_modal: false,
            modal_mode: ModalMode::Create,
            editing_batch: None,
            show_delete_confirm: false,
            deleting_id: None,
            form_batch_no: String::new(),
            form_color_code: String::new(),
            form_color_name: String::new(),
            form_fabric_type: String::new(),
            form_weight_kg: String::new(),
            form_quality_grade: String::new(),
            form_remarks: String::new(),
            form_complete_quality_grade: String::new(),
            form_complete_remarks: String::new(),
            form_error: None,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadBatches);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadBatches => {
                self.loading = true;
                self.error = None;
                let query = DyeBatchQuery {
                    page: Some(1),
                    page_size: Some(1000),
                    batch_no: None,
                    color_code: None,
                    status: None,
                    quality_grade: None,
                    start_date: None,
                    end_date: None,
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match DyeBatchService::list(query).await {
                        Ok(batches) => link.send_message(Msg::BatchesLoaded(batches.items)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::BatchesLoaded(batches) => {
                self.loading = false;
                self.batches = batches;
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
                self.editing_batch = None;
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(batch) => {
                self.form_batch_no = batch.batch_no.clone();
                self.form_color_code = batch.color_code.clone();
                self.form_color_name = batch.color_name.clone();
                self.form_fabric_type = batch.fabric_type.clone().unwrap_or_default();
                self.form_weight_kg = batch.weight_kg.clone().unwrap_or_default();
                self.form_quality_grade = batch.quality_grade.clone().unwrap_or_default();
                self.form_remarks = batch.remarks.clone().unwrap_or_default();
                self.form_error = None;
                self.editing_batch = Some(batch);
                self.modal_mode = ModalMode::Edit;
                self.show_modal = true;
                true
            }
            Msg::OpenCompleteModal(batch) => {
                self.editing_batch = Some(batch);
                self.form_complete_quality_grade = String::new();
                self.form_complete_remarks = String::new();
                self.form_error = None;
                self.modal_mode = ModalMode::Complete;
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.editing_batch = None;
                self.form_error = None;
                true
            }
            Msg::SubmitForm => {
                // 表单验证
                if self.form_batch_no.is_empty() {
                    self.form_error = Some("缸号不能为空".to_string());
                    return true;
                }
                if self.form_color_code.is_empty() {
                    self.form_error = Some("色号不能为空".to_string());
                    return true;
                }
                if self.form_color_name.is_empty() {
                    self.form_error = Some("颜色名称不能为空".to_string());
                    return true;
                }

                self.form_error = None;

                if self.modal_mode == ModalMode::Edit {
                    if let Some(batch) = &self.editing_batch {
                        let id = batch.id;
                        let req = UpdateDyeBatchRequest {
                            color_code: Some(self.form_color_code.clone()),
                            color_name: Some(self.form_color_name.clone()),
                            fabric_type: if self.form_fabric_type.is_empty() { None } else { Some(self.form_fabric_type.clone()) },
                            weight_kg: if self.form_weight_kg.is_empty() { None } else { Some(self.form_weight_kg.clone()) },
                            status: None,
                            completion_date: None,
                            quality_grade: if self.form_quality_grade.is_empty() { None } else { Some(self.form_quality_grade.clone()) },
                            remarks: if self.form_remarks.is_empty() { None } else { Some(self.form_remarks.clone()) },
                        };
                        let link = ctx.link().clone();
                        spawn_local(async move {
                            match DyeBatchService::update(id, req).await {
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
                    let req = CreateDyeBatchRequest {
                        batch_no: self.form_batch_no.clone(),
                        color_code: self.form_color_code.clone(),
                        color_name: self.form_color_name.clone(),
                        fabric_type: if self.form_fabric_type.is_empty() { None } else { Some(self.form_fabric_type.clone()) },
                        weight_kg: if self.form_weight_kg.is_empty() { None } else { Some(self.form_weight_kg.clone()) },
                        status: Some("待生产".to_string()),
                        production_date: None,
                        quality_grade: if self.form_quality_grade.is_empty() { None } else { Some(self.form_quality_grade.clone()) },
                        remarks: if self.form_remarks.is_empty() { None } else { Some(self.form_remarks.clone()) },
                        created_by: None,
                    };
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match DyeBatchService::create(req).await {
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
            Msg::SubmitComplete => {
                // 表单验证
                if self.form_complete_quality_grade.is_empty() {
                    self.form_error = Some("质量等级不能为空".to_string());
                    return true;
                }

                self.form_error = None;

                if let Some(batch) = &self.editing_batch {
                    let id = batch.id;
                    let req = CompleteDyeBatchRequest {
                        quality_grade: self.form_complete_quality_grade.clone(),
                        remarks: if self.form_complete_remarks.is_empty() { None } else { Some(self.form_complete_remarks.clone()) },
                    };
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match DyeBatchService::complete(id, req).await {
                            Ok(_) => {
                                toast_helper::show_success("完成操作成功");
                                link.send_message(Msg::FormSubmitted);
                            }
                            Err(e) => {
                                toast_helper::show_error(&format!("完成操作失败: {}", e));
                            }
                        }
                    });
                }
                false
            }
            Msg::FormSubmitted => {
                self.show_modal = false;
                self.editing_batch = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadBatches);
                false
            }
            Msg::DeleteBatch(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match DyeBatchService::delete(id).await {
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
                ctx.link().send_message(Msg::LoadBatches);
                false
            }
            Msg::FormBatchNoChanged(v) => { self.form_batch_no = v; true }
            Msg::FormColorCodeChanged(v) => { self.form_color_code = v; true }
            Msg::FormColorNameChanged(v) => { self.form_color_name = v; true }
            Msg::FormFabricTypeChanged(v) => { self.form_fabric_type = v; true }
            Msg::FormWeightKgChanged(v) => { self.form_weight_kg = v; true }
            Msg::FormQualityGradeChanged(v) => { self.form_quality_grade = v; true }
            Msg::FormRemarksChanged(v) => { self.form_remarks = v; true }
            Msg::FormCompleteQualityGradeChanged(v) => { self.form_complete_quality_grade = v; true }
            Msg::FormCompleteRemarksChanged(v) => { self.form_complete_remarks = v; true }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="dye-batch-page">
                <PageHeader title={"染色批次管理".to_string()} subtitle={Some("管理染色缸号与生产批次".to_string())}>
                    <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                        {"+ 新建缸号"}
                    </button>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索缸号或色号...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载染色批次数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadBatches)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_batches.is_empty() {
                    <EmptyState
                        icon={"🏭".to_string()}
                        title={"暂无染色批次数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一个缸号".to_string()
                        } else {
                            "没有匹配搜索条件的缸号".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"缸号"}</th>
                                    <th>{"色号"}</th>
                                    <th>{"颜色名称"}</th>
                                    <th>{"面料类型"}</th>
                                    <th class="numeric">{"重量(kg)"}</th>
                                    <th>{"状态"}</th>
                                    <th>{"质量等级"}</th>
                                    <th>{"生产日期"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_batches().iter().map(|batch| {
                                    let batch_clone = batch.clone();
                                    let batch_clone2 = batch.clone();
                                    let batch_clone3 = batch.clone();
                                    let batch_id = batch.id;
                                    let is_completed = batch.status == "已完成";
                                    html! {
                                        <tr>
                                            <td>{&batch.batch_no}</td>
                                            <td>{&batch.color_code}</td>
                                            <td>{&batch.color_name}</td>
                                            <td>{batch.fabric_type.as_deref().unwrap_or("-")}</td>
                                            <td class="numeric">{batch.weight_kg.as_ref().unwrap_or(&"-".to_string())}</td>
                                            <td>
                                                <span class={format!("status-badge status-{}", batch.status)}>
                                                    {&batch.status}
                                                </span>
                                            </td>
                                            <td>{batch.quality_grade.as_deref().unwrap_or("-")}</td>
                                            <td>{batch.production_date.as_deref().unwrap_or("-")}</td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    if !is_completed {
                                                        <button
                                                            class="btn btn-sm btn-success"
                                                            onclick={link.callback(move |_| Msg::OpenCompleteModal(batch_clone.clone()))}
                                                        >
                                                            {"完成"}
                                                        </button>
                                                    }
                                                    <button
                                                        class="btn btn-sm btn-secondary"
                                                        onclick={link.callback(move |_| Msg::OpenEditModal(batch_clone2.clone()))}
                                                    >
                                                        {"编辑"}
                                                    </button>
                                                    <PermissionGuard resource="dye_batch" action="delete">
                                                        <button
                                                            class="btn btn-sm btn-danger"
                                                            onclick={link.callback(move |_| Msg::DeleteBatch(batch_id))}
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
                            total={self.filtered_batches.len() as u64}
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
                    message={"确定要删除这个缸号吗？此操作不可撤销。".to_string()}
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

impl DyeBatchPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_batches = self.batches.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_batches = self.batches.iter()
                .filter(|b| {
                    b.batch_no.to_lowercase().contains(&keyword) ||
                    b.color_code.to_lowercase().contains(&keyword) ||
                    b.color_name.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_batches(&self) -> Vec<DyeBatch> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_batches[start..end.min(self.filtered_batches.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_batch_no = String::new();
        self.form_color_code = String::new();
        self.form_color_name = String::new();
        self.form_fabric_type = String::new();
        self.form_weight_kg = String::new();
        self.form_quality_grade = String::new();
        self.form_remarks = String::new();
        self.form_complete_quality_grade = String::new();
        self.form_complete_remarks = String::new();
        self.form_error = None;
    }

    fn render_modal(&self, ctx: &Context<Self>) -> Html {
        match self.modal_mode {
            ModalMode::Complete => self.render_complete_modal(ctx),
            _ => self.render_form_modal(ctx),
        }
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let is_edit = self.modal_mode == ModalMode::Edit;
        let title = if is_edit { "编辑缸号" } else { "新建缸号" };

        let on_batch_no_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormBatchNoChanged(input.value()))
        });
        let on_color_code_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormColorCodeChanged(input.value()))
        });
        let on_color_name_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormColorNameChanged(input.value()))
        });
        let on_fabric_type_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormFabricTypeChanged(input.value()))
        });
        let on_weight_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormWeightKgChanged(input.value()))
        });
        let on_quality_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormQualityGradeChanged(input.value()))
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
                            <label>{"缸号 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_batch_no.clone()}
                                oninput={on_batch_no_change}
                                placeholder="请输入缸号"
                                disabled={is_edit}
                            />
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"色号 *"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_color_code.clone()}
                                    oninput={on_color_code_change}
                                    placeholder="请输入色号"
                                />
                            </div>
                            <div class="form-group">
                                <label>{"颜色名称 *"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_color_name.clone()}
                                    oninput={on_color_name_change}
                                    placeholder="请输入颜色名称"
                                />
                            </div>
                        </div>
                        <div class="form-group">
                            <label>{"面料类型"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_fabric_type.clone()}
                                oninput={on_fabric_type_change}
                                placeholder="请输入面料类型"
                            />
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"重量(kg)"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_weight_kg.clone()}
                                    oninput={on_weight_change}
                                    placeholder="请输入重量"
                                />
                            </div>
                            <div class="form-group">
                                <label>{"质量等级"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_quality_grade.clone()}
                                    oninput={on_quality_change}
                                    placeholder="如：A级、B级"
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
                            {if is_edit { "保存修改" } else { "创建缸号" }}
                        </button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_complete_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        let on_quality_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormCompleteQualityGradeChanged(input.value()))
        });
        let on_remarks_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormCompleteRemarksChanged(input.value()))
        });

        html! {
            <div class="modal-overlay" onclick={link.callback(|_| Msg::CloseModal)}>
                <div class="modal-content" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <div class="modal-header">
                        <h3>{"完成缸号"}</h3>
                        <button class="close-btn" onclick={link.callback(|_| Msg::CloseModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        if let Some(err) = &self.form_error {
                            <div class="form-error">{err}</div>
                        }
                        <div class="form-group">
                            <label>{"质量等级 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_complete_quality_grade.clone()}
                                oninput={on_quality_change}
                                placeholder="请输入质量等级"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"备注"}</label>
                            <textarea
                                class="form-input"
                                value={self.form_complete_remarks.clone()}
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
                        <button class="btn btn-success" onclick={link.callback(|_| Msg::SubmitComplete)}>
                            {"确认完成"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}
