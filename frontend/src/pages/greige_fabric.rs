use crate::utils::permissions;
use crate::utils::toast_helper;
/// 坯布管理页面（原料布匹管理）

use yew::prelude::*;
use crate::components::permission_guard::PermissionGuard;
use crate::utils::dom_helper;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::models::greige_fabric::{
    CreateGreigeFabricRequest, UpdateGreigeFabricRequest,
    GreigeFabric, GreigeFabricQuery,
};
use crate::services::greige_fabric_service::GreigeFabricService;
use crate::services::crud_service::CrudService;


#[derive(Clone, PartialEq)]
pub enum ModalMode {
    Create,
    Edit,
}

pub struct GreigeFabricPage {
    fabrics: Vec<GreigeFabric>,
    loading: bool,
    error: Option<String>,
    filter_fabric_no: String,
    filter_fabric_name: String,
    filter_status: String,
    page: u64,
    page_size: u64,
    modal_mode: ModalMode,
    selected_fabric: Option<GreigeFabric>,
    show_modal: bool,
}

pub enum Msg {
    LoadFabrics,
    FabricsLoaded(Vec<GreigeFabric>),
    LoadError(String),
    SetFilterFabricNo(String),
    SetFilterFabricName(String),
    SetFilterStatus(String),
    DeleteFabric(i32),
    ChangePage(u64),
    OpenCreateModal,
    OpenEditModal(i32),
    ShowModalWithData(ModalMode, GreigeFabric),
    CloseModal,
    CreateFabric(CreateGreigeFabricRequest),
    UpdateFabric(i32, UpdateGreigeFabricRequest),
    OperationSuccess(String),
    StockOut(i32),
}

impl Component for GreigeFabricPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            fabrics: Vec::new(),
            loading: true,
            error: None,
            filter_fabric_no: String::new(),
            filter_fabric_name: String::new(),
            filter_status: String::from("全部"),
            page: 1,
            page_size: 20,
            modal_mode: ModalMode::Create,
            selected_fabric: None,
            show_modal: false,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadFabrics);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadFabrics => {
                self.loading = true;
                let query = GreigeFabricQuery {
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                    fabric_no: if self.filter_fabric_no.is_empty() { None } else { Some(self.filter_fabric_no.clone()) },
                    fabric_name: if self.filter_fabric_name.is_empty() { None } else { Some(self.filter_fabric_name.clone()) },
                    status: if self.filter_status == "全部" { None } else { Some(self.filter_status.clone()) },
                    ..Default::default()
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match GreigeFabricService::list_with_query(&query).await {
                        Ok(fabrics) => link.send_message(Msg::FabricsLoaded(fabrics.items)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::FabricsLoaded(fabrics) => {
                self.fabrics = fabrics;
                self.loading = false;
                true
            }
            Msg::LoadError(e) => {
                self.error = Some(e);
                self.loading = false;
                true
            }
            Msg::SetFilterFabricNo(fabric_no) => {
                self.filter_fabric_no = fabric_no;
                ctx.link().send_message(Msg::LoadFabrics);
                false
            }
            Msg::SetFilterFabricName(fabric_name) => {
                self.filter_fabric_name = fabric_name;
                ctx.link().send_message(Msg::LoadFabrics);
                false
            }
            Msg::SetFilterStatus(status) => {
                self.filter_status = status;
                ctx.link().send_message(Msg::LoadFabrics);
                false
            }
            Msg::DeleteFabric(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match GreigeFabricService::delete(id).await {
                        Ok(_) => link.send_message(Msg::LoadFabrics),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ChangePage(page) => {
                self.page = page;
                ctx.link().send_message(Msg::LoadFabrics);
                false
            }
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
            }
            Msg::StockOut(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match GreigeFabricService::stock_out(id, crate::models::greige_fabric::StockOutRequest {
                        weight_kg: None,
                        length_m: None,
                        remarks: Some("手动出库".to_string()),
                    }).await {
                        Ok(_) => {
                            link.send_message(Msg::OperationSuccess("出库成功".to_string()));
                            link.send_message(Msg::LoadFabrics);
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_fabric_no_change = ctx.link().batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlInputElement>().ok()?;
            Some(Msg::SetFilterFabricNo(target.value()))
        });

        let on_fabric_name_change = ctx.link().batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlInputElement>().ok()?;
            Some(Msg::SetFilterFabricName(target.value()))
        });

        let on_status_change = ctx.link().batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlSelectElement>().ok()?;
            Some(Msg::SetFilterStatus(target.value()))
        });

        html! {
            <div class="greige-fabric-page">
                <div class="page-header">
                    <h1>{"📦 坯布管理"}</h1>
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::OpenCreateModal)}>
                        {"+ 新增坯布"}
                    </button>
                </div>

                <div class="filter-bar">
                    <div class="filter-item">
                        <label>{"坯布编号："}</label>
                        <input type="text" placeholder="请输入坯布编号"
                            value={self.filter_fabric_no.clone()}
                            onchange={on_fabric_no_change}
                        />
                    </div>
                    <div class="filter-item">
                        <label>{"坯布名称："}</label>
                        <input type="text" placeholder="请输入坯布名称"
                            value={self.filter_fabric_name.clone()}
                            onchange={on_fabric_name_change}
                        />
                    </div>
                    <div class="filter-item">
                        <label>{"状态："}</label>
                        <select value={self.filter_status.clone()} onchange={on_status_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="在库">{"在库"}</option>
                            <option value="已出库">{"已出库"}</option>
                            <option value="待入库">{"待入库"}</option>
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

impl GreigeFabricPage {
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
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadFabrics)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        if self.fabrics.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"📦"}</div>
                    <p>{"暂无坯布数据"}</p>
                </div>
            };
        }

        html! {
            <div class="table-responsive">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>{"坯布编号"}</th>
                            <th>{"坯布名称"}</th>
                            <th>{"坯布类型"}</th>
                            <th>{"幅宽(cm)"}</th>
                            <th>{"重量(kg)"}</th>
                            <th>{"长度(m)"}</th>
                            <th>{"状态"}</th>
                            <th>{"质量等级"}</th>
                            <th>{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.fabrics.iter().map(|fabric| {
                            let fabric_id = fabric.id;
                            let status = fabric.status.clone();
                            html! {
                                <tr>
                                    <td>{&fabric.fabric_no}</td>
                                    <td>{&fabric.fabric_name}</td>
                                    <td>{&fabric.fabric_type}</td>
                                    <td class="numeric">{fabric.width_cm.clone().map(|w| format!("{:.1}", w)).unwrap_or("-".to_string())}</td>
                                    <td class="numeric">{fabric.weight_kg.clone().map(|w| format!("{:.2}", w)).unwrap_or("-".to_string())}</td>
                                    <td class="numeric">{fabric.length_m.clone().map(|l| format!("{:.2}", l)).unwrap_or("-".to_string())}</td>
                                    <td>
                                        <span class={format!("status-badge status-{}", status)}>
                                            {&status}
                                        </span>
                                    </td>
                                    <td>{fabric.quality_grade.as_deref().unwrap_or("-")}</td>
                                    <td class="actions">
                                        if status == "在库" {
                                            <button class="btn-small btn-warning" onclick={ctx.link().callback(move |_| Msg::StockOut(fabric_id))}>
                                                {"出库"}
                                            </button>
                                        }
                                        <button class="btn-small btn-primary" style="margin-right: 5px;"
                                            onclick={ctx.link().callback(move |_| Msg::OpenEditModal(fabric_id))}>
                                            {"编辑"}
                                        </button>
                                        <PermissionGuard resource="greige_fabric" action="delete">
<button class="btn-small btn-danger"
                                            onclick={ctx.link().callback(move |_| Msg::DeleteFabric(fabric_id))}>
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
                            let get_val = |id: &str| -> String { dom_helper::get_input_value(id).or_else(|| dom_helper::get_textarea_value(id)).unwrap_or_default() };
                            let get_opt = |id: &str| -> Option<String> { let v = get_val(id); if v.is_empty() { None } else { Some(v) } };
                            
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
}
