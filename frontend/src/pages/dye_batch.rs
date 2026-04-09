use crate::components::main_layout::MainLayout;
use crate::models::dye_batch::DyeBatch;
use crate::services::dye_batch::DyeBatchService;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

pub struct DyeBatchPage {
    batches: Vec<DyeBatch>,
    loading: bool,
    error: Option<String>,
    show_create_form: bool,

    // Form fields
    new_batch_no: String,
    new_recipe_code: String,
    new_greige_code: String,
    new_total_weight_kg: String,
    new_status: String,
}

pub enum Msg {
    LoadBatches,
    BatchesLoaded(Vec<DyeBatch>),
    LoadError(String),
    ToggleCreateForm,
    UpdateNewField(String, String),
    SubmitCreate,
}

impl Component for DyeBatchPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            batches: Vec::new(),
            loading: true,
            error: None,
            show_create_form: false,
            new_batch_no: String::new(),
            new_recipe_code: String::new(),
            new_greige_code: String::new(),
            new_total_weight_kg: String::new(),
            new_status: String::from("排缸"),
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadBatches);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadBatches => {
                self.loading = true;
                let link = ctx.link().clone();
                spawn_local(async move {
                    match DyeBatchService::get_list().await {
                        Ok(res) => link.send_message(Msg::BatchesLoaded(res)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::BatchesLoaded(batches) => {
                self.batches = batches;
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
                    "batch_no" => self.new_batch_no = val,
                    "recipe_code" => self.new_recipe_code = val,
                    "greige_code" => self.new_greige_code = val,
                    "total_weight_kg" => self.new_total_weight_kg = val,
                    "status" => self.new_status = val,
                    _ => {}
                }
                true
            }
            Msg::SubmitCreate => {
                let req = DyeBatch {
                    id: 0,
                    batch_no: self.new_batch_no.clone(),
                    recipe_code: self.new_recipe_code.clone(),
                    greige_code: self.new_greige_code.clone(),
                    total_weight_kg: self.new_total_weight_kg.parse().unwrap_or(0.0),
                    status: self.new_status.clone(),
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match DyeBatchService::create(&req).await {
                        Ok(_) => {
                            link.send_message(Msg::ToggleCreateForm);
                            link.send_message(Msg::LoadBatches);
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
            <MainLayout current_page={"染缸批次"}>
                <div class="page-container">
                    <div class="page-header" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;">
                        <h1 style="font-size: 1.25rem; margin: 0;">{"🏭 染缸批次"}</h1>
                        <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::ToggleCreateForm)}>
                            {"+ 新增"}
                        </button>
                    </div>

                    if self.show_create_form {
                        <div class="create-form-container" style="background: var(--surface-color); padding: 1rem; border-radius: var(--radius-md); box-shadow: var(--shadow-sm); margin-bottom: 1rem; border: 1px solid var(--border-color);">
                            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 0.75rem; margin-bottom: 1rem;">
                                <input class="form-input" placeholder="缸号" value={self.new_batch_no.clone()} onchange={ctx.link().callback(|e: Event| Msg::UpdateNewField("batch_no".into(), e.target_unchecked_into::<web_sys::HtmlInputElement>().value()))} />
                                <input class="form-input" placeholder="配方编号" value={self.new_recipe_code.clone()} onchange={ctx.link().callback(|e: Event| Msg::UpdateNewField("recipe_code".into(), e.target_unchecked_into::<web_sys::HtmlInputElement>().value()))} />
                                <input class="form-input" placeholder="坯布编号" value={self.new_greige_code.clone()} onchange={ctx.link().callback(|e: Event| Msg::UpdateNewField("greige_code".into(), e.target_unchecked_into::<web_sys::HtmlInputElement>().value()))} />
                                <input class="form-input" placeholder="总重量(kg)" type="number" step="0.1" value={self.new_total_weight_kg.clone()} onchange={ctx.link().callback(|e: Event| Msg::UpdateNewField("total_weight_kg".into(), e.target_unchecked_into::<web_sys::HtmlInputElement>().value()))} />
                                <select class="form-input" value={self.new_status.clone()} onchange={ctx.link().callback(|e: Event| Msg::UpdateNewField("status".into(), e.target_unchecked_into::<web_sys::HtmlSelectElement>().value()))}>
                                    <option value="排缸">{"排缸"}</option>
                                    <option value="染色中">{"染色中"}</option>
                                    <option value="已完成">{"已完成"}</option>
                                </select>
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

impl DyeBatchPage {
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
                            <th style="padding: 0.5rem; text-align: left;">{"缸号"}</th>
                            <th style="padding: 0.5rem; text-align: left;">{"配方编号"}</th>
                            <th style="padding: 0.5rem; text-align: left;">{"坯布编号"}</th>
                            <th class="numeric-cell" style="padding: 0.5rem;">{"总重量(kg)"}</th>
                            <th style="padding: 0.5rem; text-align: center;">{"状态"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.batches.iter().map(|b| {
                            let status_bg = match b.status.as_str() {
                                "排缸" => "#9CA3AF",      // Gray
                                "染色中" => "#3B82F6",    // Blue
                                "已完成" => "#10B981",    // Green
                                _ => "#6B7280"
                            };
                            html! {
                                <tr style="border-bottom: 1px solid var(--border-color);">
                                    <td style="padding: 0.5rem;">{&b.batch_no}</td>
                                    <td style="padding: 0.5rem;">{&b.recipe_code}</td>
                                    <td style="padding: 0.5rem;">{&b.greige_code}</td>
                                    <td class="numeric-cell" style="padding: 0.5rem;">{format!("{:.2}", b.total_weight_kg)}</td>
                                    <td style="padding: 0.5rem; text-align: center;">
                                        <span class="status-badge" style={format!("background-color: {}; color: white; padding: 2px 6px; border-radius: 4px; font-size: 11px;", status_bg)}>
                                            {&b.status}
                                        </span>
                                    </td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
            </div>
        }
    }
}
