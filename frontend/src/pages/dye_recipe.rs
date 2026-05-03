use crate::utils::permissions;
use crate::utils::toast_helper;
/// 染色配方管理页面

use yew::prelude::*;
use crate::components::permission_guard::PermissionGuard;
use crate::utils::dom_helper;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
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
    loading: bool,
    error: Option<String>,
    filter_recipe_no: String,
    filter_color_code: String,
    filter_status: String,
    page: u64,
    page_size: u64,
    selected_recipe: Option<DyeRecipe>,
    show_modal: bool,
    modal_mode: ModalMode,
}

pub enum Msg {
    LoadRecipes,
    RecipesLoaded(Vec<DyeRecipe>),
    LoadError(String),
    SetFilterRecipeNo(String),
    SetFilterColorCode(String),
    SetFilterStatus(String),
    ViewRecipe(i32),
    OpenCreateModal,
    OpenEditModal(i32),
    ShowModalWithData(ModalMode, DyeRecipe),
    CloseModal,
    CreateRecipe(CreateDyeRecipeRequest),
    UpdateRecipe(i32, UpdateDyeRecipeRequest),
    OperationSuccess(String),
    ApproveRecipe(i32),
    DeleteRecipe(i32),
    ChangePage(u64),
}

impl Component for DyeRecipePage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            recipes: Vec::new(),
            loading: true,
            error: None,
            filter_recipe_no: String::new(),
            filter_color_code: String::new(),
            filter_status: String::from("全部"),
            page: 1,
            page_size: 20,
            selected_recipe: None,
            show_modal: false,
            modal_mode: ModalMode::View,
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
                let query = DyeRecipeQuery {
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                    recipe_no: if self.filter_recipe_no.is_empty() { None } else { Some(self.filter_recipe_no.clone()) },
                    color_code: if self.filter_color_code.is_empty() { None } else { Some(self.filter_color_code.clone()) },
                    status: if self.filter_status == "全部" { None } else { Some(self.filter_status.clone()) },
                    ..Default::default()
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match DyeRecipeService::list_with_query(&query).await {
                        Ok(recipes) => link.send_message(Msg::RecipesLoaded(recipes.items)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::RecipesLoaded(recipes) => {
                self.recipes = recipes;
                self.loading = false;
                true
            }
            Msg::LoadError(e) => {
                self.error = Some(e);
                self.loading = false;
                true
            }
            Msg::SetFilterRecipeNo(recipe_no) => {
                self.filter_recipe_no = recipe_no;
                ctx.link().send_message(Msg::LoadRecipes);
                false
            }
            Msg::SetFilterColorCode(color_code) => {
                self.filter_color_code = color_code;
                ctx.link().send_message(Msg::LoadRecipes);
                false
            }
            Msg::SetFilterStatus(status) => {
                self.filter_status = status;
                ctx.link().send_message(Msg::LoadRecipes);
                false
            }
            Msg::ViewRecipe(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match DyeRecipeService::get(id).await {
                        Ok(recipe) => {
                            link.send_message(Msg::ShowModalWithData(ModalMode::View, recipe));
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::OpenCreateModal => {
                self.modal_mode = ModalMode::Create;
                self.selected_recipe = None;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match DyeRecipeService::get(id).await {
                        Ok(recipe) => {
                            link.send_message(Msg::ShowModalWithData(ModalMode::Edit, recipe));
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ShowModalWithData(mode, recipe) => {
                self.modal_mode = mode;
                self.selected_recipe = Some(recipe);
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.selected_recipe = None;
                true
            }
            Msg::CreateRecipe(req) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match DyeRecipeService::create(req).await {
                        Ok(_) => {
                            link.send_message(Msg::CloseModal);
                            link.send_message(Msg::OperationSuccess("创建成功".to_string()));
                            link.send_message(Msg::LoadRecipes);
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::UpdateRecipe(id, req) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match DyeRecipeService::update(id, req).await {
                        Ok(_) => {
                            link.send_message(Msg::CloseModal);
                            link.send_message(Msg::OperationSuccess("更新成功".to_string()));
                            link.send_message(Msg::LoadRecipes);
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::OperationSuccess(msg) => {
                if let Some(win) = web_sys::window() { win.alert_with_message(&msg).ok(); }
                false
            }
            Msg::ApproveRecipe(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    let req = crate::models::dye_recipe::ApproveRecipeRequest {
                        approved_by: 1,
                    };
                    match DyeRecipeService::approve(id, req).await {
                        Ok(_) => link.send_message(Msg::LoadRecipes),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::DeleteRecipe(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match DyeRecipeService::delete(id).await {
                        Ok(_) => link.send_message(Msg::LoadRecipes),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ChangePage(page) => {
                self.page = page;
                ctx.link().send_message(Msg::LoadRecipes);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_recipe_no_change = ctx.link().batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlInputElement>().ok()?;
            Some(Msg::SetFilterRecipeNo(target.value()))
        });

        let on_color_code_change = ctx.link().batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlInputElement>().ok()?;
            Some(Msg::SetFilterColorCode(target.value()))
        });

        let on_status_change = ctx.link().batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlSelectElement>().ok()?;
            Some(Msg::SetFilterStatus(target.value()))
        });

        html! {
            <div class="dye-recipe-page">
                <div class="page-header">
                    <h1>{"🧪 染色配方管理"}</h1>
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::OpenCreateModal)}>
                        {"+ 新增配方"}
                    </button>
                </div>

                <div class="filter-bar">
                    <div class="filter-item">
                        <label>{"配方编号："}</label>
                        <input type="text" placeholder="请输入配方编号"
                            value={self.filter_recipe_no.clone()}
                            onchange={on_recipe_no_change}
                        />
                    </div>
                    <div class="filter-item">
                        <label>{"色号："}</label>
                        <input type="text" placeholder="请输入色号"
                            value={self.filter_color_code.clone()}
                            onchange={on_color_code_change}
                        />
                    </div>
                    <div class="filter-item">
                        <label>{"状态："}</label>
                        <select value={self.filter_status.clone()} onchange={on_status_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="草稿">{"草稿"}</option>
                            <option value="已审核">{"已审核"}</option>
                        </select>
                    </div>
                </div>

                {self.render_content(ctx)}

                if self.show_modal {
                    {self.render_modal(ctx)}
                }
            </div>
        }
    }
}

impl DyeRecipePage {
    fn render_content(&self, ctx: &Context<Self>) -> Html {
        if self.loading {
            return html! {
                <div class="loading-container">
                    <div class="spinner"></div>
                    <p>{"加载中..."}</p>
                </div>
            };
        }

        if let Some(error) = &self.error {
            return html! {
                <div class="error-container">
                    <div class="error-icon">{"⚠️"}</div>
                    <p class="error-message">{error}</p>
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadRecipes)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        if self.recipes.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"🧪"}</div>
                    <p>{"暂无配方数据"}</p>
                </div>
            };
        }

        html! {
            <div class="table-responsive">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>{"配方编号"}</th>
                            <th>{"色号"}</th>
                            <th>{"颜色名称"}</th>
                            <th>{"染色类型"}</th>
                            <th>{"温度(℃)"}</th>
                            <th>{"时间(分钟)"}</th>
                            <th>{"浴比"}</th>
                            <th>{"版本"}</th>
                            <th>{"状态"}</th>
                            <th>{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.recipes.iter().map(|recipe| {
                            let recipe_id = recipe.id;
                            let status = recipe.status.clone();
                            let is_draft = status == "草稿";
                            html! {
                                <tr>
                                    <td>{&recipe.recipe_no}</td>
                                    <td>{&recipe.color_code}</td>
                                    <td>{&recipe.color_name}</td>
                                    <td>{recipe.dye_type.as_deref().unwrap_or("-")}</td>
                                    <td class="numeric">{recipe.temperature.clone().map(|t| format!("{:.1}", t)).unwrap_or("-".to_string())}</td>
                                    <td class="numeric">{recipe.ph_value.clone().map(|p| format!("{:.1}", p)).unwrap_or("-".to_string())}</td>
                                    <td class="numeric">{recipe.liquor_ratio.clone().map(|l| format!("1:{}", l)).unwrap_or("-".to_string())}</td>
                                    <td class="numeric">{format!("V{}", recipe.version.unwrap_or(1))}</td>
                                    <td>
                                        <span class={format!("status-badge status-{}", if is_draft { "draft" } else { "approved" })}>
                                            {&status}
                                        </span>
                                    </td>
                                    <td class="actions">
                                        <button class="btn-small btn-info" style="margin-right: 5px;"
                                            onclick={ctx.link().callback(move |_| Msg::ViewRecipe(recipe_id))}>
                                            {"详情"}
                                        </button>
                                        if is_draft {
                                            <button class="btn-small btn-primary" style="margin-right: 5px;"
                                                onclick={ctx.link().callback(move |_| Msg::OpenEditModal(recipe_id))}>
                                                {"编辑"}
                                            </button>
                                        }
                                        if is_draft {
                                            <PermissionGuard resource="dye_recipe" action="approve">
<button class="btn-small btn-success"
                                                onclick={ctx.link().callback(move |_| Msg::ApproveRecipe(recipe_id))}>
                                                {"审核"}
                                            </button>
</PermissionGuard>
                                        }
                                        <PermissionGuard resource="dye_recipe" action="delete">
<button class="btn-small btn-danger"
                                            onclick={ctx.link().callback(move |_| Msg::DeleteRecipe(recipe_id))}>
                                            {"删除"}
                                        </button>
</PermissionGuard>
                                    </td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
            </div>
        }
    }


    fn render_modal(&self, ctx: &Context<Self>) -> Html {
        let on_close = ctx.link().callback(|_| Msg::CloseModal);
        
        if self.modal_mode == ModalMode::View {
            if let Some(recipe) = &self.selected_recipe {
                return html! {
                    <div class="modal-overlay" onclick={on_close.clone()}>
                        <div class="modal-content modal-lg" onclick={|e: MouseEvent| e.stop_propagation()}>
                            <div class="modal-header">
                                <h2>{"配方详情"}</h2>
                                <button class="modal-close" onclick={on_close.clone()}>
                                    {"×"}
                                </button>
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
                                        <span>{recipe.temperature.clone().map(|t| format!("{}℃", t)).unwrap_or("-".to_string())}</span>
                                    </div>
                                    <div class="detail-item">
                                        <label>{"时间："}</label>
                                        <span>{recipe.time_minutes.map(|t| format!("{}分钟", t)).unwrap_or("-".to_string())}</span>
                                    </div>
                                    <div class="detail-item">
                                        <label>{"pH值："}</label>
                                        <span>{recipe.ph_value.clone().map(|p| format!("{:.1}", p)).unwrap_or("-".to_string())}</span>
                                    </div>
                                    <div class="detail-item">
                                        <label>{"浴比："}</label>
                                        <span>{recipe.liquor_ratio.clone().map(|l| format!("1:{}", l)).unwrap_or("-".to_string())}</span>
                                    </div>
                                    <div class="detail-item">
                                        <label>{"版本："}</label>
                                        <span>{format!("V{}", recipe.version.unwrap_or(1))}</span>
                                    </div>
                                </div>
                                if let Some(formula) = &recipe.chemical_formula {
                                    <div class="detail-section">
                                        <h3>{"化学配方"}</h3>
                                        <pre>{formula}</pre>
                                    </div>
                                }
                                if let Some(aux) = &recipe.auxiliaries {
                                    <div class="detail-section">
                                        <h3>{"助剂信息"}</h3>
                                        <pre>{serde_json::to_string_pretty(aux).unwrap_or_default()}</pre>
                                    </div>
                                }
                            </div>
                            <div class="modal-footer">
                                <button class="btn-primary" onclick={on_close}>{"关闭"}</button>
                            </div>
                        </div>
                    </div>
                };
            } else {
                return html! {};
            }
        }
        
        let is_edit = self.modal_mode == ModalMode::Edit;
        let title = if is_edit { "编辑配方" } else { "新建配方" };
        
        let recipe = self.selected_recipe.clone().unwrap_or_else(|| DyeRecipe {
            id: 0,
            recipe_no: String::new(),
            color_code: String::new(),
            color_name: String::new(),
            fabric_type: None,
            dye_type: None,
            chemical_formula: None,
            temperature: None,
            time_minutes: None,
            ph_value: None,
            liquor_ratio: None,
            auxiliaries: None,
            status: "草稿".to_string(),
            version: Some(1),
            parent_recipe_id: None,
            remarks: None,
            created_by: None,
            created_at: String::new(),
            updated_at: String::new(),
            approved_by: None,
            approved_at: None,
        });

        html! {
            <div class="modal-overlay">
                <div class="modal-content modal-lg">
                    <div class="modal-header">
                        <h2>{title}</h2>
                        <button class="close-btn" onclick={on_close.clone()}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        <div class="form-group">
                            <label>{"配方编号"}</label>
                            <input type="text" id="recipe-no" value={recipe.recipe_no.clone()} disabled={is_edit} />
                        </div>
                        <div class="form-group">
                            <label>{"色号"}</label>
                            <input type="text" id="color-code" value={recipe.color_code.clone()} />
                        </div>
                        <div class="form-group">
                            <label>{"颜色名称"}</label>
                            <input type="text" id="color-name" value={recipe.color_name.clone()} />
                        </div>
                        <div class="form-group">
                            <label>{"面料类型"}</label>
                            <input type="text" id="fabric-type" value={recipe.fabric_type.clone().unwrap_or_default()} />
                        </div>
                        <div class="form-group">
                            <label>{"染色类型"}</label>
                            <input type="text" id="dye-type" value={recipe.dye_type.clone().unwrap_or_default()} />
                        </div>
                        <div class="form-group">
                            <label>{"温度(℃)"}</label>
                            <input type="text" id="temperature" value={recipe.temperature.clone().unwrap_or_default()} />
                        </div>
                        <div class="form-group">
                            <label>{"时间(分钟)"}</label>
                            <input type="number" id="time-minutes" value={recipe.time_minutes.map(|t| t.to_string()).unwrap_or_default()} />
                        </div>
                        <div class="form-group">
                            <label>{"pH值"}</label>
                            <input type="text" id="ph-value" value={recipe.ph_value.clone().unwrap_or_default()} />
                        </div>
                        <div class="form-group">
                            <label>{"化学配方"}</label>
                            <textarea id="chemical-formula" value={recipe.chemical_formula.clone().unwrap_or_default()}></textarea>
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn-secondary" onclick={on_close.clone()}>{"取消"}</button>
                        <button class="btn-primary" onclick={ctx.link().callback(move |_| {
                            let get_val = |id: &str| -> String { dom_helper::get_input_value(id).or_else(|| dom_helper::get_textarea_value(id)).unwrap_or_default() };
                            let get_opt = |id: &str| -> Option<String> { let v = get_val(id); if v.is_empty() { None } else { Some(v) } };
                            
                            if is_edit {
                                Msg::UpdateRecipe(recipe.id, UpdateDyeRecipeRequest {
                                    color_code: Some(get_val("color-code")),
                                    color_name: Some(get_val("color-name")),
                                    fabric_type: get_opt("fabric-type"),
                                    dye_type: get_opt("dye-type"),
                                    chemical_formula: get_opt("chemical-formula"),
                                    temperature: get_opt("temperature"),
                                    time_minutes: get_opt("time-minutes").and_then(|s| s.parse().ok()),
                                    ph_value: get_opt("ph-value"),
                                    liquor_ratio: None,
                                    auxiliaries: None,
                                    status: None,
                                    remarks: None,
                                })
                            } else {
                                Msg::CreateRecipe(CreateDyeRecipeRequest {
                                    recipe_no: get_val("recipe-no"),
                                    color_code: get_val("color-code"),
                                    color_name: get_val("color-name"),
                                    fabric_type: get_opt("fabric-type"),
                                    dye_type: get_opt("dye-type"),
                                    chemical_formula: get_opt("chemical-formula"),
                                    temperature: get_opt("temperature"),
                                    time_minutes: get_opt("time-minutes").and_then(|s| s.parse().ok()),
                                    ph_value: get_opt("ph-value"),
                                    liquor_ratio: None,
                                    auxiliaries: None,
                                    status: Some("草稿".to_string()),
                                    version: Some(1),
                                    parent_recipe_id: None,
                                    remarks: None,
                                    created_by: None,
                                })
                            }
                        })}>{"保存"}</button>
                    </div>
                </div>
            </div>
        }
    }
}
