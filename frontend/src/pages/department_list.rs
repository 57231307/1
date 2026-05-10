use crate::utils::permissions;
use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use crate::components::{
    confirm_dialog::ConfirmDialog,
    search_bar::SearchBar,
    pagination::Pagination,
    page_header::PageHeader,
    empty_state::EmptyState,
    loading_state::LoadingState,
};
use crate::models::department::{Department, CreateDepartmentRequest, UpdateDepartmentRequest};
use crate::services::department_service::DepartmentService;
use crate::services::crud_service::CrudService;
use crate::utils::toast_helper;

pub struct DepartmentListPage {
    departments: Vec<Department>,
    filtered_departments: Vec<Department>,
    total: u64,
    page: u64,
    page_size: u64,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_department: Option<Department>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    form_name: String,
    form_code: String,
    form_parent_id: String,
    form_manager: String,
    form_phone: String,
    form_error: Option<String>,
}

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    Create,
    Edit,
}

pub enum Msg {
    LoadData,
    DataLoaded(Vec<Department>),
    Error(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    CloseModal,
    OpenEditModal(Department),
    OpenDeleteConfirm(i32),
    CancelDelete,
    ConfirmDelete,
    Deleted,
    FormNameChanged(String),
    FormCodeChanged(String),
    FormParentIdChanged(String),
    FormManagerChanged(String),
    FormPhoneChanged(String),
    SubmitForm,
    FormSubmitted,
}

impl Component for DepartmentListPage {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::LoadData);
        Self {
            departments: Vec::new(),
            filtered_departments: Vec::new(),
            total: 0,
            page: 0,
            page_size: 10,
            loading: true,
            error: None,
            search_keyword: String::new(),
            show_modal: false,
            modal_mode: ModalMode::Create,
            editing_department: None,
            show_delete_confirm: false,
            deleting_id: None,
            form_name: String::new(),
            form_code: String::new(),
            form_parent_id: String::new(),
            form_manager: String::new(),
            form_phone: String::new(),
            form_error: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadData => {
                self.loading = true;
                self.error = None;
                let link = ctx.link().clone();
                spawn_local(async move {
                    match DepartmentService::list().await {
                        Ok(response) => link.send_message(Msg::DataLoaded(response.data)),
                        Err(e) => link.send_message(Msg::Error(e)),
                    }
                });
                false
            }
            Msg::DataLoaded(departments) => {
                self.loading = false;
                self.departments = departments.clone();
                self.apply_filter();
                true
            }
            Msg::Error(e) => {
                self.loading = false;
                self.error = Some(e);
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
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.editing_department = None;
                self.form_error = None;
                true
            }
            Msg::OpenEditModal(department) => {
                self.form_name = department.name.clone();
                self.form_code = department.code.clone();
                self.form_parent_id = department.parent_id.map(|id| id.to_string()).unwrap_or_default();
                self.form_manager = department.manager.clone().unwrap_or_default();
                self.form_phone = department.phone.clone().unwrap_or_default();
                self.editing_department = Some(department);
                self.modal_mode = ModalMode::Edit;
                self.show_modal = true;
                true
            }
            Msg::OpenDeleteConfirm(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::CancelDelete => {
                self.show_delete_confirm = false;
                self.deleting_id = None;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match DepartmentService::delete(id).await {
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
            Msg::Deleted => {
                self.show_delete_confirm = false;
                self.deleting_id = None;
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::FormNameChanged(v) => { self.form_name = v; true }
            Msg::FormCodeChanged(v) => { self.form_code = v; true }
            Msg::FormParentIdChanged(v) => { self.form_parent_id = v; true }
            Msg::FormManagerChanged(v) => { self.form_manager = v; true }
            Msg::FormPhoneChanged(v) => { self.form_phone = v; true }
            Msg::SubmitForm => {
                if self.form_name.is_empty() {
                    self.form_error = Some("部门名称不能为空".to_string());
                    return true;
                }
                if self.form_code.is_empty() {
                    self.form_error = Some("部门编码不能为空".to_string());
                    return true;
                }

                let name = self.form_name.clone();
                let code = self.form_code.clone();
                let parent_id = self.form_parent_id.parse().ok();
                let manager = if self.form_manager.is_empty() { None } else { Some(self.form_manager.clone()) };
                let phone = if self.form_phone.is_empty() { None } else { Some(self.form_phone.clone()) };

                let link = ctx.link().clone();

                if self.modal_mode == ModalMode::Edit {
                    if let Some(department) = &self.editing_department {
                        let id = department.id;
                        spawn_local(async move {
                            let req = UpdateDepartmentRequest {
                                name: Some(name),
                                code: Some(code),
                                parent_id,
                                manager,
                                phone,
                            };
                            match DepartmentService::update(id, req).await {
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
                        let req = CreateDepartmentRequest {
                            name,
                            code,
                            parent_id,
                            manager,
                            phone,
                        };
                        match DepartmentService::create(req).await {
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
                self.editing_department = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadData);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="department-list-page">
                <PageHeader title={"部门管理".to_string()} subtitle={Some("管理所有部门信息".to_string())}>
                    <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                        {"+ 新建部门"}
                    </button>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索部门名称或编码...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载部门数据...".to_string()} />
                } else if let Some(error) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{error}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadData)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_departments.is_empty() {
                    <EmptyState
                        icon={"🏢".to_string()}
                        title={"暂无部门数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一个部门".to_string()
                        } else {
                            "没有匹配搜索条件的部门".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"ID"}</th>
                                    <th>{"部门编码"}</th>
                                    <th>{"部门名称"}</th>
                                    <th>{"负责人"}</th>
                                    <th>{"联系电话"}</th>
                                    <th>{"上级部门"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_departments().iter().map(|department| {
                                    let department_clone = department.clone();
                                    let id = department.id;
                                    html! {
                                        <tr>
                                            <td>{department.id}</td>
                                            <td>{&department.code}</td>
                                            <td>{&department.name}</td>
                                            <td>{department.manager.clone().unwrap_or_else(|| "-".to_string())}</td>
                                            <td>{department.phone.clone().unwrap_or_else(|| "-".to_string())}</td>
                                            <td>{department.parent_id.map(|id| id.to_string()).unwrap_or_else(|| "-".to_string())}</td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-secondary"
                                                        onclick={link.callback(move |_| Msg::OpenEditModal(department_clone.clone()))}
                                                    >
                                                        {"编辑"}
                                                    </button>
                                                    <button
                                                        class="btn btn-sm btn-danger"
                                                        onclick={link.callback(move |_| Msg::OpenDeleteConfirm(id))}
                                                    >
                                                        {"删除"}
                                                    </button>
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
                            total={self.filtered_departments.len() as u64}
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
                    message={"确定要删除这个部门吗？此操作不可撤销。".to_string()}
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

impl DepartmentListPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_departments = self.departments.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_departments = self.departments.iter()
                .filter(|d| {
                    d.name.to_lowercase().contains(&keyword) ||
                    d.code.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
        }
        self.total = self.filtered_departments.len() as u64;
    }

    fn paginated_departments(&self) -> Vec<Department> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_departments[start..end.min(self.filtered_departments.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_name = String::new();
        self.form_code = String::new();
        self.form_parent_id = String::new();
        self.form_manager = String::new();
        self.form_phone = String::new();
        self.form_error = None;
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let is_edit = self.modal_mode == ModalMode::Edit;
        let title = if is_edit { "编辑部门" } else { "新建部门" };

        let on_name_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormNameChanged(input.value()))
        });
        let on_code_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormCodeChanged(input.value()))
        });
        let on_parent_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormParentIdChanged(input.value()))
        });
        let on_manager_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormManagerChanged(input.value()))
        });
        let on_phone_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormPhoneChanged(input.value()))
        });

        html! {
            <div class="modal-overlay" onclick={link.callback(|_| Msg::CloseModal)}>
                <div class="modal-content" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <div class="modal-header">
                        <h3>{title}</h3>
                        <button class="close-btn" onclick={link.callback(|_| Msg::CloseModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        if let Some(error) = &self.form_error {
                            <div class="form-error">{error}</div>
                        }
                        <div class="form-group">
                            <label>{"部门名称 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_name.clone()}
                                oninput={on_name_change}
                                placeholder="请输入部门名称"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"部门编码 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_code.clone()}
                                oninput={on_code_change}
                                placeholder="请输入部门编码"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"上级部门ID"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_parent_id.clone()}
                                oninput={on_parent_change}
                                placeholder="请输入上级部门ID（可选）"
                            />
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"负责人"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_manager.clone()}
                                    oninput={on_manager_change}
                                    placeholder="请输入负责人姓名"
                                />
                            </div>
                            <div class="form-group">
                                <label>{"联系电话"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_phone.clone()}
                                    oninput={on_phone_change}
                                    placeholder="请输入联系电话"
                                />
                            </div>
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={link.callback(|_| Msg::CloseModal)}>
                            {"取消"}
                        </button>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::SubmitForm)}>
                            {if is_edit { "保存修改" } else { "创建部门" }}
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}
