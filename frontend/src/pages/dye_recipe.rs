use crate::components::main_layout::MainLayout;
use crate::models::dye_recipe::DyeRecipe;
use crate::services::dye_recipe::DyeRecipeService;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

pub struct DyeRecipePage {
    recipes: Vec<DyeRecipe>,
    loading: bool,
    error: Option<String>,
    show_create_form: bool,

    // Form fields
    new_recipe_code: String,
    new_color_name: String,
    new_fabric_type: String,
    new_dyes: String,
    new_temp_c: String,
    new_time_mins: String,
}

pub enum Msg {
    LoadRecipes,
    RecipesLoaded(Vec<DyeRecipe>),
    LoadError(String),
    ToggleCreateForm,
    UpdateNewField(String, String),
    SubmitCreate,
}

impl Component for DyeRecipePage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            recipes: Vec::new(),
            loading: true,
            error: None,
            show_create_form: false,
            new_recipe_code: String::new(),
            new_color_name: String::new(),
            new_fabric_type: String::new(),
            new_dyes: String::new(),
            new_temp_c: String::new(),
            new_time_mins: String::new(),
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
                let link = ctx.link().clone();
                spawn_local(async move {
                    match DyeRecipeService::get_list().await {
                        Ok(res) => link.send_message(Msg::RecipesLoaded(res)),
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
            Msg::ToggleCreateForm => {
                self.show_create_form = !self.show_create_form;
                true
            }
            Msg::UpdateNewField(field, val) => {
                match field.as_str() {
                    "recipe_code" => self.new_recipe_code = val,
                    "color_name" => self.new_color_name = val,
                    "fabric_type" => self.new_fabric_type = val,
                    "dyes" => self.new_dyes = val,
                    "temp_c" => self.new_temp_c = val,
                    "time_mins" => self.new_time_mins = val,
                    _ => {}
                }
                true
            }
            Msg::SubmitCreate => {
                let req = DyeRecipe {
                    id: 0,
                    recipe_code: self.new_recipe_code.clone(),
                    color_name: self.new_color_name.clone(),
                    fabric_type: self.new_fabric_type.clone(),
                    dyes: self.new_dyes.clone(),
                    temp_c: self.new_temp_c.parse().unwrap_or(0),
                    time_mins: self.new_time_mins.parse().unwrap_or(0),
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match DyeRecipeService::create(&req).await {
                        Ok(_) => {
                            link.send_message(Msg::ToggleCreateForm);
                            link.send_message(Msg::LoadRecipes);
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <MainLayout current_page={"染化料配方"}>
                <div class="page-container">
                    <div class="page-header" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;">
                        <h1 style="font-size: 1.25rem; margin: 0;">{"🧪 染化料配方"}</h1>
                        <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::ToggleCreateForm)}>
                            {"+ 新增"}
                        </button>
                    </div>

                    if self.show_create_form {
                        <div class="create-form-container" style="background: var(--surface-color); padding: 1rem; border-radius: var(--radius-md); box-shadow: var(--shadow-sm); margin-bottom: 1rem; border: 1px solid var(--border-color);">
                            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 0.75rem; margin-bottom: 1rem;">
                                <input class="form-input" placeholder="配方编号" value={self.new_recipe_code.clone()} onchange={ctx.link().callback(|e: Event| Msg::UpdateNewField("recipe_code".into(), e.target_unchecked_into::<web_sys::HtmlInputElement>().value()))} />
                                <input class="form-input" placeholder="颜色名称" value={self.new_color_name.clone()} onchange={ctx.link().callback(|e: Event| Msg::UpdateNewField("color_name".into(), e.target_unchecked_into::<web_sys::HtmlInputElement>().value()))} />
                                <input class="form-input" placeholder="面料类型" value={self.new_fabric_type.clone()} onchange={ctx.link().callback(|e: Event| Msg::UpdateNewField("fabric_type".into(), e.target_unchecked_into::<web_sys::HtmlInputElement>().value()))} />
                                <input class="form-input" placeholder="染料" value={self.new_dyes.clone()} onchange={ctx.link().callback(|e: Event| Msg::UpdateNewField("dyes".into(), e.target_unchecked_into::<web_sys::HtmlInputElement>().value()))} />
                                <input class="form-input" placeholder="温度(℃)" type="number" value={self.new_temp_c.clone()} onchange={ctx.link().callback(|e: Event| Msg::UpdateNewField("temp_c".into(), e.target_unchecked_into::<web_sys::HtmlInputElement>().value()))} />
                                <input class="form-input" placeholder="时间(分)" type="number" value={self.new_time_mins.clone()} onchange={ctx.link().callback(|e: Event| Msg::UpdateNewField("time_mins".into(), e.target_unchecked_into::<web_sys::HtmlInputElement>().value()))} />
                            </div>
                            <div style="display: flex; justify-content: flex-end; gap: 0.5rem;">
                                <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::ToggleCreateForm)}>{"取消"}</button>
                                <button class="btn-success" onclick={ctx.link().callback(|_| Msg::SubmitCreate)}>{"保存"}</button>
                            </div>
                        </div>
                    }

                    {self.render_table(ctx)}
                </div>
            </MainLayout>
        }
    }
}

impl DyeRecipePage {
    fn render_table(&self, _ctx: &Context<Self>) -> Html {
        if self.loading {
            return html! { <div style="padding: 2rem; text-align: center;">{"加载中..."}</div> };
        }

        if let Some(err) = &self.error {
            return html! { <div style="padding: 2rem; color: red; text-align: center;">{err}</div> };
        }

        html! {
            <div style="overflow-x: auto; background: var(--surface-color); border-radius: var(--radius-md); border: 1px solid var(--border-color);">
                <table class="data-table" style="width: 100%; border-collapse: collapse; font-size: 13px;">
                    <thead style="background: #f9fafb; border-bottom: 2px solid var(--border-color);">
                        <tr>
                            <th style="padding: 0.5rem; text-align: left;">{"配方编号"}</th>
                            <th style="padding: 0.5rem; text-align: left;">{"颜色名称"}</th>
                            <th style="padding: 0.5rem; text-align: left;">{"面料类型"}</th>
                            <th style="padding: 0.5rem; text-align: left;">{"染料"}</th>
                            <th class="numeric-cell" style="padding: 0.5rem;">{"温度(℃)"}</th>
                            <th class="numeric-cell" style="padding: 0.5rem;">{"时间(分)"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.recipes.iter().map(|r| {
                            html! {
                                <tr style="border-bottom: 1px solid var(--border-color);">
                                    <td style="padding: 0.5rem;">{&r.recipe_code}</td>
                                    <td style="padding: 0.5rem;">{&r.color_name}</td>
                                    <td style="padding: 0.5rem;">{&r.fabric_type}</td>
                                    <td style="padding: 0.5rem;">{&r.dyes}</td>
                                    <td class="numeric-cell" style="padding: 0.5rem;">{r.temp_c}</td>
                                    <td class="numeric-cell" style="padding: 0.5rem;">{r.time_mins}</td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
            </div>
        }
    }
}
