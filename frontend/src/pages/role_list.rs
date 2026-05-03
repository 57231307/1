use crate::utils::permissions;
use yew::prelude::*;
use crate::components::permission_guard::PermissionGuard;
use yew_router::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen::JsCast;
use crate::app::Route;
use crate::services::api::ApiService;
use crate::services::crud_service::CrudService;
use crate::utils::storage::Storage;

#[derive(Debug, Clone, serde::Deserialize)]
struct Role {
    id: i32,
    name: String,
    code: String,
    description: Option<String>,
    is_system: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct RoleListResponse {
    roles: Vec<Role>,
    total: u64,
}

pub struct RoleListPage {
    roles: Vec<Role>,
    total: u64,
    is_loading: bool,
    error_message: Option<String>,
    show_create_modal: bool,
    show_edit_modal: bool,
    editing_role: Option<Role>,
    name: String,
    code: String,
    description: String,
    is_system: bool,
}

pub enum Msg {
    LoadRoles,
    RolesLoaded(Vec<Role>, u64),
    LoadError(String),
    LoadingChanged(bool),
    ShowCreateModal,
    CloseCreateModal,
    ShowEditModal(Role),
    CloseEditModal,
    CreateRole,
    UpdateRole,
    DeleteRole(i32),
    NameChanged(String),
    CodeChanged(String),
    DescriptionChanged(String),
    IsSystemChanged(bool),
    Logout,
}

impl Component for RoleListPage {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let mut this = Self {
            roles: Vec::new(),
            total: 0,
            is_loading: false,
            error_message: None,
            show_create_modal: false,
            show_edit_modal: false,
            editing_role: None,
            name: String::new(),
            code: String::new(),
            description: String::new(),
            is_system: false,
        };

        // 检查是否已登录
        if Storage::get_token().is_none() {
            if let Some(navigator) = ctx.link().navigator() {
                navigator.push(&Route::Login);
            }
            return this;
        } else {
            this.load_roles(ctx);
        }

        this
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadRoles => {
                self.is_loading = true;
                self.error_message = None;

                let link = ctx.link().clone();

                spawn_local(async move {
                    match ApiService::get::<RoleListResponse>("/roles").await {
                        Ok(response) => {
                            link.send_message(Msg::RolesLoaded(response.roles, response.total));
                        }
                        Err(error) => {
                            link.send_message(Msg::LoadError(error));
                        }
                    }
                });
                true
            }
            Msg::RolesLoaded(roles, total) => {
                self.is_loading = false;
                self.roles = roles;
                self.total = total;
                true
            }
            Msg::LoadError(error) => {
                self.is_loading = false;
                self.error_message = Some(error);
                true
            }
            Msg::LoadingChanged(loading) => {
                self.is_loading = loading;
                true
            }
            Msg::ShowCreateModal => {
                self.name = String::new();
                self.code = String::new();
                self.description = String::new();
                self.is_system = false;
                self.show_create_modal = true;
                true
            }
            Msg::CloseCreateModal => {
                self.show_create_modal = false;
                true
            }
            Msg::ShowEditModal(role) => {
                self.editing_role = Some(role.clone());
                self.name = role.name;
                self.code = role.code;
                self.description = role.description.unwrap_or_default();
                self.is_system = role.is_system;
                self.show_edit_modal = true;
                true
            }
            Msg::CloseEditModal => {
                self.show_edit_modal = false;
                self.editing_role = None;
                true
            }
            Msg::CreateRole => {
                let name = self.name.clone();
                let code = self.code.clone();
                let description = if self.description.is_empty() {
                    None
                } else {
                    Some(self.description.clone())
                };
                let is_system = self.is_system;

                let link = ctx.link().clone();

                spawn_local(async move {
                    let payload = serde_json::json!({
                        "name": name,
                        "code": code,
                        "description": description,
                        "is_system": is_system
                    });

                    match ApiService::post::<serde_json::Value, serde_json::Value>("/roles", &payload).await {
                        Ok(_) => {
                            link.send_message(Msg::CloseCreateModal);
                            link.send_message(Msg::LoadRoles);
                        }
                        Err(error) => {
                            link.send_message(Msg::LoadError(error));
                        }
                    }
                });
                true
            }
            Msg::UpdateRole => {
                if let Some(role) = self.editing_role.as_ref() {
                    let role_id = role.id;
                    let name = self.name.clone();
                    let code = self.code.clone();
                    let description = if self.description.is_empty() {
                        None
                    } else {
                        Some(self.description.clone())
                    };
                    let is_system = self.is_system;

                    let link = ctx.link().clone();

                    spawn_local(async move {
                        let payload = serde_json::json!({
                            "name": name,
                            "code": code,
                            "description": description,
                            "is_system": is_system
                        });

                        match ApiService::put::<serde_json::Value, serde_json::Value>(&format!("/roles/{}", role_id), &payload).await {
                            Ok(_) => {
                                link.send_message(Msg::CloseEditModal);
                                link.send_message(Msg::LoadRoles);
                            }
                            Err(error) => {
                                link.send_message(Msg::LoadError(error));
                            }
                        }
                    });
                }
                true
            }
            Msg::DeleteRole(id) => {
                let link = ctx.link().clone();

                spawn_local(async move {
                    if let Some(win) = web_sys::window() {
                        if win.confirm_with_message("确定要删除这个角色吗？").unwrap_or(false) {
                            match ApiService::delete(&format!("/roles/{}", id)).await {
                                Ok(_) => {
                                    link.send_message(Msg::LoadRoles);
                                }
                                Err(error) => {
                                    link.send_message(Msg::LoadError(error));
                                }
                            }
                        }
                    }
                });
                true
            }
            Msg::NameChanged(value) => {
                self.name = value;
                true
            }
            Msg::CodeChanged(value) => {
                self.code = value;
                true
            }
            Msg::DescriptionChanged(value) => {
                self.description = value;
                true
            }
            Msg::IsSystemChanged(value) => {
                self.is_system = value;
                true
            }
            Msg::Logout => {
                Storage::clear_all();
                if let Some(navigator) = ctx.link().navigator() {
                    navigator.push(&Route::Login);
                }
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        let on_logout = link.callback(|_| Msg::Logout);
        let on_create = link.callback(|_| Msg::ShowCreateModal);
        let on_refresh = link.callback(|_| Msg::LoadRoles);

        html! {
            <div class="role-list-page">
                <div class="header">
                    <h1>{"角色管理"}</h1>
                    <div class="header-actions">
                        <button class="refresh-button" onclick={on_refresh}>{"刷新"}</button>
                        <button class="create-button" onclick={on_create}>{"新建角色"}</button>
                        <button class="logout-button" onclick={on_logout}>{"退出登录"}</button>
                    </div>
                </div>

                if let Some(error) = &self.error_message {
                    <div class="error-message">{error}</div>
                }

                if self.is_loading {
                    <div class="loading">{"加载中..."}</div>
                } else {
                    <div class="role-table-container">
                        <table class="role-table">
                            <thead>
                                <tr>
                                    <th>{"ID"}</th>
                                    <th>{"角色名称"}</th>
                                    <th>{"角色编码"}</th>
                                    <th>{"描述"}</th>
                                    <th>{"系统角色"}</th>
                                    <th>{"创建时间"}</th>
                                    <th>{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.roles.iter().map(|role| {
                                    let role_edit = role.clone();
                                    let role_delete = role.clone();
                                    let on_edit = link.callback(move |_| Msg::ShowEditModal(role_edit.clone()));
                                    let on_delete = link.callback(move |_| Msg::DeleteRole(role_delete.id));
                                    
                                    html! {
                                        <tr>
                                            <td>{role.id}</td>
                                            <td>{&role.name}</td>
                                            <td>{&role.code}</td>
                                            <td>
                                                if let Some(desc) = &role.description {
                                                    {desc}
                                                } else {
                                                    {"-"}
                                                }
                                            </td>
                                            <td>
                                                if role.is_system {
                                                    <span class="badge-system">{"是"}</span>
                                                } else {
                                                    <span class="badge-user">{"否"}</span>
                                                }
                                            </td>
                                            <td>{&role.created_at}</td>
                                            <td class="actions">
                                                <button class="edit-btn" onclick={on_edit}>{"编辑"}</button>
                                                <button class="delete-btn" onclick={on_delete}>{"删除"}</button>
                                            </td>
                                        </tr>
                                    }
                                })}
                            </tbody>
                        </table>

                        <div class="summary">
                            {format!("共 {} 个角色", self.total)}
                        </div>
                    </div>
                }

                if self.show_create_modal {
                    <div class="modal-overlay" onclick={link.callback(|_| Msg::CloseCreateModal)}>
                        <div class="modal" onclick={|e: MouseEvent| e.stop_propagation()}>
                            <div class="modal-header">
                                <h2>{"新建角色"}</h2>
                                <button class="close-btn" onclick={link.callback(|_| Msg::CloseCreateModal)}>{"×"}</button>
                            </div>
                            <div class="modal-body">
                                <div class="form-group">
                                    <label>{"角色名称"}</label>
                                    <input
                                        type="text"
                                        value={self.name.clone()}
                                        onchange={link.batch_callback(|e: Event| -> Option<crate::pages::role_list::Msg> {
                                            let input = e.target()?.dyn_into::<web_sys::HtmlInputElement>().ok()?;
                                            Some(Msg::NameChanged(input.value()))
                                        })}
                                        placeholder="请输入角色名称"
                                    />
                                </div>
                                <div class="form-group">
                                    <label>{"角色编码"}</label>
                                    <input
                                        type="text"
                                        value={self.code.clone()}
                                        onchange={link.batch_callback(|e: Event| -> Option<crate::pages::role_list::Msg> {
                                            let input = e.target()?.dyn_into::<web_sys::HtmlInputElement>().ok()?;
                                            Some(Msg::CodeChanged(input.value()))
                                        })}
                                        placeholder="请输入角色编码"
                                    />
                                </div>
                                <div class="form-group">
                                    <label>{"描述"}</label>
                                    <textarea
                                        value={self.description.clone()}
                                        onchange={link.batch_callback(|e: Event| -> Option<crate::pages::role_list::Msg> {
                                            let input = e.target()?.dyn_into::<web_sys::HtmlTextAreaElement>().ok()?;
                                            Some(Msg::DescriptionChanged(input.value()))
                                        })}
                                        placeholder="请输入角色描述"
                                    />
                                </div>
                                <div class="form-group">
                                    <label class="checkbox-label">
                                        <input
                                            type="checkbox"
                                            checked={self.is_system}
                                            onchange={link.batch_callback(|e: Event| -> Option<crate::pages::role_list::Msg> {
                                                let input = e.target()?.dyn_into::<web_sys::HtmlInputElement>().ok()?;
                                                Some(Msg::IsSystemChanged(input.checked()))
                                            })}
                                        />
                                        <span>{"系统角色"}</span>
                                    </label>
                                </div>
                            </div>
                            <div class="modal-footer">
                                <button class="cancel-btn" onclick={link.callback(|_| Msg::CloseCreateModal)}>{"取消"}</button>
                                <PermissionGuard resource="role_list" action="create">
<button class="confirm-btn" onclick={link.callback(|_| Msg::CreateRole)}>{"确定"}</button>
</PermissionGuard>
                            </div>
                        </div>
                    </div>
                }

                if self.show_edit_modal {
                    <div class="modal-overlay" onclick={link.callback(|_| Msg::CloseEditModal)}>
                        <div class="modal" onclick={|e: MouseEvent| e.stop_propagation()}>
                            <div class="modal-header">
                                <h2>{"编辑角色"}</h2>
                                <button class="close-btn" onclick={link.callback(|_| Msg::CloseEditModal)}>{"×"}</button>
                            </div>
                            <div class="modal-body">
                                <div class="form-group">
                                    <label>{"角色名称"}</label>
                                    <input
                                        type="text"
                                        value={self.name.clone()}
                                        onchange={link.batch_callback(|e: Event| -> Option<crate::pages::role_list::Msg> {
                                            let input = e.target()?.dyn_into::<web_sys::HtmlInputElement>().ok()?;
                                            Some(Msg::NameChanged(input.value()))
                                        })}
                                        placeholder="请输入角色名称"
                                    />
                                </div>
                                <div class="form-group">
                                    <label>{"角色编码"}</label>
                                    <input
                                        type="text"
                                        value={self.code.clone()}
                                        onchange={link.batch_callback(|e: Event| -> Option<crate::pages::role_list::Msg> {
                                            let input = e.target()?.dyn_into::<web_sys::HtmlInputElement>().ok()?;
                                            Some(Msg::CodeChanged(input.value()))
                                        })}
                                        placeholder="请输入角色编码"
                                    />
                                </div>
                                <div class="form-group">
                                    <label>{"描述"}</label>
                                    <textarea
                                        value={self.description.clone()}
                                        onchange={link.batch_callback(|e: Event| -> Option<crate::pages::role_list::Msg> {
                                            let input = e.target()?.dyn_into::<web_sys::HtmlTextAreaElement>().ok()?;
                                            Some(Msg::DescriptionChanged(input.value()))
                                        })}
                                        placeholder="请输入角色描述"
                                    />
                                </div>
                                <div class="form-group">
                                    <label class="checkbox-label">
                                        <input
                                            type="checkbox"
                                            checked={self.is_system}
                                            onchange={link.batch_callback(|e: Event| -> Option<crate::pages::role_list::Msg> {
                                                let input = e.target()?.dyn_into::<web_sys::HtmlInputElement>().ok()?;
                                                Some(Msg::IsSystemChanged(input.checked()))
                                            })}
                                        />
                                        <span>{"系统角色"}</span>
                                    </label>
                                </div>
                            </div>
                            <div class="modal-footer">
                                <button class="cancel-btn" onclick={link.callback(|_| Msg::CloseEditModal)}>{"取消"}</button>
                                <PermissionGuard resource="role_list" action="update">
<button class="confirm-btn" onclick={link.callback(|_| Msg::UpdateRole)}>{"确定"}</button>
</PermissionGuard>
                            </div>
                        </div>
                    </div>
                }
            </div>
        }
    }
}

impl RoleListPage {
    fn load_roles(&mut self, ctx: &Context<Self>) {
        ctx.link().send_message(Msg::LoadRoles);
    }
}
