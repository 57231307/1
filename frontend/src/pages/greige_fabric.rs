use crate::models::greige_fabric::{GreigeFabric, GreigeFabricQuery};
use crate::services::greige_fabric_service::GreigeFabricService;
use gloo_dialogs;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
/// 坯布管理页面（原料布匹管理）
use yew::prelude::*;

pub struct GreigeFabricPage {
    fabrics: Vec<GreigeFabric>,
    loading: bool,
    error: Option<String>,
    filter_fabric_no: String,
    filter_fabric_name: String,
    filter_status: String,
    page: u64,
    page_size: u64,
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
                    fabric_no: if self.filter_fabric_no.is_empty() {
                        None
                    } else {
                        Some(self.filter_fabric_no.clone())
                    },
                    fabric_name: if self.filter_fabric_name.is_empty() {
                        None
                    } else {
                        Some(self.filter_fabric_name.clone())
                    },
                    status: if self.filter_status == "全部" {
                        None
                    } else {
                        Some(self.filter_status.clone())
                    },
                    ..Default::default()
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match GreigeFabricService::list(query).await {
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
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_fabric_no_change = ctx.link().callback(|e: Event| {
            let target = e
                .target()
                .unwrap()
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap();
            Msg::SetFilterFabricNo(target.value())
        });

        let on_fabric_name_change = ctx.link().callback(|e: Event| {
            let target = e
                .target()
                .unwrap()
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap();
            Msg::SetFilterFabricName(target.value())
        });

        let on_status_change = ctx.link().callback(|e: Event| {
            let target = e
                .target()
                .unwrap()
                .dyn_into::<web_sys::HtmlSelectElement>()
                .unwrap();
            Msg::SetFilterStatus(target.value())
        });

        html! {
            <div class="greige-fabric-page">
                <div class="page-header">
                    <h1>{"📦 坯布管理"}</h1>
                    <button class="btn-primary" onclick={Callback::from(|_| gloo_dialogs::alert("功能开发中..."))}>
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
                                            <button class="btn-small btn-warning" onclick={Callback::from(|_| gloo_dialogs::alert("功能开发中..."))}>
                                                {"出库"}
                                            </button>
                                        }
                                        <button class="btn-small btn-danger"
                                            onclick={ctx.link().callback(move |_| Msg::DeleteFabric(fabric_id))}>
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
}
