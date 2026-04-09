use crate::app::Route;
use crate::components::main_layout::MainLayout;
use crate::models::user::User;
use crate::services::user_service::UserService;
use crate::utils::storage::Storage;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use web_sys::window;
use yew_router::prelude::*;

pub struct UserListPage {
    users: Vec<User>,
    total: u64,
    page: u64,
    page_size: u64,
    is_loading: bool,
    error_message: Option<String>,
    user_service: UserService,
}

pub enum Msg {
    LoadUsers,
    UsersLoaded(Vec<User>, u64),
    LoadError(String),
    LoadingChanged(bool),
    PageChanged(u64),
    Logout,
}

impl Component for UserListPage {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let mut this = Self {
            users: Vec::new(),
            total: 0,
            page: 0,
            page_size: 20,
            is_loading: false,
            error_message: None,
            user_service: UserService::new(),
        };

        // 检查是否已登录
        if Storage::get_token().is_none() {
            if let Some(navigator) = ctx.link().navigator() {
                navigator.push(&Route::Login);
            }
            return this;
        } else {
            this.load_users(ctx);
        }

        this
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadUsers => {
                self.is_loading = true;
                self.error_message = None;

                let service = self.user_service.clone();
                let page = self.page;
                let page_size = self.page_size;
                let link = ctx.link().clone();

                spawn_local(async move {
                    match service.list_users(page, page_size).await {
                        Ok(response) => {
                            link.send_message(Msg::UsersLoaded(response.users, response.total));
                        }
                        Err(error) => {
                            link.send_message(Msg::LoadError(error));
                        }
                    }
                });
                true
            }
            Msg::UsersLoaded(users, total) => {
                self.is_loading = false;
                self.users = users;
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
            Msg::PageChanged(new_page) => {
                self.page = new_page;
                self.load_users(ctx);
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

        let on_prev = link.callback(move |_: MouseEvent| Msg::PageChanged(0));

        let on_next = link.callback(move |_: MouseEvent| Msg::PageChanged(0));

        let _on_logout = link.callback(|_: MouseEvent| Msg::Logout);

        html! {
            <MainLayout current_page={"users".to_string()}>
                <div class="user-list-page">
                    <div class="page-header">
                        <h1>{"用户管理"}</h1>
                    </div>

                    if let Some(error) = &self.error_message {
                        <div class="error-message">{error}</div>
                    }

                    if self.is_loading {
                        <div class="loading">{"加载中..."}</div>
                    } else {
                        <div class="user-table-container">
                            <div class="table-responsive overflow-x-auto w-full pb-4 shadow-sm sm:rounded-lg">
<table class="data-table w-full">
                                <thead>
                                    <tr>
                                        <th class="numeric-cell text-right">{"ID"}</th>
                                        <th>{"用户名"}</th>
                                        <th>{"邮箱"}</th>
                                        <th>{"手机号"}</th>
                                        <th class="numeric-cell text-right">{"角色"}</th>
                                        <th>{"关联源单据"}</th>
                                                <th>{"状态"}</th>
                                        <th>{"创建时间"}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {for self.users.iter().map(|user| {
                                        html! {
                                            <tr>
                                                <td class="numeric-cell text-right">{user.id}</td>
                                                <td>{&user.username}</td>
                                                <td>
                                                    if let Some(email) = &user.email {
                                                        {email}
                                                    } else {
                                                        {"-"}
                                                    }
                                                </td>
                                                <td>
                                                    if let Some(phone) = &user.phone {
                                                        {phone}
                                                    } else {
                                                        {"-"}
                                                    }
                                                </td>
                                                <td class="numeric-cell text-right">
                                                    if let Some(role_id) = user.role_id {
                                                        {format!("角色 #{}", role_id)}
                                                    } else {
                                                        {"无"}
                                                    }
                                                </td>
                                                <td>
                                                    if user.is_active {
                                                        <span class="status-badge status-active">{"正常"}</span>
                                                    } else {
                                                        <span class="status-badge status-inactive">{"禁用"}</span>
                                                    }
                                                </td>
                                                <td>{&user.created_at}</td>
                                            </tr>
                                        }
                                    })}
                                </tbody>
                            </table>
</div>

                            <div class="pagination">
                                <button onclick={on_prev} disabled={self.page == 0}>
                                    {"上一页"}
                                </button>
                                <span class="page-info">
                                    {format!("第 {} 页 / 共 {} 条", self.page + 1, self.total)}
                                </span>
                                <button onclick={on_next} disabled={(self.page + 1) * self.page_size >= self.total}>
                                    {"下一页"}
                                </button>
                            </div>
                        </div>
                    }
                </div>
            </MainLayout>
        }
    }
}

impl UserListPage {
    fn load_users(&mut self, ctx: &Context<Self>) {
        ctx.link().send_message(Msg::LoadUsers);
    }
}
