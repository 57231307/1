// 坯布管理页面

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
use crate::models::greige_fabric::{
    CreateGreigeFabricRequest, UpdateGreigeFabricRequest,
    GreigeFabric, GreigeFabricQuery,
};
use crate::services::greige_fabric_service::GreigeFabricService;
use crate::services::crud_service::CrudService;

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    Create,
    Edit,
}

pub struct GreigeFabricPage {
    fabrics: Vec<GreigeFabric>,
    filtered_fabrics: Vec<GreigeFabric>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_fabric: Option<GreigeFabric>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    // 表单字段
    form_fabric_no: String,
    form_fabric_name: String,
    form_fabric_type: String,
    form_width_cm: String,
    form_weight_kg: String,
    form_length_m: String,
    form_quality_grade: String,
    form_remarks: String,
    form_error: Option<String>,
}

pub enum Msg {
    LoadFabrics,
    FabricsLoaded(Vec<GreigeFabric>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    OpenEditModal(GreigeFabric),
    CloseModal,
    SubmitForm,
    FormSubmitted,
    DeleteFabric(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    // 表单字段变更
    FormFabricNoChanged(String),
    FormFabricNameChanged(String),
    FormFabricTypeChanged(String),
    FormWidthCmChanged(String),
    FormWeightKgChanged(String),
    FormLengthMChanged(String),
    FormQualityGradeChanged(String),
    FormRemarksChanged(String),
}

impl Component for GreigeFabricPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            fabrics: Vec::new(),
            filtered_fabrics: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            show_modal: false,
            modal_mode: ModalMode::Create,
            editing_fabric: None,
            show_delete_confirm: false,
            deleting_id: None,
            form_fabric_no: String::new(),
            form_fabric_name: String::new(),
            form_fabric_type: String::new(),
            form_width_cm: String::new(),
            form_weight_kg: String::new(),
            form_length_m: String::new(),
            form_quality_grade: String::new(),
            form_remarks: String::new(),
            form_error: None,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadFabrics);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadFabrics => {
                self.loading = true;
                self.error = None;
                let query = GreigeFabricQuery {
                    page: Some(1),
                    page_size: Some(1000),
                    fabric_no: None,
                    fabric_name: None,
                    fabric_type: None,
                    supplier_id: None,
                    warehouse_id: None,
                    status: None,
                    quality_grade: None,
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match GreigeFabricService::list(query).await {
                        Ok(fabrics) => link.send_message(Msg::FabricsLoaded(fabrics.items)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::FabricsLoaded(fabrics) => {
                self.loading = false;
                self.fabrics = fabrics;
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
                self.editing_fabric = None;
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(fabric) => {
                self.form_fabric_no = fabric.fabric_no.clone();
                self.form_fabric_name = fabric.fabric_name.clone();
                self.form_fabric_type = fabric.fabric_type.clone();
                self.form_width_cm = fabric.width_cm.clone().unwrap_or_default();
                self.form_weight_kg = fabric.weight_kg.clone().unwrap_or_default();
                self.form_length_m = fabric.length_m.clone().unwrap_or_default();
                self.form_quality_grade = fabric.quality_grade.clone().unwrap_or_default();
                self.form_remarks = fabric.remarks.clone().unwrap_or_default();
                self.form_error = None;
                self.editing_fabric = Some(fabric);
                self.modal_mode = ModalMode::Edit;
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.editing_fabric = None;
                self.form_error = None;
                true
            }
            Msg::SubmitForm => {
                // 表单验证
                if self.form_fabric_no.is_empty() {
                    self.form_error = Some("坯布编号不能为空".to_string());
                    return true;
                }
                if self.form_fabric_name.is_empty() {
                    self.form_error = Some("坯布名称不能为空".to_string());
                    return true;
                }
                if self.form_fabric_type.is_empty() {
                    self.form_error = Some("坯布类型不能为空".to_string());
                    return true;
                }

                self.form_error = None;

                if self.modal_mode == ModalMode::Edit {
                    if let Some(fabric) = &self.editing_fabric {
                        let id = fabric.id;
                        let req = UpdateGreigeFabricRequest {
                            fabric_name: Some(self.form_fabric_name.clone()),
                            fabric_type: Some(self.form_fabric_type.clone()),
                            color_code: None,
                            width_cm: if self.form_width_cm.is_empty() { None } else { Some(self.form_width_cm.clone()) },
                            weight_kg: if self.form_weight_kg.is_empty() { None } else { Some(self.form_weight_kg.clone()) },
                            length_m: if self.form_length_m.is_empty() { None } else { Some(self.form_length_m.clone()) },
                            supplier_id: None,
                            batch_no: None,
                            warehouse_id: None,
                            location: None,
                            status: None,
                            quality_grade: if self.form_quality_grade.is_empty() { None } else { Some(self.form_quality_grade.clone()) },
                            remarks: if self.form_remarks.is_empty() { None } else { Some(self.form_remarks.clone()) },
                        };
                        let link = ctx.link().clone();
                        spawn_local(async move {
                            match GreigeFabricService::update(id, req).await {
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
                    let req = CreateGreigeFabricRequest {
                        fabric_no: self.form_fabric_no.clone(),
                        fabric_name: self.form_fabric_name.clone(),
                        fabric_type: self.form_fabric_type.clone(),
                        color_code: None,
                        width_cm: if self.form_width_cm.is_empty() { None } else { Some(self.form_width_cm.clone()) },
                        weight_kg: if self.form_weight_kg.is_empty() { None } else { Some(self.form_weight_kg.clone()) },
                        length_m: if self.form_length_m.is_empty() { None } else { Some(self.form_length_m.clone()) },
                        supplier_id: None,
                        batch_no: None,
                        warehouse_id: None,
                        location: None,
                        status: Some("在库".to_string()),
                        quality_grade: if self.form_quality_grade.is_empty() { None } else { Some(self.form_quality_grade.clone()) },
                        purchase_date: None,
                        remarks: if self.form_remarks.is_empty() { None } else { Some(self.form_remarks.clone()) },
                        created_by: None,
                    };
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match GreigeFabricService::create(req).await {
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
                self.editing_fabric = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadFabrics);
                false
            }
            Msg::DeleteFabric(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match GreigeFabricService::delete(id).await {
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
                ctx.link().send_message(Msg::LoadFabrics);
                false
            }
            Msg::FormFabricNoChanged(v) => { self.form_fabric_no = v; true }
            Msg::FormFabricNameChanged(v) => { self.form_fabric_name = v; true }
            Msg::FormFabricTypeChanged(v) => { self.form_fabric_type = v; true }
            Msg::FormWidthCmChanged(v) => { self.form_width_cm = v; true }
            Msg::FormWeightKgChanged(v) => { self.form_weight_kg = v; true }
            Msg::FormLengthMChanged(v) => { self.form_length_m = v; true }
            Msg::FormQualityGradeChanged(v) => { self.form_quality_grade = v; true }
            Msg::FormRemarksChanged(v) => { self.form_remarks = v; true }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="greige-fabric-page">
                <PageHeader title={"坯布管理".to_string()} subtitle={Some("管理坯布库存信息".to_string())}>
                    <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                        {"+ 新建坯布"}
                    </button>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索坯布编号或名称...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载坯布数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadFabrics)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_fabrics.is_empty() {
                    <EmptyState
                        icon={"📦".to_string()}
                        title={"暂无坯布数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一个坯布".to_string()
                        } else {
                            "没有匹配搜索条件的坯布".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"坯布编号"}</th>
                                    <th>{"坯布名称"}</th>
                                    <th>{"坯布类型"}</th>
                                    <th class="numeric">{"幅宽(cm)"}</th>
                                    <th class="numeric">{"重量(kg)"}</th>
                                    <th class="numeric">{"长度(m)"}</th>
                                    <th>{"状态"}</th>
                                    <th>{"质量等级"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_fabrics().iter().map(|fabric| {
                                    let fabric_clone = fabric.clone();
                                    let fabric_id = fabric.id;
                                    html! {
                                        <tr>
                                            <td>{&fabric.fabric_no}</td>
                                            <td>{&fabric.fabric_name}</td>
                                            <td>{&fabric.fabric_type}</td>
                                            <td class="numeric">{fabric.width_cm.as_ref().unwrap_or(&"-".to_string())}</td>
                                            <td class="numeric">{fabric.weight_kg.as_ref().unwrap_or(&"-".to_string())}</td>
                                            <td class="numeric">{fabric.length_m.as_ref().unwrap_or(&"-".to_string())}</td>
                                            <td>
                                                <span class={format!("status-badge status-{}", fabric.status)}>
                                                    {&fabric.status}
                                                </span>
                                            </td>
                                            <td>{fabric.quality_grade.as_deref().unwrap_or("-")}</td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-secondary"
                                                        onclick={link.callback(move |_| Msg::OpenEditModal(fabric_clone.clone()))}
                                                    >
                                                        {"编辑"}
                                                    </button>
                                                    <PermissionGuard resource="greige_fabric" action="delete">
                                                        <button
                                                            class="btn btn-sm btn-danger"
                                                            onclick={link.callback(move |_| Msg::DeleteFabric(fabric_id))}
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
                            total={self.filtered_fabrics.len() as u64}
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
                    message={"确定要删除这个坯布吗？此操作不可撤销。".to_string()}
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

impl GreigeFabricPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_fabrics = self.fabrics.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_fabrics = self.fabrics.iter()
                .filter(|f| {
                    f.fabric_no.to_lowercase().contains(&keyword) ||
                    f.fabric_name.to_lowercase().contains(&keyword) ||
                    f.fabric_type.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_fabrics(&self) -> Vec<GreigeFabric> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_fabrics[start..end.min(self.filtered_fabrics.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_fabric_no = String::new();
        self.form_fabric_name = String::new();
        self.form_fabric_type = String::new();
        self.form_width_cm = String::new();
        self.form_weight_kg = String::new();
        self.form_length_m = String::new();
        self.form_quality_grade = String::new();
        self.form_remarks = String::new();
        self.form_error = None;
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let is_edit = self.modal_mode == ModalMode::Edit;
        let title = if is_edit { "编辑坯布" } else { "新建坯布" };

        let on_fabric_no_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormFabricNoChanged(input.value()))
        });
        let on_fabric_name_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormFabricNameChanged(input.value()))
        });
        let on_fabric_type_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormFabricTypeChanged(input.value()))
        });
        let on_width_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormWidthCmChanged(input.value()))
        });
        let on_weight_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormWeightKgChanged(input.value()))
        });
        let on_length_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormLengthMChanged(input.value()))
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
                            <label>{"坯布编号 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_fabric_no.clone()}
                                oninput={on_fabric_no_change}
                                placeholder="请输入坯布编号"
                                disabled={is_edit}
                            />
                        </div>
                        <div class="form-group">
                            <label>{"坯布名称 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_fabric_name.clone()}
                                oninput={on_fabric_name_change}
                                placeholder="请输入坯布名称"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"坯布类型 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_fabric_type.clone()}
                                oninput={on_fabric_type_change}
                                placeholder="如：棉布、涤纶、混纺"
                            />
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"幅宽(cm)"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_width_cm.clone()}
                                    oninput={on_width_change}
                                    placeholder="请输入幅宽"
                                />
                            </div>
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
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"长度(m)"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_length_m.clone()}
                                    oninput={on_length_change}
                                    placeholder="请输入长度"
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
                            {if is_edit { "保存修改" } else { "创建坯布" }}
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}
