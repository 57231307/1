import re

with open("src/pages/dye_recipe.rs", "r") as f:
    content = f.read()

# 1. Imports
if "CreateDyeRecipeRequest" not in content:
    content = content.replace(
        "use crate::models::dye_recipe::{",
        "use crate::models::dye_recipe::{\n    CreateDyeRecipeRequest, UpdateDyeRecipeRequest,"
    )

# 2. Add ModalMode
if "pub enum ModalMode {" not in content:
    content = content.replace("pub struct DyeRecipePage {", """
#[derive(Clone, PartialEq)]
pub enum ModalMode {
    Create,
    Edit,
    View,
}

pub struct DyeRecipePage {""")

# 3. Modify state fields
content = content.replace(
    "    selected_recipe: Option<DyeRecipe>,\n    show_detail_modal: bool,\n}",
    "    selected_recipe: Option<DyeRecipe>,\n    show_modal: bool,\n    modal_mode: ModalMode,\n}"
)
content = content.replace(
    "            selected_recipe: None,\n            show_detail_modal: false,\n        }",
    "            selected_recipe: None,\n            show_modal: false,\n            modal_mode: ModalMode::View,\n        }"
)

# 4. Update Msg variants
content = content.replace(
    "    ViewRecipe(i32),\n    CloseDetail,",
    "    ViewRecipe(i32),\n    OpenCreateModal,\n    OpenEditModal(i32),\n    ShowModalWithData(ModalMode, DyeRecipe),\n    CloseModal,\n    CreateRecipe(CreateDyeRecipeRequest),\n    UpdateRecipe(i32, UpdateDyeRecipeRequest),\n    OperationSuccess(String),"
)

# 5. Update update() match arms
# Replace ViewRecipe and CloseDetail
update_match = re.search(r'(\s*Msg::ViewRecipe\(id\) => \{.*?\n\s*true\n\s*\}\n\s*Msg::CloseDetail => \{.*?\n\s*true\n\s*\})', content, re.DOTALL)
if update_match:
    new_msgs = """
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
            }"""
    content = content[:update_match.start()] + new_msgs + content[update_match.end():]

# 6. Change buttons
content = content.replace(
    """onclick={Callback::from(|_| gloo_dialogs::alert("功能开发中..."))}>
                        {"+ 新增配方"}""",
    """onclick={ctx.link().callback(|_| Msg::OpenCreateModal)}>
                        {"+ 新增配方"}"""
)

# Edit button in list
content = content.replace(
    """                                        <button class="btn-small btn-info"
                                            onclick={ctx.link().callback(move |_| Msg::ViewRecipe(recipe_id))}>
                                            {"详情"}
                                        </button>""",
    """                                        <button class="btn-small btn-info" style="margin-right: 5px;"
                                            onclick={ctx.link().callback(move |_| Msg::ViewRecipe(recipe_id))}>
                                            {"详情"}
                                        </button>
                                        if is_draft {
                                            <button class="btn-small btn-primary" style="margin-right: 5px;"
                                                onclick={ctx.link().callback(move |_| Msg::OpenEditModal(recipe_id))}>
                                                {"编辑"}
                                            </button>
                                        }"""
)

# 7. Render modal
content = content.replace(
    """                if self.show_detail_modal {
                    {self.render_detail_modal(ctx)}
                }""",
    """                if self.show_modal {
                    {self.render_modal(ctx)}
                }"""
)

# 8. Modify render_detail_modal to render_modal
# Find the start of `fn render_detail_modal`
idx = content.find("    fn render_detail_modal(&self, ctx: &Context<Self>) -> Html {")
if idx != -1:
    render_modal = """
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
                            let get_val = |id: &str| -> String {
                                web_sys::window().unwrap().document().unwrap().get_element_by_id(id).unwrap()
                                    .dyn_into::<web_sys::HtmlInputElement>().map(|e| e.value())
                                    .or_else(|_| web_sys::window().unwrap().document().unwrap().get_element_by_id(id).unwrap().dyn_into::<web_sys::HtmlTextAreaElement>().map(|e| e.value()))
                                    .unwrap_or_default()
                            };
                            let get_opt = |id: &str| -> Option<String> {
                                let v = get_val(id);
                                if v.is_empty() { None } else { Some(v) }
                            };
                            
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
"""
    content = content[:idx] + render_modal

with open("src/pages/dye_recipe.rs", "w") as f:
    f.write(content)

