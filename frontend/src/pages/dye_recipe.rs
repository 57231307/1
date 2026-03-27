//! 染色配方管理页面

use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::services::dye_recipe_service::{
    DyeRecipeService, DyeRecipe, DyeRecipeQuery,
};

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
    show_detail_modal: bool,
}

pub enum Msg {
    LoadRecipes,
    RecipesLoaded(Vec<DyeRecipe>),
    LoadError(String),
    SetFilterRecipeNo(String),
    SetFilterColorCode(String),
    SetFilterStatus(String),
    ViewRecipe(i32),
    CloseDetail,
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
            show_detail_modal: false,
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
                    match DyeRecipeService::list(query).await {
                        Ok(recipes) => link.send_message(Msg::RecipesLoaded(recipes)),
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
                self.selected_recipe = self.recipes.iter().find(|r| r.id == id).cloned();
                self.show_detail_modal = true;
                true
            }
            Msg::CloseDetail => {
                self.show_detail_modal = false;
                self.selected_recipe = None;
                true
            }
            Msg::ApproveRecipe(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    let req = crate::services::dye_recipe_service::ApproveRecipeRequest {
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
        let on_recipe_no_change = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
            Msg::SetFilterRecipeNo(target.value())
        });

        let on_color_code_change = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
            Msg::SetFilterColorCode(target.value())
        });

        let on_status_change = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<web_sys::HtmlSelectElement>().unwrap();
            Msg::SetFilterStatus(target.value())
        });

        html! {
            <div class="dye-recipe-page">
                <div class="page-header">
                    <h1>{"🧪 染色配方管理"}</h1>
                    <button class="btn-primary">
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

                if self.show_detail_modal {
                    {self.render_detail_modal(ctx)}
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
                                    <td class="numeric">{recipe.temperature.map(|t| format!("{:.1}", t)).unwrap_or("-".to_string())}</td>
                                    <td class="numeric">{recipe.time_minutes.map(|t| t.to_string()).unwrap_or("-".to_string())}</td>
                                    <td class="numeric">{recipe.liquor_ratio.map(|l| format!("1:{}", l)).unwrap_or("-".to_string())}</td>
                                    <td class="numeric">{format!("V{}", recipe.version.unwrap_or(1))}</td>
                                    <td>
                                        <span class={format!("status-badge status-{}", if is_draft { "draft" } else { "approved" })}>
                                            {&status}
                                        </span>
                                    </td>
                                    <td class="actions">
                                        <button class="btn-small btn-info"
                                            onclick={ctx.link().callback(move |_| Msg::ViewRecipe(recipe_id))}>
                                            {"详情"}
                                        </button>
                                        if is_draft {
                                            <button class="btn-small btn-success"
                                                onclick={ctx.link().callback(move |_| Msg::ApproveRecipe(recipe_id))}>
                                                {"审核"}
                                            </button>
                                        }
                                        <button class="btn-small btn-danger"
                                            onclick={ctx.link().callback(move |_| Msg::DeleteRecipe(recipe_id))}>
                                            {"删除"}
                                        </button>
                                    </td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
            </div>
        }
    }

    fn render_detail_modal(&self, ctx: &Context<Self>) -> Html {
        if let Some(recipe) = &self.selected_recipe {
            html! {
                <div class="modal-overlay" onclick={ctx.link().callback(|_| Msg::CloseDetail)}>
                    <div class="modal-content modal-lg" onclick={|e: MouseEvent| e.stop_propagation()}>
                        <div class="modal-header">
                            <h2>{"配方详情"}</h2>
                            <button class="modal-close" onclick={ctx.link().callback(|_| Msg::CloseDetail)}>
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
                                    <span>{recipe.temperature.map(|t| format!("{}℃", t)).unwrap_or("-".to_string())}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"时间："}</label>
                                    <span>{recipe.time_minutes.map(|t| format!("{}分钟", t)).unwrap_or("-".to_string())}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"pH值："}</label>
                                    <span>{recipe.ph_value.map(|p| format!("{:.1}", p)).unwrap_or("-".to_string())}</span>
                                </div>
                                <div class="detail-item">
                                    <label>{"浴比："}</label>
                                    <span>{recipe.liquor_ratio.map(|l| format!("1:{}", l)).unwrap_or("-".to_string())}</span>
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
                    </div>
                </div>
            }
        } else {
            html! {}
        }
    }
}
