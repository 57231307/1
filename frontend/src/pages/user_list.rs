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
use crate::models::user::{User, CreateUserRequest, UserListResponse};
use crate::services::user_service::UserService;
use crate::services::crud_service::CrudService;
use crate::utils::toast_helper;

pub struct UserListPage {
    users: Vec<User>,
    filtered_users: Vec<User>,
    total: u64,
    page: u64,
    page_size: u64,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    show_create_modal: bool,
    show_edit_modal: bool,
    editing_user: Option<User>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    form_username: String,
    form_password: String,
    form_email: String,
    form_phone: String,
    form_role_id: String,
    form_department_id: String,
    form_is_active: bool,
    form_error: Option<String>,
}

pub enum Msg {
    LoadData,
    DataLoaded(Vec<User>, u64),
    Error(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    CloseModal,
    OpenEditModal(User),
    OpenDeleteConfirm(i32),
    CancelDelete,
    ConfirmDelete,
    Deleted,
    FormUsernameChanged(String),
    FormPasswordChanged(String),
    FormEmailChanged(String),
    FormPhoneChanged(String),
    FormRoleIdChanged(String),
    FormDepartmentIdChanged(String),
    FormIsActiveChanged,
    SubmitForm,
    FormSubmitted,
}

impl Component for UserListPage {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::LoadData);
        Self {
            users: Vec::new(),
            filtered_users: Vec::new(),
            total: 0,
            page: 0,
            page_size: 10,
            loading: true,
            error: None,
            search_keyword: String::new(),
            show_create_modal: false,
            show_edit_modal: false,
            editing_user: None,
            show_delete_confirm: false,
            deleting_id: None,
            form_username: String::new(),
            form_password: String::new(),
            form_email: String::new(),
            form_phone: String::new(),
            form_role_id: String::new(),
            form_department_id: String::new(),
            form_is_active: true,
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
                    match UserService::list_with_query(&crate::services::user_service::UserQuery { page: 0, page_size: 1000 }).await {
                        Ok(response) => link.send_message(Msg::DataLoaded(response.users, response.total)),
                        Err(e) => link.send_message(Msg::Error(e)),
                    }
                });
                false
            }
            Msg::DataLoaded(users, total) => {
                self.loading = false;
                self.users = users.clone();
                self.total = total;
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
                self.show_create_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_create_modal = false;
                self.show_edit_modal = false;
                self.editing_user = None;
                self.form_error = None;
                true
            }
            Msg::OpenEditModal(user) => {
                self.form_username = user.username.clone();
                self.form_email = user.email.clone().unwrap_or_default();
                self.form_phone = user.phone.clone().unwrap_or_default();
                self.form_role_id = user.role_id.map(|id| id.to_string()).unwrap_or_default();
                self.form_department_id = user.department_id.map(|id| id.to_string()).unwrap_or_default();
                self.form_is_active = user.is_active;
                self.editing_user = Some(user);
                self.show_edit_modal = true;
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
                        match UserService::delete(id).await {
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
            Msg::FormUsernameChanged(v) => { self.form_username = v; true }
            Msg::FormPasswordChanged(v) => { self.form_password = v; true }
            Msg::FormEmailChanged(v) => { self.form_email = v; true }
            Msg::FormPhoneChanged(v) => { self.form_phone = v; true }
            Msg::FormRoleIdChanged(v) => { self.form_role_id = v; true }
            Msg::FormDepartmentIdChanged(v) => { self.form_department_id = v; true }
            Msg::FormIsActiveChanged => { self.form_is_active = !self.form_is_active; true }
            Msg::SubmitForm => {
                if self.form_username.is_empty() {
                    self.form_error = Some("用户名不能为空".to_string());
                    return true;
                }
                if !self.show_edit_modal && self.form_password.is_empty() {
                    self.form_error = Some("密码不能为空".to_string());
                    return true;
                }

                let username = self.form_username.clone();
                let password = self.form_password.clone();
                let email = if self.form_email.is_empty() { None } else { Some(self.form_email.clone()) };
                let phone = if self.form_phone.is_empty() { None } else { Some(self.form_phone.clone()) };
                let role_id = self.form_role_id.parse().ok();
                let department_id = self.form_department_id.parse().ok();
                let is_active = self.form_is_active;

                let link = ctx.link().clone();

                if self.show_edit_modal {
                    if let Some(user) = &self.editing_user {
                        let id = user.id;
                        spawn_local(async move {
                            let req = crate::models::user::CreateUserRequest {
                                username: username.clone(),
                                password: if password.is_empty() { "unchanged".to_string() } else { password },
                                email,
                                phone,
                                role_id,
                                department_id,
                            };
                            match UserService::update(id, req).await {
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
                        let req = CreateUserRequest {
                            username,
                            password,
                            email,
                            phone,
                            role_id,
                            department_id,
                        };
                        match UserService::create(req).await {
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
                self.show_create_modal = false;
                self.show_edit_modal = false;
                self.editing_user = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadData);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="user-list-page">
                <PageHeader title={"用户管理".to_string()} subtitle={Some("管理系统用户信息".to_string())}>
                    <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                        {"+ 新建用户"}
                    </button>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索用户名、邮箱或手机号...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载用户数据...".to_string()} />
                } else if let Some(error) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{error}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadData)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_users.is_empty() {
                    <EmptyState
                        icon={"👥".to_string()}
                        title={"暂无用户数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一个用户".to_string()
                        } else {
                            "没有匹配搜索条件的用户".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"ID"}</th>
                                    <th>{"用户名"}</th>
                                    <th>{"邮箱"}</th>
                                    <th>{"手机号"}</th>
                                    <th>{"角色"}</th>
                                    <th>{"状态"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_users().iter().map(|user| {
                                    let user_clone = user.clone();
                                    let id = user.id;
                                    html! {
                                        <tr>
                                            <td>{user.id}</td>
                                            <td>{&user.username}</td>
                                            <td>{user.email.clone().unwrap_or_else(|| "-".to_string())}</td>
                                            <td>{user.phone.clone().unwrap_or_else(|| "-".to_string())}</td>
                                            <td>{user.role_id.map(|id| format!("角色 #{}", id)).unwrap_or_else(|| "-".to_string())}</td>
                                            <td>
                                                if user.is_active {
                                                    <span class="status-badge status-success">{"正常"}</span>
                                                } else {
                                                    <span class="status-badge status-error">{"禁用"}</span>
                                                }
                                            </td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-secondary"
                                                        onclick={link.callback(move |_| Msg::OpenEditModal(user_clone.clone()))}
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
                            total={self.filtered_users.len() as u64}
                            on_page_change={link.callback(|page| Msg::PageChanged(page))}
                        />
                    </div>
                }

                // 新建/编辑弹窗
                if self.show_create_modal || self.show_edit_modal {
                    {self.render_form_modal(ctx)}
                }

                // 删除确认对话框
                <ConfirmDialog
                    title={"确认删除".to_string()}
                    message={"确定要删除这个用户吗？此操作不可撤销。".to_string()}
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

impl UserListPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_users = self.users.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_users = self.users.iter()
                .filter(|u| {
                    u.username.to_lowercase().contains(&keyword) ||
                    u.email.as_ref().map(|e| e.to_lowercase().contains(&keyword)).unwrap_or(false) ||
                    u.phone.as_ref().map(|p| p.to_lowercase().contains(&keyword)).unwrap_or(false)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_users(&self) -> Vec<User> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_users[start..end.min(self.filtered_users.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_username = String::new();
        self.form_password = String::new();
        self.form_email = String::new();
        self.form_phone = String::new();
        self.form_role_id = String::new();
        self.form_department_id = String::new();
        self.form_is_active = true;
        self.form_error = None;
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let is_edit = self.show_edit_modal;
        let title = if is_edit { "编辑用户" } else { "新建用户" };

        let on_username_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormUsernameChanged(input.value()))
        });
        let on_password_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormPasswordChanged(input.value()))
        });
        let on_email_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormEmailChanged(input.value()))
        });
        let on_phone_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormPhoneChanged(input.value()))
        });
        let on_role_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormRoleIdChanged(input.value()))
        });
        let on_dept_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormDepartmentIdChanged(input.value()))
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
                            <label>{"用户名 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_username.clone()}
                                oninput={on_username_change}
                                placeholder="请输入用户名"
                            />
                        </div>
                        if !is_edit {
                            <div class="form-group">
                                <label>{"密码 *"}</label>
                                <input
                                    type="password"
                                    class="form-input"
                                    value={self.form_password.clone()}
                                    oninput={on_password_change}
                                    placeholder="请输入密码"
                                />
                            </div>
                        }
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"邮箱"}</label>
                                <input
                                    type="email"
                                    class="form-input"
                                    value={self.form_email.clone()}
                                    oninput={on_email_change}
                                    placeholder="请输入邮箱"
                                />
                            </div>
                            <div class="form-group">
                                <label>{"手机号"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_phone.clone()}
                                    oninput={on_phone_change}
                                    placeholder="请输入手机号"
                                />
                            </div>
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"角色ID"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_role_id.clone()}
                                    oninput={on_role_change}
                                    placeholder="请输入角色ID"
                                />
                            </div>
                            <div class="form-group">
                                <label>{"部门ID"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_department_id.clone()}
                                    oninput={on_dept_change}
                                    placeholder="请输入部门ID"
                                />
                            </div>
                        </div>
                        if is_edit {
                            <div class="form-group">
                                <label class="checkbox-label">
                                    <input
                                        type="checkbox"
                                        checked={self.form_is_active}
                                        onclick={link.callback(|_| Msg::FormIsActiveChanged)}
                                    />
                                    <span>{"启用账号"}</span>
                                </label>
                            </div>
                        }
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={link.callback(|_| Msg::CloseModal)}>
                            {"取消"}
                        </button>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::SubmitForm)}>
                            {if is_edit { "保存修改" } else { "创建用户" }}
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}
