// 资金管理页面

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
use crate::services::fund_management_service::FundManagementService;
use crate::services::crud_service::CrudService;
use crate::models::fund_management::{FundAccount, FundAccountQueryParams, CreateFundAccountRequest};

pub struct FundManagementPage {
    accounts: Vec<FundAccount>,
    filtered_accounts: Vec<FundAccount>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_account: Option<FundAccount>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    // 表单字段
    form_name: String,
    form_no: String,
    form_type: String,
    form_currency: String,
    form_bank: String,
    form_remark: String,
    form_error: Option<String>,
}

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    Create,
    Edit,
}

pub enum Msg {
    LoadData,
    DataLoaded(Vec<FundAccount>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    OpenEditModal(FundAccount),
    CloseModal,
    SubmitForm,
    FormSubmitted,
    DeleteAccount(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    // 表单字段变更
    FormNameChanged(String),
    FormNoChanged(String),
    FormTypeChanged(String),
    FormCurrencyChanged(String),
    FormBankChanged(String),
    FormRemarkChanged(String),
}

impl Component for FundManagementPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            accounts: Vec::new(),
            filtered_accounts: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            show_modal: false,
            modal_mode: ModalMode::Create,
            editing_account: None,
            show_delete_confirm: false,
            deleting_id: None,
            form_name: String::new(),
            form_no: String::new(),
            form_type: String::new(),
            form_currency: "CNY".to_string(),
            form_bank: String::new(),
            form_remark: String::new(),
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
                    let query = FundAccountQueryParams {
                        account_type: None,
                        status: None,
                        page: Some(1),
                        page_size: Some(1000),
                    };
                    match FundManagementService::list_accounts(query).await {
                        Ok(res) => link.send_message(Msg::DataLoaded(res)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::DataLoaded(data) => {
                self.loading = false;
                self.accounts = data;
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
                self.editing_account = None;
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(account) => {
                self.form_name = account.account_name.clone();
                self.form_no = account.account_no.clone();
                self.form_type = account.account_type.clone();
                self.form_currency = account.currency.clone();
                self.form_bank = account.bank_name.clone().unwrap_or_default();
                self.form_remark = account.remark.clone().unwrap_or_default();
                self.form_error = None;
                self.editing_account = Some(account);
                self.modal_mode = ModalMode::Edit;
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.editing_account = None;
                self.form_error = None;
                true
            }
            Msg::SubmitForm => {
                // 表单验证
                if self.form_name.is_empty() {
                    self.form_error = Some("账户名称不能为空".to_string());
                    return true;
                }
                if self.form_no.is_empty() {
                    self.form_error = Some("账号不能为空".to_string());
                    return true;
                }
                if self.form_type.is_empty() {
                    self.form_error = Some("账户类型不能为空".to_string());
                    return true;
                }

                self.form_error = None;

                let req = CreateFundAccountRequest {
                    account_name: self.form_name.clone(),
                    account_no: self.form_no.clone(),
                    account_type: self.form_type.clone(),
                    bank_name: if self.form_bank.is_empty() { None } else { Some(self.form_bank.clone()) },
                    currency: self.form_currency.clone(),
                    opened_date: None,
                    remark: if self.form_remark.is_empty() { None } else { Some(self.form_remark.clone()) },
                };

                let link = ctx.link().clone();

                if self.modal_mode == ModalMode::Edit {
                    if let Some(account) = &self.editing_account {
                        let id = account.id;
                        spawn_local(async move {
                            match FundManagementService::update_account(id, req).await {
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
                        match FundManagementService::create_account(req).await {
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
                self.editing_account = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::DeleteAccount(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match FundManagementService::delete_account(id).await {
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
            Msg::FormNameChanged(v) => { self.form_name = v; true }
            Msg::FormNoChanged(v) => { self.form_no = v; true }
            Msg::FormTypeChanged(v) => { self.form_type = v; true }
            Msg::FormCurrencyChanged(v) => { self.form_currency = v; true }
            Msg::FormBankChanged(v) => { self.form_bank = v; true }
            Msg::FormRemarkChanged(v) => { self.form_remark = v; true }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="fund-management-page">
                <PageHeader title={"资金账户管理".to_string()} subtitle={Some("管理所有资金账户信息".to_string())}>
                    <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                        {"+ 新建资金账户"}
                    </button>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索账户名称或账号...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载资金账户数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadData)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_accounts.is_empty() {
                    <EmptyState
                        icon={"💰".to_string()}
                        title={"暂无资金账户数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一个资金账户".to_string()
                        } else {
                            "没有匹配搜索条件的账户".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"ID"}</th>
                                    <th>{"账户名称"}</th>
                                    <th>{"账号"}</th>
                                    <th>{"类型"}</th>
                                    <th>{"银行"}</th>
                                    <th>{"币种"}</th>
                                    <th class="numeric">{"余额"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_accounts().iter().map(|a| {
                                    let a_clone = a.clone();
                                    let id = a.id;
                                    html! {
                                        <tr>
                                            <td>{a.id}</td>
                                            <td>{&a.account_name}</td>
                                            <td>{&a.account_no}</td>
                                            <td>{&a.account_type}</td>
                                            <td>{a.bank_name.clone().unwrap_or_else(|| "-".to_string())}</td>
                                            <td>{&a.currency}</td>
                                            <td class="numeric">{&a.balance}</td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-secondary"
                                                        onclick={link.callback(move |_| Msg::OpenEditModal(a_clone.clone()))}
                                                    >
                                                        {"编辑"}
                                                    </button>
                                                    <PermissionGuard resource="fund_management" action="delete">
                                                        <button
                                                            class="btn btn-sm btn-danger"
                                                            onclick={link.callback(move |_| Msg::DeleteAccount(id))}
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
                            total={self.filtered_accounts.len() as u64}
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
                    message={"确定要删除这个资金账户吗？此操作不可撤销。".to_string()}
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

impl FundManagementPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_accounts = self.accounts.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_accounts = self.accounts.iter()
                .filter(|a| {
                    a.account_name.to_lowercase().contains(&keyword) ||
                    a.account_no.to_lowercase().contains(&keyword) ||
                    a.account_type.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_accounts(&self) -> Vec<FundAccount> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_accounts[start..end.min(self.filtered_accounts.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_name = String::new();
        self.form_no = String::new();
        self.form_type = String::new();
        self.form_currency = "CNY".to_string();
        self.form_bank = String::new();
        self.form_remark = String::new();
        self.form_error = None;
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let is_edit = self.modal_mode == ModalMode::Edit;
        let title = if is_edit { "编辑资金账户" } else { "新建资金账户" };

        let on_name_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormNameChanged(input.value()))
        });
        let on_no_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormNoChanged(input.value()))
        });
        let on_type_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormTypeChanged(input.value()))
        });
        let on_currency_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormCurrencyChanged(input.value()))
        });
        let on_bank_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormBankChanged(input.value()))
        });
        let on_remark_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormRemarkChanged(input.value()))
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
                            <label>{"账户名称 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_name.clone()}
                                oninput={on_name_change}
                                placeholder="请输入账户名称"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"账号 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_no.clone()}
                                oninput={on_no_change}
                                placeholder="请输入账号"
                            />
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"账户类型 *"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_type.clone()}
                                    oninput={on_type_change}
                                    placeholder="如：现金、银行、支付宝"
                                />
                            </div>
                            <div class="form-group">
                                <label>{"币种"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_currency.clone()}
                                    oninput={on_currency_change}
                                    placeholder="如：CNY、USD"
                                />
                            </div>
                        </div>
                        <div class="form-group">
                            <label>{"开户银行"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_bank.clone()}
                                oninput={on_bank_change}
                                placeholder="请输入开户银行"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"备注"}</label>
                            <textarea
                                class="form-input"
                                value={self.form_remark.clone()}
                                oninput={on_remark_change}
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
                            {if is_edit { "保存修改" } else { "创建账户" }}
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}
