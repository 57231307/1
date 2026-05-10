// 染色配方管理页面

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
use crate::models::dye_recipe::{
    CreateDyeRecipeRequest, UpdateDyeRecipeRequest,
    DyeRecipe, DyeRecipeQuery,
};
use crate::services::dye_recipe_service::DyeRecipeService;
use crate::services::crud_service::CrudService;

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    Create,
    Edit,
    View,
}

pub struct DyeRecipePage {
    recipes: Vec<DyeRecipe>,
    filtered_recipes: Vec<DyeRecipe>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_recipe: Option<DyeRecipe>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    // 表单字段
    form_recipe_no: String,
    form_color_code: String,
    form_color_name: String,
    form_fabric_type: String,
    form_dye_type: String,
    form_temperature: String,
    form_time_minutes: String,
    form_ph_value: String,
    form_chemical_formula: String,
    form_remarks: String,
    form_error: Option<String>,
}

pub enum Msg {
    LoadRecipes,
    RecipesLoaded(Vec<DyeRecipe>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    OpenEditModal(DyeRecipe),
    OpenViewModal(DyeRecipe),
    CloseModal,
    SubmitForm,
    FormSubmitted,
    ApproveRecipe(i32),
    DeleteRecipe(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    // 表单字段变更
    FormRecipeNoChanged(String),
    FormColorCodeChanged(String),
    FormColorNameChanged(String),
    FormFabricTypeChanged(String),
    FormDyeTypeChanged(String),
    FormTemperatureChanged(String),
    FormTimeMinutesChanged(String),
    FormPhValueChanged(String),
    FormChemicalFormulaChanged(String),
    FormRemarksChanged(String),
}

impl Component for DyeRecipePage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            recipes: Vec::new(),
            filtered_recipes: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            show_modal: false,
            modal_mode: ModalMode::View,
            editing_recipe: None,
            show_delete_confirm: false,
            deleting_id: None,
            form_recipe_no: String::new(),
            form_color_code: String::new(),
            form_color_name: String::new(),
            form_fabric_type: String::new(),
            form_dye_type: String::new(),
            form_temperature: String::new(),
            form_time_minutes: String::new(),
            form_ph_value: String::new(),
            form_chemical_formula: String::new(),
            form_remarks: String::new(),
            form_error: None,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadRecipes);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadRecipes => {
                self.loading = true;
                self.error = None;
                let query = DyeRecipeQuery {
                    page: Some(1),
                    page_size: Some(1000),
                    recipe_no: None,
                    color_code: None,
                    color_name: None,
                    dye_type: None,
                    status: None,
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match DyeRecipeService::list(query).await {
                        Ok(recipes) => link.send_message(Msg::RecipesLoaded(recipes.items)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::RecipesLoaded(recipes) => {
                self.loading = false;
                self.recipes = recipes;
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
                self.editing_recipe = None;
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(recipe) => {
                self.form_recipe_no = recipe.recipe_no.clone();
                self.form_color_code = recipe.color_code.clone();
                self.form_color_name = recipe.color_name.clone();
                self.form_fabric_type = recipe.fabric_type.clone().unwrap_or_default();
                self.form_dye_type = recipe.dye_type.clone().unwrap_or_default();
                self.form_temperature = recipe.temperature.clone().unwrap_or_default();
                self.form_time_minutes = recipe.time_minutes.map(|t| t.to_string()).unwrap_or_default();
                self.form_ph_value = recipe.ph_value.clone().unwrap_or_default();
                self.form_chemical_formula = recipe.chemical_formula.clone().unwrap_or_default();
                self.form_remarks = recipe.remarks.clone().unwrap_or_default();
                self.form_error = None;
                self.editing_recipe = Some(recipe);
                self.modal_mode = ModalMode::Edit;
                self.show_modal = true;
                true
            }
            Msg::OpenViewModal(recipe) => {
                self.editing_recipe = Some(recipe);
                self.modal_mode = ModalMode::View;
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.editing_recipe = None;
                self.form_error = None;
                true
            }
            Msg::SubmitForm => {
                // 表单验证
                if self.form_recipe_no.is_empty() {
                    self.form_error = Some("配方编号不能为空".to_string());
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
                    if let Some(recipe) = &self.editing_recipe {
                        let id = recipe.id;
                        let req = UpdateDyeRecipeRequest {
                            color_code: Some(self.form_color_code.clone()),
                            color_name: Some(self.form_color_name.clone()),
                            fabric_type: if self.form_fabric_type.is_empty() { None } else { Some(self.form_fabric_type.clone()) },
                            dye_type: if self.form_dye_type.is_empty() { None } else { Some(self.form_dye_type.clone()) },
                            chemical_formula: if self.form_chemical_formula.is_empty() { None } else { Some(self.form_chemical_formula.clone()) },
                            temperature: if self.form_temperature.is_empty() { None } else { Some(self.form_temperature.clone()) },
                            time_minutes: self.form_time_minutes.parse().ok(),
                            ph_value: if self.form_ph_value.is_empty() { None } else { Some(self.form_ph_value.clone()) },
                            liquor_ratio: None,
                            auxiliaries: None,
                            status: None,
                            remarks: if self.form_remarks.is_empty() { None } else { Some(self.form_remarks.clone()) },
                        };
                        let link = ctx.link().clone();
                        spawn_local(async move {
                            match DyeRecipeService::update(id, req).await {
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
                    let req = CreateDyeRecipeRequest {
                        recipe_no: self.form_recipe_no.clone(),
                        color_code: self.form_color_code.clone(),
                        color_name: self.form_color_name.clone(),
                        fabric_type: if self.form_fabric_type.is_empty() { None } else { Some(self.form_fabric_type.clone()) },
                        dye_type: if self.form_dye_type.is_empty() { None } else { Some(self.form_dye_type.clone()) },
                        chemical_formula: if self.form_chemical_formula.is_empty() { None } else { Some(self.form_chemical_formula.clone()) },
                        temperature: if self.form_temperature.is_empty() { None } else { Some(self.form_temperature.clone()) },
                        time_minutes: self.form_time_minutes.parse().ok(),
                        ph_value: if self.form_ph_value.is_empty() { None } else { Some(self.form_ph_value.clone()) },
                        liquor_ratio: None,
                        auxiliaries: None,
                        status: Some("草稿".to_string()),
                        version: Some(1),
                        parent_recipe_id: None,
                        remarks: if self.form_remarks.is_empty() { None } else { Some(self.form_remarks.clone()) },
                        created_by: None,
                    };
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match DyeRecipeService::create(req).await {
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
                self.editing_recipe = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadRecipes);
                false
            }
            Msg::ApproveRecipe(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    let req = crate::models::dye_recipe::ApproveRecipeRequest {
                        approved_by: 1,
                    };
                    match DyeRecipeService::approve(id, req).await {
                        Ok(_) => {
                            toast_helper::show_success("审核成功");
                            link.send_message(Msg::LoadRecipes);
                        }
                        Err(e) => {
                            toast_helper::show_error(&format!("审核失败: {}", e));
                        }
                    }
                });
                false
            }
            Msg::DeleteRecipe(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match DyeRecipeService::delete(id).await {
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
                ctx.link().send_message(Msg::LoadRecipes);
                false
            }
            Msg::FormRecipeNoChanged(v) => { self.form_recipe_no = v; true }
            Msg::FormColorCodeChanged(v) => { self.form_color_code = v; true }
            Msg::FormColorNameChanged(v) => { self.form_color_name = v; true }
            Msg::FormFabricTypeChanged(v) => { self.form_fabric_type = v; true }
            Msg::FormDyeTypeChanged(v) => { self.form_dye_type = v; true }
            Msg::FormTemperatureChanged(v) => { self.form_temperature = v; true }
            Msg::FormTimeMinutesChanged(v) => { self.form_time_minutes = v; true }
            Msg::FormPhValueChanged(v) => { self.form_ph_value = v; true }
            Msg::FormChemicalFormulaChanged(v) => { self.form_chemical_formula = v; true }
            Msg::FormRemarksChanged(v) => { self.form_remarks = v; true }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="dye-recipe-page">
                <PageHeader title={"染色配方管理".to_string()} subtitle={Some("管理染色工艺配方".to_string())}>
                    <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                        {"+ 新建配方"}
                    </button>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索配方编号或色号...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载染色配方数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadRecipes)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_recipes.is_empty() {
                    <EmptyState
                        icon={"🧪".to_string()}
                        title={"暂无染色配方数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一个配方".to_string()
                        } else {
                            "没有匹配搜索条件的配方".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"配方编号"}</th>
                                    <th>{"色号"}</th>
                                    <th>{"颜色名称"}</th>
                                    <th>{"染色类型"}</th>
                                    <th class="numeric">{"温度(℃)"}</th>
                                    <th class="numeric">{"时间(分钟)"}</th>
                                    <th class="numeric">{"pH值"}</th>
                                    <th>{"版本"}</th>
                                    <th>{"状态"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_recipes().iter().map(|recipe| {
                                    let recipe_clone = recipe.clone();
                                    let recipe_clone2 = recipe.clone();
                                    let recipe_clone3 = recipe.clone();
                                    let recipe_id = recipe.id;
                                    let is_draft = recipe.status == "草稿";
                                    html! {
                                        <tr>
                                            <td>{&recipe.recipe_no}</td>
                                            <td>{&recipe.color_code}</td>
                                            <td>{&recipe.color_name}</td>
                                            <td>{recipe.dye_type.as_deref().unwrap_or("-")}</td>
                                            <td class="numeric">{recipe.temperature.as_ref().unwrap_or(&"-".to_string())}</td>
                                            <td class="numeric">{recipe.time_minutes.map(|t| t.to_string()).unwrap_or("-".to_string())}</td>
                                            <td class="numeric">{recipe.ph_value.as_ref().unwrap_or(&"-".to_string())}</td>
                                            <td class="numeric">{format!("V{}", recipe.version.unwrap_or(1))}</td>
                                            <td>
                                                <span class={format!("status-badge status-{}", if is_draft { "draft" } else { "approved" })}>
                                                    {&recipe.status}
                                                </span>
                                            </td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-info"
                                                        onclick={link.callback(move |_| Msg::OpenViewModal(recipe_clone.clone()))}
                                                    >
                                                        {"详情"}
                                                    </button>
                                                    if is_draft {
                                                        <button
                                                            class="btn btn-sm btn-secondary"
                                                            onclick={link.callback(move |_| Msg::OpenEditModal(recipe_clone2.clone()))}
                                                        >
                                                            {"编辑"}
                                                        </button>
                                                    }
                                                    if is_draft {
                                                        <PermissionGuard resource="dye_recipe" action="approve">
                                                            <button
                                                                class="btn btn-sm btn-success"
                                                                onclick={link.callback(move |_| Msg::ApproveRecipe(recipe_id))}
                                                            >
                                                                {"审核"}
                                                            </button>
                                                        </PermissionGuard>
                                                    }
                                                    <PermissionGuard resource="dye_recipe" action="delete">
                                                        <button
                                                            class="btn btn-sm btn-danger"
                                                            onclick={link.callback(move |_| Msg::DeleteRecipe(recipe_id))}
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
                            total={self.filtered_recipes.len() as u64}
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
                    message={"确定要删除这个染色配方吗？此操作不可撤销。".to_string()}
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

impl DyeRecipePage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_recipes = self.recipes.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_recipes = self.recipes.iter()
                .filter(|r| {
                    r.recipe_no.to_lowercase().contains(&keyword) ||
                    r.color_code.to_lowercase().contains(&keyword) ||
                    r.color_name.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_recipes(&self) -> Vec<DyeRecipe> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_recipes[start..end.min(self.filtered_recipes.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_recipe_no = String::new();
        self.form_color_code = String::new();
        self.form_color_name = String::new();
        self.form_fabric_type = String::new();
        self.form_dye_type = String::new();
        self.form_temperature = String::new();
        self.form_time_minutes = String::new();
        self.form_ph_value = String::new();
        self.form_chemical_formula = String::new();
        self.form_remarks = String::new();
        self.form_error = None;
    }

    fn render_modal(&self, ctx: &Context<Self>) -> Html {
        match self.modal_mode {
            ModalMode::View => self.render_view_modal(ctx),
            _ => self.render_form_modal(ctx),
        }
    }

    fn render_view_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        if let Some(recipe) = &self.editing_recipe {
            html! {
                <div class="modal-overlay" onclick={link.callback(|_| Msg::CloseModal)}>
                    <div class="modal-content modal-lg" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                        <div class="modal-header">
                            <h3>{"配方详情"}</h3>
                            <button class="close-btn" onclick={link.callback(|_| Msg::CloseModal)}>{"×"}</button>
                        </div>
                        <div class="modal-body">
                            <div class="detail-grid">
                                <div class="detail-item">
                                    <label>{"配方编号："}</label>
                                    <span>{&recipe.recipe_no}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"色号："}</label>
                                    <span>{&recipe.color_code}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"颜色名称："}</label>
                                    <span>{&recipe.color_name}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"面料类型："}</label>
                                    <span>{recipe.fabric_type.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"染色类型："}</label>
                                    <span>{recipe.dye_type.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"温度："}</label>
                                    <span>{recipe.temperature.as_ref().map(|t| format!("{}℃", t)).unwrap_or("-".to_string())}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"时间："}</label>
                                    <span>{recipe.time_minutes.map(|t| format!("{}分钟", t)).unwrap_or("-".to_string())}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"pH值："}</label>
                                    <span>{recipe.ph_value.as_deref().unwrap_or("-")}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"浴比："}</label>
                                    <span>{recipe.liquor_ratio.as_ref().map(|l| format!("1:{}", l)).unwrap_or("-".to_string())}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"版本："}</label>
                                    <span>{format!("V{}", recipe.version.unwrap_or(1))}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"状态："}</label>
                                    <span>{&recipe.status}</span>
                                </div>
                            </div>
                            if let Some(formula) = &recipe.chemical_formula {
                                <div class="detail-section">
                                    <h3>{"化学配方"}</h3>
                                    <pre>{formula}</pre>
                                </div>
                            }
                        </div>
                        <div class="modal-footer">
                            <button class="btn btn-primary" onclick={link.callback(|_| Msg::CloseModal)}>{"关闭"}</button>
                        </div>
                    </div>
                </div>
            }
        } else {
            html! {}
        }
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let is_edit = self.modal_mode == ModalMode::Edit;
        let title = if is_edit { "编辑配方" } else { "新建配方" };

        let on_recipe_no_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormRecipeNoChanged(input.value()))
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
        let on_dye_type_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormDyeTypeChanged(input.value()))
        });
        let on_temperature_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormTemperatureChanged(input.value()))
        });
        let on_time_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormTimeMinutesChanged(input.value()))
        });
        let on_ph_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormPhValueChanged(input.value()))
        });
        let on_formula_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormChemicalFormulaChanged(input.value()))
        });
        let on_remarks_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormRemarksChanged(input.value()))
        });

        html! {
            <div class="modal-overlay" onclick={link.callback(|_| Msg::CloseModal)}>
                <div class="modal-content modal-lg" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <div class="modal-header">
                        <h3>{title}</h3>
                        <button class="close-btn" onclick={link.callback(|_| Msg::CloseModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        if let Some(err) = &self.form_error {
                            <div class="form-error">{err}</div>
                        }
                        <div class="form-group">
                            <label>{"配方编号 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_recipe_no.clone()}
                                oninput={on_recipe_no_change}
                                placeholder="请输入配方编号"
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
                        <div class="form-row">
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
                            <div class="form-group">
                                <label>{"染色类型"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_dye_type.clone()}
                                    oninput={on_dye_type_change}
                                    placeholder="如：活性染料、分散染料"
                                />
                            </div>
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"温度(℃)"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_temperature.clone()}
                                    oninput={on_temperature_change}
                                    placeholder="请输入温度"
                                />
                            </div>
                            <div class="form-group">
                                <label>{"时间(分钟)"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_time_minutes.clone()}
                                    oninput={on_time_change}
                                    placeholder="请输入时间"
                                />
                            </div>
                            <div class="form-group">
                                <label>{"pH值"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_ph_value.clone()}
                                    oninput={on_ph_change}
                                    placeholder="请输入pH值"
                                />
                            </div>
                        </div>
                        <div class="form-group">
                            <label>{"化学配方"}</label>
                            <textarea
                                class="form-input"
                                value={self.form_chemical_formula.clone()}
                                oninput={on_formula_change}
                                placeholder="请输入化学配方"
                                rows="4"
                            />
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
                            {if is_edit { "保存修改" } else { "创建配方" }}
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}
