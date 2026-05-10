// 预算管理页面

use crate::utils::toast_helper;
use yew::prelude::*;
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
use crate::services::budget_management_service::BudgetManagementService;
use crate::services::crud_service::CrudService;
use crate::models::budget_management::{BudgetItem, BudgetItemQuery, CreateBudgetItemRequest, UpdateBudgetItemRequest};

pub struct BudgetManagementPage {
    budgets: Vec<BudgetItem>,
    filtered_budgets: Vec<BudgetItem>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_budget: Option<BudgetItem>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    // 表单字段
    form_code: String,
    form_name: String,
    form_type: String,
    form_year: String,
    form_amount: String,
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
    DataLoaded(Vec<BudgetItem>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    OpenEditModal(BudgetItem),
    CloseModal,
    SubmitForm,
    FormSubmitted,
    DeleteBudget(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    // 表单字段变更
    FormCodeChanged(String),
    FormNameChanged(String),
    FormTypeChanged(String),
    FormYearChanged(String),
    FormAmountChanged(String),
    FormRemarkChanged(String),
}

impl Component for BudgetManagementPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            budgets: Vec::new(),
            filtered_budgets: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            show_modal: false,
            modal_mode: ModalMode::Create,
            editing_budget: None,
            show_delete_confirm: false,
            deleting_id: None,
            form_code: String::new(),
            form_name: String::new(),
            form_type: String::new(),
            form_year: chrono::Local::now().format("%Y").to_string(),
            form_amount: String::new(),
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
                    let query = BudgetItemQuery {
                        item_type: None,
                        status: None,
                        page: Some(1),
                        page_size: Some(1000),
                    };
                    match BudgetManagementService::list_items(query).await {
                        Ok(res) => link.send_message(Msg::DataLoaded(res.data)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::DataLoaded(data) => {
                self.loading = false;
                self.budgets = data;
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
                self.editing_budget = None;
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(budget) => {
                self.form_code = budget.item_code.clone();
                self.form_name = budget.item_name.clone();
                self.form_type = budget.item_type.clone();
                self.form_year = budget.budget_year.to_string();
                self.form_amount = budget.planned_amount.clone();
                self.form_remark = budget.remark.clone().unwrap_or_default();
                self.form_error = None;
                self.editing_budget = Some(budget);
                self.modal_mode = ModalMode::Edit;
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.editing_budget = None;
                self.form_error = None;
                true
            }
            Msg::SubmitForm => {
                if self.form_code.is_empty() {
                    self.form_error = Some("预算编码不能为空".to_string());
                    return true;
                }
                if self.form_name.is_empty() {
                    self.form_error = Some("预算名称不能为空".to_string());
                    return true;
                }
                if self.form_type.is_empty() {
                    self.form_error = Some("预算类型不能为空".to_string());
                    return true;
                }
                if self.form_amount.is_empty() {
                    self.form_error = Some("预算金额不能为空".to_string());
                    return true;
                }

                self.form_error = None;

                if self.modal_mode == ModalMode::Edit {
                    if let Some(budget) = &self.editing_budget {
                        let id = budget.id;
                        let req = UpdateBudgetItemRequest {
                            item_name: Some(self.form_name.clone()),
                            item_type: Some(self.form_type.clone()),
                            planned_amount: Some(self.form_amount.clone()),
                            status: None,
                            remark: if self.form_remark.is_empty() { None } else { Some(self.form_remark.clone()) },
                        };
                        let link = ctx.link().clone();
                        spawn_local(async move {
                            match BudgetManagementService::update_item(id, req).await {
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
                    let year = self.form_year.parse().unwrap_or(2024);
                    let req = CreateBudgetItemRequest {
                        item_code: self.form_code.clone(),
                        item_name: self.form_name.clone(),
                        item_type: self.form_type.clone(),
                        parent_id: None,
                        budget_year: year,
                        planned_amount: self.form_amount.clone(),
                        remark: if self.form_remark.is_empty() { None } else { Some(self.form_remark.clone()) },
                    };
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match BudgetManagementService::create_item(req).await {
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
                self.editing_budget = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::DeleteBudget(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match BudgetManagementService::delete_item(id).await {
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
            Msg::FormCodeChanged(v) => { self.form_code = v; true }
            Msg::FormNameChanged(v) => { self.form_name = v; true }
            Msg::FormTypeChanged(v) => { self.form_type = v; true }
            Msg::FormYearChanged(v) => { self.form_year = v; true }
            Msg::FormAmountChanged(v) => { self.form_amount = v; true }
            Msg::FormRemarkChanged(v) => { self.form_remark = v; true }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="budget-management-page">
                <PageHeader title={"预算管理".to_string()} subtitle={Some("管理预算科目信息".to_string())}>
                    <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                        {"+ 新建预算"}
                    </button>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索预算编码或名称...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载预算数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadData)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_budgets.is_empty() {
                    <EmptyState
                        icon={"📊".to_string()}
                        title={"暂无预算数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一个预算科目".to_string()
                        } else {
                            "没有匹配搜索条件的预算".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"ID"}</th>
                                    <th>{"预算编码"}</th>
                                    <th>{"预算名称"}</th>
                                    <th>{"类型"}</th>
                                    <th>{"年度"}</th>
                                    <th class="numeric">{"预算金额"}</th>
                                    <th>{"状态"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_budgets().iter().map(|b| {
                                    let b_clone = b.clone();
                                    let id = b.id;
                                    html! {
                                        <tr>
                                            <td>{b.id}</td>
                                            <td>{&b.item_code}</td>
                                            <td>{&b.item_name}</td>
                                            <td>{&b.item_type}</td>
                                            <td>{b.budget_year}</td>
                                            <td class="numeric">{&b.planned_amount}</td>
                                            <td>
                                                <span class="status-badge status-success">
                                                    {b.status.as_deref().unwrap_or("正常")}
                                                </span>
                                            </td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-secondary"
                                                        onclick={link.callback(move |_| Msg::OpenEditModal(b_clone.clone()))}
                                                    >
                                                        {"编辑"}
                                                    </button>
                                                    <button
                                                        class="btn btn-sm btn-danger"
                                                        onclick={link.callback(move |_| Msg::DeleteBudget(id))}
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
                            total={self.filtered_budgets.len() as u64}
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
                    message={"确定要删除这个预算科目吗？此操作不可撤销。".to_string()}
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

impl BudgetManagementPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_budgets = self.budgets.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_budgets = self.budgets.iter()
                .filter(|b| {
                    b.item_code.to_lowercase().contains(&keyword) ||
                    b.item_name.to_lowercase().contains(&keyword) ||
                    b.item_type.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_budgets(&self) -> Vec<BudgetItem> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_budgets[start..end.min(self.filtered_budgets.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_code = String::new();
        self.form_name = String::new();
        self.form_type = String::new();
        self.form_year = chrono::Local::now().format("%Y").to_string();
        self.form_amount = String::new();
        self.form_remark = String::new();
        self.form_error = None;
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let is_edit = self.modal_mode == ModalMode::Edit;
        let title = if is_edit { "编辑预算科目" } else { "新建预算科目" };

        let on_code_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormCodeChanged(input.value()))
        });
        let on_name_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormNameChanged(input.value()))
        });
        let on_type_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormTypeChanged(input.value()))
        });
        let on_year_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormYearChanged(input.value()))
        });
        let on_amount_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormAmountChanged(input.value()))
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
                            <label>{"预算编码 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_code.clone()}
                                oninput={on_code_change}
                                placeholder="请输入预算编码"
                                disabled={is_edit}
                            />
                        </div>
                        <div class="form-group">
                            <label>{"预算名称 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_name.clone()}
                                oninput={on_name_change}
                                placeholder="请输入预算名称"
                            />
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"预算类型 *"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_type.clone()}
                                    oninput={on_type_change}
                                    placeholder="如：运营、资本、人力"
                                />
                            </div>
                            <div class="form-group">
                                <label>{"预算年度"}</label>
                                <input
                                    type="number"
                                    class="form-input"
                                    value={self.form_year.clone()}
                                    oninput={on_year_change}
                                    placeholder="如：2024"
                                />
                            </div>
                        </div>
                        <div class="form-group">
                            <label>{"预算金额 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_amount.clone()}
                                oninput={on_amount_change}
                                placeholder="请输入预算金额"
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
                            {if is_edit { "保存修改" } else { "创建预算" }}
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}
