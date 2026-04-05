//! 供应商管理页面

use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::models::supplier::{
    Supplier, SupplierQuery, CreateSupplierRequest, UpdateSupplierRequest,
};
use crate::services::supplier_service::SupplierService;

pub struct SupplierPage {
    suppliers: Vec<Supplier>,
    loading: bool,
    error: Option<String>,
    show_modal: bool,
    modal_mode: ModalMode,
    current_supplier: Option<Supplier>,
    filter_status: String,
    filter_type: String,
    keyword: String,
    page: u64,
    page_size: u64,
}

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    View,
    Create,
    Edit,
}

pub enum Msg {
    LoadSuppliers,
    SuppliersLoaded(crate::models::supplier::SupplierListResponse),
    LoadError(String),
    SetFilterStatus(String),
    SetFilterType(String),
    SetKeyword(String),
    OpenModal(ModalMode, Option<Supplier>),
    CloseModal,
    CreateSupplier(CreateSupplierRequest),
    UpdateSupplier(i32, UpdateSupplierRequest),
    DeleteSupplier(i32),
    ToggleStatus(i32, bool),
    ChangePage(u64),
}

impl Component for SupplierPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            suppliers: Vec::new(),
            loading: true,
            error: None,
            show_modal: false,
            modal_mode: ModalMode::View,
            current_supplier: None,
            filter_status: String::from("全部"),
            filter_type: String::from("全部"),
            keyword: String::new(),
            page: 1,
            page_size: 20,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadSuppliers);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadSuppliers => {
                self.loading = true;
                let query = SupplierQuery {
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                    status: if self.filter_status == "全部" { None } else { Some(self.filter_status.clone()) },
                    supplier_type: if self.filter_type == "全部" { None } else { Some(self.filter_type.clone()) },
                    keyword: if self.keyword.is_empty() { None } else { Some(self.keyword.clone()) },
                    grade: None,
                    sort_by: None,
                    sort_order: None,
                    is_enabled: None,
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match SupplierService::list(query).await {
                        Ok(suppliers) => link.send_message(Msg::SuppliersLoaded(suppliers)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::SuppliersLoaded(response) => {
                self.suppliers = response.data;
                self.loading = false;
                true
            }
            Msg::LoadError(e) => {
                self.error = Some(e);
                self.loading = false;
                true
            }
            Msg::SetFilterStatus(status) => {
                self.filter_status = status;
                ctx.link().send_message(Msg::LoadSuppliers);
                false
            }
            Msg::SetFilterType(filter_type) => {
                self.filter_type = filter_type;
                ctx.link().send_message(Msg::LoadSuppliers);
                false
            }
            Msg::SetKeyword(keyword) => {
                self.keyword = keyword;
                ctx.link().send_message(Msg::LoadSuppliers);
                false
            }
            Msg::OpenModal(mode, supplier) => {
                self.modal_mode = mode;
                self.current_supplier = supplier;
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.current_supplier = None;
                true
            }
            Msg::CreateSupplier(req) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match SupplierService::create(req).await {
                        Ok(_) => link.send_message(Msg::LoadSuppliers),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                self.show_modal = false;
                false
            }
            Msg::UpdateSupplier(id, req) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match SupplierService::update(id, req).await {
                        Ok(_) => link.send_message(Msg::LoadSuppliers),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                self.show_modal = false;
                false
            }
            Msg::DeleteSupplier(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match SupplierService::delete(id).await {
                        Ok(_) => link.send_message(Msg::LoadSuppliers),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ToggleStatus(id, enable) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match SupplierService::toggle_status(id, enable).await {
                        Ok(_) => link.send_message(Msg::LoadSuppliers),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ChangePage(page) => {
                self.page = page;
                ctx.link().send_message(Msg::LoadSuppliers);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_status_change = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<web_sys::HtmlSelectElement>().unwrap();
            Msg::SetFilterStatus(target.value())
        });

        let on_type_change = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<web_sys::HtmlSelectElement>().unwrap();
            Msg::SetFilterType(target.value())
        });

        let on_keyword_change = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
            Msg::SetKeyword(target.value())
        });

        html! {
            <div class="supplier-page">
                <div class="page-header">
                    <h1>{"🏭 供应商管理"}</h1>
                    <div class="header-actions">
                        <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::OpenModal(ModalMode::Create, None))}>
                            {"新建供应商"}
                        </button>
                    </div>
                </div>

                <div class="filter-bar">
                    <div class="filter-item">
                        <label>{"状态："}</label>
                        <select value={self.filter_status.clone()} onchange={on_status_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="正常">{"正常"}</option>
                            <option value="禁用">{"禁用"}</option>
                        </select>
                    </div>
                    <div class="filter-item">
                        <label>{"类型："}</label>
                        <select value={self.filter_type.clone()} onchange={on_type_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="原材料">{"原材料"}</option>
                            <option value="设备">{"设备"}</option>
                            <option value="辅料">{"辅料"}</option>
                            <option value="服务">{"服务"}</option>
                        </select>
                    </div>
                    <div class="filter-item">
                        <label>{"搜索："}</label>
                        <input type="text" placeholder="供应商名称/编码"
                            value={self.keyword.clone()}
                            onchange={on_keyword_change}
                        />
                    </div>
                </div>

                {self.render_content(ctx)}
            </div>
        }
    }
}

impl SupplierPage {
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
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadSuppliers)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        if self.suppliers.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"🏭"}</div>
                    <p>{"暂无供应商数据"}</p>
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::OpenModal(ModalMode::Create, None))}>
                        {"添加第一个供应商"}
                    </button>
                </div>
            };
        }

        html! {
            <div class="table-responsive">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>{"供应商编号"}</th>
                            <th>{"供应商名称"}</th>
                            <th>{"简称"}</th>
                            <th>{"类型"}</th>
                            <th>{"联系人"}</th>
                            <th>{"联系电话"}</th>
                            <th>{"评级"}</th>
                            <th>{"状态"}</th>
                            <th>{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.suppliers.iter().map(|supplier| {
                            let supplier_clone = supplier.clone();
                            let supplier_id = supplier.id;
                            let is_enabled = supplier.is_enabled;
                            html! {
                                <tr>
                                    <td>{supplier.credit_code.chars().take(10).collect::<String>()}</td>
                                    <td>{&supplier.supplier_name}</td>
                                    <td>{&supplier.supplier_short_name}</td>
                                    <td>{&supplier.supplier_type}</td>
                                    <td>{supplier.legal_representative.clone()}</td>
                                    <td>{&supplier.contact_phone}</td>
                                    <td>
                                        {supplier.grade.as_deref().unwrap_or("-")}
                                    </td>
                                    <td>
                                        <span class={format!("status-badge status-{}", if is_enabled { "success" } else { "danger" })}>
                                            {if is_enabled { "正常" } else { "禁用" }}
                                        </span>
                                    </td>
                                    <td>
                                        <div class="action-buttons">
                                            <button class="btn-sm btn-info" onclick={ctx.link().callback(move |_| Msg::OpenModal(ModalMode::View, Some(supplier_clone.clone())))}>
                                                {"查看"}
                                            </button>
                                            <button class={format!("btn-sm {}", if is_enabled { "btn-warning" } else { "btn-success" })} onclick={ctx.link().callback(move |_| Msg::ToggleStatus(supplier_id, !is_enabled))}>
                                                {if is_enabled { "禁用" } else { "启用" }}
                                            </button>
                                            <button class="btn-sm btn-danger" onclick={ctx.link().callback(move |_| Msg::DeleteSupplier(supplier_id))}>
                                                {"删除"}
                                            </button>
                                        </div>
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