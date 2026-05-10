// 会计科目管理页面

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
use crate::services::account_subject_service::AccountSubjectService;
use crate::services::crud_service::CrudService;
use crate::models::account_subject::{SubjectTreeNode, CreateSubjectRequest, UpdateSubjectRequest};

pub struct AccountSubjectPage {
    tree: Vec<SubjectTreeNode>,
    flat_subjects: Vec<FlatSubject>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_id: Option<i32>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    // 表单字段
    form_code: String,
    form_name: String,
    form_level: String,
    form_balance_direction: String,
    form_assist_customer: bool,
    form_assist_supplier: bool,
    form_assist_batch: bool,
    form_assist_color_no: bool,
    form_enable_dual_unit: bool,
    form_error: Option<String>,
}

#[derive(Clone)]
struct FlatSubject {
    id: i32,
    code: String,
    name: String,
    level: i32,
    depth: usize,
}

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    Create,
    Edit,
}

pub enum Msg {
    LoadData,
    DataLoaded(Vec<SubjectTreeNode>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    OpenEditModal(i32, String, String),
    CloseModal,
    SubmitForm,
    FormSubmitted,
    DeleteSubject(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    // 表单字段变更
    FormCodeChanged(String),
    FormNameChanged(String),
    FormLevelChanged(String),
    FormBalanceDirectionChanged(String),
    FormAssistCustomerChanged(bool),
    FormAssistSupplierChanged(bool),
    FormAssistBatchChanged(bool),
    FormAssistColorNoChanged(bool),
    FormEnableDualUnitChanged(bool),
}

impl Component for AccountSubjectPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            tree: Vec::new(),
            flat_subjects: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            show_modal: false,
            modal_mode: ModalMode::Create,
            editing_id: None,
            show_delete_confirm: false,
            deleting_id: None,
            form_code: String::new(),
            form_name: String::new(),
            form_level: "1".to_string(),
            form_balance_direction: "借".to_string(),
            form_assist_customer: false,
            form_assist_supplier: false,
            form_assist_batch: false,
            form_assist_color_no: false,
            form_enable_dual_unit: false,
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
                    match AccountSubjectService::get_subject_tree().await {
                        Ok(data) => link.send_message(Msg::DataLoaded(data)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::DataLoaded(data) => {
                self.loading = false;
                self.tree = data;
                self.rebuild_flat_subjects();
                self.error = None;
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
                self.rebuild_flat_subjects();
                true
            }
            Msg::ResetSearch => {
                self.search_keyword = String::new();
                self.page = 0;
                self.rebuild_flat_subjects();
                true
            }
            Msg::PageChanged(page) => {
                self.page = page;
                true
            }
            Msg::OpenCreateModal => {
                self.reset_form();
                self.editing_id = None;
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(id, code, name) => {
                self.form_code = code;
                self.form_name = name;
                self.form_error = None;
                self.editing_id = Some(id);
                self.modal_mode = ModalMode::Edit;
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.editing_id = None;
                self.form_error = None;
                true
            }
            Msg::SubmitForm => {
                if self.form_code.is_empty() {
                    self.form_error = Some("科目代码不能为空".to_string());
                    return true;
                }
                if self.form_name.is_empty() {
                    self.form_error = Some("科目名称不能为空".to_string());
                    return true;
                }

                self.form_error = None;

                if self.modal_mode == ModalMode::Edit {
                    if let Some(id) = self.editing_id {
                        let req = UpdateSubjectRequest {
                            name: Some(self.form_name.clone()),
                            balance_direction: Some(self.form_balance_direction.clone()),
                            assist_customer: self.form_assist_customer,
                            assist_supplier: self.form_assist_supplier,
                            assist_batch: self.form_assist_batch,
                            assist_color_no: self.form_assist_color_no,
                            enable_dual_unit: self.form_enable_dual_unit,
                        };
                        let link = ctx.link().clone();
                        spawn_local(async move {
                            match AccountSubjectService::update_subject(id, req).await {
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
                    let level = self.form_level.parse().unwrap_or(1);
                    let req = CreateSubjectRequest {
                        code: self.form_code.clone(),
                        name: self.form_name.clone(),
                        level,
                        parent_id: None,
                        balance_direction: Some(self.form_balance_direction.clone()),
                        assist_customer: self.form_assist_customer,
                        assist_supplier: self.form_assist_supplier,
                        assist_batch: self.form_assist_batch,
                        assist_color_no: self.form_assist_color_no,
                        enable_dual_unit: self.form_enable_dual_unit,
                    };
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match AccountSubjectService::create_subject(req).await {
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
                self.editing_id = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::DeleteSubject(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match AccountSubjectService::delete_subject(id).await {
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
            Msg::FormLevelChanged(v) => { self.form_level = v; true }
            Msg::FormBalanceDirectionChanged(v) => { self.form_balance_direction = v; true }
            Msg::FormAssistCustomerChanged(v) => { self.form_assist_customer = v; true }
            Msg::FormAssistSupplierChanged(v) => { self.form_assist_supplier = v; true }
            Msg::FormAssistBatchChanged(v) => { self.form_assist_batch = v; true }
            Msg::FormAssistColorNoChanged(v) => { self.form_assist_color_no = v; true }
            Msg::FormEnableDualUnitChanged(v) => { self.form_enable_dual_unit = v; true }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="account-subject-page">
                <PageHeader title={"会计科目管理".to_string()} subtitle={Some("管理会计科目体系".to_string())}>
                    <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                        {"+ 新建科目"}
                    </button>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索科目代码或名称...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载科目数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadData)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.flat_subjects.is_empty() {
                    <EmptyState
                        icon={"📚".to_string()}
                        title={"暂无科目数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一个会计科目".to_string()
                        } else {
                            "没有匹配搜索条件的科目".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"科目代码"}</th>
                                    <th>{"科目名称"}</th>
                                    <th>{"层级"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_subjects().iter().map(|s| {
                                    let id = s.id;
                                    let code = s.code.clone();
                                    let name = s.name.clone();
                                    let padding = s.depth * 20;
                                    html! {
                                        <tr>
                                            <td style={format!("padding-left: {}px", padding + 12)}>
                                                if s.depth > 0 {
                                                    <span style="margin-right: 4px;">{"└─ "}</span>
                                                }
                                                {&s.code}
                                            </td>
                                            <td>{&s.name}</td>
                                            <td>{s.level}</td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-secondary"
                                                        onclick={link.callback(move |_| Msg::OpenEditModal(id, code.clone(), name.clone()))}
                                                    >
                                                        {"编辑"}
                                                    </button>
                                                    <button
                                                        class="btn btn-sm btn-danger"
                                                        onclick={link.callback(move |_| Msg::DeleteSubject(id))}
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
                            total={self.flat_subjects.len() as u64}
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
                    message={"确定要删除这个会计科目吗？此操作不可撤销。".to_string()}
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

impl AccountSubjectPage {
    fn rebuild_flat_subjects(&mut self) {
        self.flat_subjects = Vec::new();
        let keyword = self.search_keyword.to_lowercase();
        let tree = self.tree.clone();
        for node in &tree {
            self.flatten_node(node, 0, &keyword);
        }
    }

    fn flatten_node(&mut self, node: &SubjectTreeNode, depth: usize, keyword: &str) {
        let matches = keyword.is_empty() ||
                      node.code.to_lowercase().contains(keyword) ||
                      node.name.to_lowercase().contains(keyword);
        if matches {
            self.flat_subjects.push(FlatSubject {
                id: node.id,
                code: node.code.clone(),
                name: node.name.clone(),
                level: node.level,
                depth,
            });
        }
        for child in &node.children {
            self.flatten_node(child, depth + 1, keyword);
        }
    }

    fn paginated_subjects(&self) -> Vec<&FlatSubject> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.flat_subjects[start..end.min(self.flat_subjects.len())].iter().collect()
    }

    fn reset_form(&mut self) {
        self.form_code = String::new();
        self.form_name = String::new();
        self.form_level = "1".to_string();
        self.form_balance_direction = "借".to_string();
        self.form_assist_customer = false;
        self.form_assist_supplier = false;
        self.form_assist_batch = false;
        self.form_assist_color_no = false;
        self.form_enable_dual_unit = false;
        self.form_error = None;
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let is_edit = self.modal_mode == ModalMode::Edit;
        let title = if is_edit { "编辑会计科目" } else { "新建会计科目" };

        let on_code_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormCodeChanged(input.value()))
        });
        let on_name_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormNameChanged(input.value()))
        });
        let on_level_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormLevelChanged(input.value()))
        });
        let on_direction_change = link.batch_callback(|e: Event| {
            e.target().and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok()).map(|select| Msg::FormBalanceDirectionChanged(select.value()))
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
                            <label>{"科目代码 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_code.clone()}
                                oninput={on_code_change}
                                placeholder="请输入科目代码"
                                disabled={is_edit}
                            />
                        </div>
                        <div class="form-group">
                            <label>{"科目名称 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_name.clone()}
                                oninput={on_name_change}
                                placeholder="请输入科目名称"
                            />
                        </div>
                        if !is_edit {
                            <div class="form-group">
                                <label>{"层级"}</label>
                                <input
                                    type="number"
                                    class="form-input"
                                    value={self.form_level.clone()}
                                    oninput={on_level_change}
                                    placeholder="如：1、2、3"
                                />
                            </div>
                        }
                        <div class="form-group">
                            <label>{"余额方向"}</label>
                            <select class="form-input" value={self.form_balance_direction.clone()} onchange={on_direction_change}>
                                <option value="借">{"借"}</option>
                                <option value="贷">{"贷"}</option>
                            </select>
                        </div>
                        <div class="form-row">
                            <div class="form-group checkbox-group">
                                <label>
                                    <input
                                        type="checkbox"
                                        checked={self.form_assist_customer}
                                        onchange={link.callback(|e: Event| {
                                            let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                            Msg::FormAssistCustomerChanged(target.checked())
                                        })}
                                    />
                                    {"辅助客户"}
                                </label>
                            </div>
                            <div class="form-group checkbox-group">
                                <label>
                                    <input
                                        type="checkbox"
                                        checked={self.form_assist_supplier}
                                        onchange={link.callback(|e: Event| {
                                            let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                            Msg::FormAssistSupplierChanged(target.checked())
                                        })}
                                    />
                                    {"辅助供应商"}
                                </label>
                            </div>
                        </div>
                        <div class="form-row">
                            <div class="form-group checkbox-group">
                                <label>
                                    <input
                                        type="checkbox"
                                        checked={self.form_assist_batch}
                                        onchange={link.callback(|e: Event| {
                                            let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                            Msg::FormAssistBatchChanged(target.checked())
                                        })}
                                    />
                                    {"辅助批次"}
                                </label>
                            </div>
                            <div class="form-group checkbox-group">
                                <label>
                                    <input
                                        type="checkbox"
                                        checked={self.form_assist_color_no}
                                        onchange={link.callback(|e: Event| {
                                            let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                            Msg::FormAssistColorNoChanged(target.checked())
                                        })}
                                    />
                                    {"辅助色号"}
                                </label>
                            </div>
                        </div>
                        <div class="form-group checkbox-group">
                            <label>
                                <input
                                    type="checkbox"
                                    checked={self.form_enable_dual_unit}
                                    onchange={link.callback(|e: Event| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::FormEnableDualUnitChanged(target.checked())
                                    })}
                                />
                                {"启用双单位"}
                            </label>
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={link.callback(|_| Msg::CloseModal)}>
                            {"取消"}
                        </button>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::SubmitForm)}>
                            {if is_edit { "保存修改" } else { "创建科目" }}
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}
