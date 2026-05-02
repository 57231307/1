import re

with open("src/pages/greige_fabric.rs", "r") as f:
    content = f.read()

# Add imports
if "use crate::models::greige_fabric::{CreateGreigeFabricRequest, UpdateGreigeFabricRequest" not in content:
    content = content.replace(
        "use crate::models::greige_fabric::{",
        "use crate::models::greige_fabric::{\n    CreateGreigeFabricRequest, UpdateGreigeFabricRequest,"
    )

# Add ModalMode enum
if "pub enum ModalMode {" not in content:
    content = content.replace("pub struct GreigeFabricPage {", """
#[derive(Clone, PartialEq)]
pub enum ModalMode {
    Create,
    Edit,
}

pub struct GreigeFabricPage {""")

# Add state fields
content = content.replace(
    "    page_size: u64,\n}",
    "    page_size: u64,\n    modal_mode: ModalMode,\n    selected_fabric: Option<GreigeFabric>,\n    show_modal: bool,\n}"
)

# Initialize state fields
content = content.replace(
    "            page_size: 20,\n        }",
    "            page_size: 20,\n            modal_mode: ModalMode::Create,\n            selected_fabric: None,\n            show_modal: false,\n        }"
)

# Add Msg variants
content = content.replace(
    "    ChangePage(u64),\n}",
    "    ChangePage(u64),\n    OpenCreateModal,\n    OpenEditModal(i32),\n    ShowModalWithData(ModalMode, GreigeFabric),\n    CloseModal,\n    CreateFabric(CreateGreigeFabricRequest),\n    UpdateFabric(i32, UpdateGreigeFabricRequest),\n    OperationSuccess(String),\n}"
)

# Handle new Msgs in update()
update_match = re.search(r'(\s*Msg::ChangePage\(page\) => \{.*?\n\s*false\n\s*\})', content, re.DOTALL)
if update_match:
    new_msgs = """
            Msg::OpenCreateModal => {
                self.modal_mode = ModalMode::Create;
                self.selected_fabric = None;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match GreigeFabricService::get(id).await {
                        Ok(fabric) => {
                            link.send_message(Msg::ShowModalWithData(ModalMode::Edit, fabric));
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ShowModalWithData(mode, fabric) => {
                self.modal_mode = mode;
                self.selected_fabric = Some(fabric);
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.selected_fabric = None;
                true
            }
            Msg::CreateFabric(req) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match GreigeFabricService::create(req).await {
                        Ok(_) => {
                            link.send_message(Msg::CloseModal);
                            link.send_message(Msg::OperationSuccess("创建成功".to_string()));
                            link.send_message(Msg::LoadFabrics);
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::UpdateFabric(id, req) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match GreigeFabricService::update(id, req).await {
                        Ok(_) => {
                            link.send_message(Msg::CloseModal);
                            link.send_message(Msg::OperationSuccess("更新成功".to_string()));
                            link.send_message(Msg::LoadFabrics);
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
    content = content[:update_match.end()] + new_msgs + content[update_match.end():]

# Fix the button
content = content.replace(
    """onclick={Callback::from(|_| gloo_dialogs::alert("功能开发中..."))}>
                        {"+ 新增坯布"}""",
    """onclick={ctx.link().callback(|_| Msg::OpenCreateModal)}>
                        {"+ 新增坯布"}"""
)

# Add Edit button next to Delete
content = content.replace(
    """<button class="btn-small btn-danger"
                                            onclick={ctx.link().callback(move |_| Msg::DeleteFabric(fabric_id))}>""",
    """<button class="btn-small btn-primary" style="margin-right: 5px;"
                                            onclick={ctx.link().callback(move |_| Msg::OpenEditModal(fabric_id))}>
                                            {"编辑"}
                                        </button>
                                        <button class="btn-small btn-danger"
                                            onclick={ctx.link().callback(move |_| Msg::DeleteFabric(fabric_id))}>"""
)

# Add modal rendering to view()
view_end = content.find("                {self.render_content(ctx)}\n            </div>")
if view_end != -1:
    content = content.replace("                {self.render_content(ctx)}\n            </div>", 
"""                {self.render_content(ctx)}
                
                if self.show_modal {
                    {self.render_modal(ctx)}
                }
            </div>""")

# Add render_modal function
impl_end = content.rfind("}")
if impl_end != -1:
    render_modal = """
    fn render_modal(&self, ctx: &Context<Self>) -> Html {
        let on_close = ctx.link().callback(|_| Msg::CloseModal);
        
        let (title, is_edit) = match self.modal_mode {
            ModalMode::Create => ("新建坯布", false),
            ModalMode::Edit => ("编辑坯布", true),
        };

        let fabric = self.selected_fabric.clone().unwrap_or_else(|| GreigeFabric {
            id: 0,
            fabric_no: String::new(),
            fabric_name: String::new(),
            fabric_type: String::new(),
            color_code: None,
            width_cm: None,
            weight_kg: None,
            length_m: None,
            supplier_id: None,
            batch_no: None,
            warehouse_id: None,
            location: None,
            status: "在库".to_string(),
            quality_grade: None,
            purchase_date: None,
            remarks: None,
            created_by: None,
            created_at: String::new(),
            updated_at: String::new(),
        });

        html! {
            <div class="modal-overlay">
                <div class="modal-content">
                    <div class="modal-header">
                        <h2>{title}</h2>
                        <button class="close-btn" onclick={on_close.clone()}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        <div class="form-group">
                            <label>{"坯布编号"}</label>
                            <input type="text" id="fabric-no" value={fabric.fabric_no.clone()} disabled={is_edit} />
                        </div>
                        <div class="form-group">
                            <label>{"坯布名称"}</label>
                            <input type="text" id="fabric-name" value={fabric.fabric_name.clone()} />
                        </div>
                        <div class="form-group">
                            <label>{"坯布类型"}</label>
                            <input type="text" id="fabric-type" value={fabric.fabric_type.clone()} />
                        </div>
                        <div class="form-group">
                            <label>{"幅宽(cm)"}</label>
                            <input type="text" id="fabric-width" value={fabric.width_cm.clone().unwrap_or_default()} />
                        </div>
                        <div class="form-group">
                            <label>{"重量(kg)"}</label>
                            <input type="text" id="fabric-weight" value={fabric.weight_kg.clone().unwrap_or_default()} />
                        </div>
                        <div class="form-group">
                            <label>{"长度(m)"}</label>
                            <input type="text" id="fabric-length" value={fabric.length_m.clone().unwrap_or_default()} />
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn-secondary" onclick={on_close.clone()}>{"取消"}</button>
                        <button class="btn-primary" onclick={ctx.link().callback(move |_| {
                            let get_val = |id: &str| -> String {
                                web_sys::window().unwrap().document().unwrap().get_element_by_id(id).unwrap()
                                    .dyn_into::<web_sys::HtmlInputElement>().unwrap().value()
                            };
                            let get_opt = |id: &str| -> Option<String> {
                                let v = get_val(id);
                                if v.is_empty() { None } else { Some(v) }
                            };
                            
                            if is_edit {
                                Msg::UpdateFabric(fabric.id, UpdateGreigeFabricRequest {
                                    fabric_name: Some(get_val("fabric-name")),
                                    fabric_type: Some(get_val("fabric-type")),
                                    color_code: None,
                                    width_cm: get_opt("fabric-width"),
                                    weight_kg: get_opt("fabric-weight"),
                                    length_m: get_opt("fabric-length"),
                                    supplier_id: None,
                                    batch_no: None,
                                    warehouse_id: None,
                                    location: None,
                                    status: None,
                                    quality_grade: None,
                                    purchase_date: None,
                                    remarks: None,
                                })
                            } else {
                                Msg::CreateFabric(CreateGreigeFabricRequest {
                                    fabric_no: get_val("fabric-no"),
                                    fabric_name: get_val("fabric-name"),
                                    fabric_type: get_val("fabric-type"),
                                    color_code: None,
                                    width_cm: get_opt("fabric-width"),
                                    weight_kg: get_opt("fabric-weight"),
                                    length_m: get_opt("fabric-length"),
                                    supplier_id: None,
                                    batch_no: None,
                                    warehouse_id: None,
                                    location: None,
                                    status: Some("在库".to_string()),
                                    quality_grade: None,
                                    purchase_date: None,
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
"""
    content = content[:impl_end] + render_modal + content[impl_end:]

with open("src/pages/greige_fabric.rs", "w") as f:
    f.write(content)
