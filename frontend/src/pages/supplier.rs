// 供应商管理页面

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
        let on_status_change = ctx.link().batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlSelectElement>().ok()?;
            Some(Msg::SetFilterStatus(target.value()))
        });

        let on_type_change = ctx.link().batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlSelectElement>().ok()?;
            Some(Msg::SetFilterType(target.value()))
        });

        let on_keyword_change = ctx.link().batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlInputElement>().ok()?;
            Some(Msg::SetKeyword(target.value()))
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
                { self.render_modal(ctx) }
            </div>
        }
    }
}

impl SupplierPage {
    
    
    fn render_modal(&self, ctx: &Context<Self>) -> Html {
        if !self.show_modal {
            return html! {};
        }

        let title = match self.modal_mode {
            ModalMode::Create => "新建供应商",
            ModalMode::Edit => "编辑供应商",
            ModalMode::View => "供应商详情",
        };

        let is_readonly = self.modal_mode == ModalMode::View;

        let name = self.current_supplier.as_ref().map(|s| s.supplier_name.clone()).unwrap_or_default();
        let short_name = self.current_supplier.as_ref().map(|s| s.supplier_short_name.clone()).unwrap_or_default();
        let credit_code = self.current_supplier.as_ref().map(|s| s.credit_code.clone()).unwrap_or_default();
        let reg_address = self.current_supplier.as_ref().map(|s| s.registered_address.clone()).unwrap_or_default();
        let legal_rep = self.current_supplier.as_ref().map(|s| s.legal_representative.clone()).unwrap_or_default();
        let reg_cap = self.current_supplier.as_ref().map(|s| s.registered_capital.clone()).unwrap_or_default();
        let est_date = self.current_supplier.as_ref().map(|s| s.establishment_date.clone()).unwrap_or_default();
        let tax_type = self.current_supplier.as_ref().map(|s| s.taxpayer_type.clone()).unwrap_or_default();
        let bank_name = self.current_supplier.as_ref().map(|s| s.bank_name.clone()).unwrap_or_default();
        let bank_account = self.current_supplier.as_ref().map(|s| s.bank_account.clone()).unwrap_or_default();
        let contact_phone = self.current_supplier.as_ref().map(|s| s.contact_phone.clone()).unwrap_or_default();

        let onsubmit = ctx.link().batch_callback(move |e: SubmitEvent| {
            e.prevent_default();
            let form = e.target_unchecked_into::<web_sys::HtmlFormElement>();
            
            let name_input = form.elements().named_item("supplier_name")?.unchecked_into::<web_sys::HtmlInputElement>();
            let short_name_input = form.elements().named_item("supplier_short_name")?.unchecked_into::<web_sys::HtmlInputElement>();
            let credit_input = form.elements().named_item("credit_code")?.unchecked_into::<web_sys::HtmlInputElement>();
            let reg_addr_input = form.elements().named_item("registered_address")?.unchecked_into::<web_sys::HtmlInputElement>();
            let legal_input = form.elements().named_item("legal_representative")?.unchecked_into::<web_sys::HtmlInputElement>();
            let reg_cap_input = form.elements().named_item("registered_capital")?.unchecked_into::<web_sys::HtmlInputElement>();
            let est_date_input = form.elements().named_item("establishment_date")?.unchecked_into::<web_sys::HtmlInputElement>();
            let tax_type_input = form.elements().named_item("taxpayer_type")?.unchecked_into::<web_sys::HtmlInputElement>();
            let bank_name_input = form.elements().named_item("bank_name")?.unchecked_into::<web_sys::HtmlInputElement>();
            let bank_account_input = form.elements().named_item("bank_account")?.unchecked_into::<web_sys::HtmlInputElement>();
            let contact_phone_input = form.elements().named_item("contact_phone")?.unchecked_into::<web_sys::HtmlInputElement>();
            
            let req = CreateSupplierRequest {
                supplier_name: name_input.value(),
                supplier_short_name: short_name_input.value(),
                supplier_type: "默认".to_string(),
                credit_code: credit_input.value(),
                registered_address: reg_addr_input.value(),
                business_address: None,
                legal_representative: legal_input.value(),
                registered_capital: reg_cap_input.value(),
                establishment_date: est_date_input.value(),
                business_term: None,
                business_scope: None,
                taxpayer_type: tax_type_input.value(),
                bank_name: bank_name_input.value(),
                bank_account: bank_account_input.value(),
                contact_phone: contact_phone_input.value(),
                fax: None,
                website: None,
                email: None,
                main_business: None,
                main_market: None,
                employee_count: None,
                annual_revenue: None,
                contacts: vec![],
                qualifications: vec![],
            };
            Some(Msg::CreateSupplier(req))
        });

        html! {
            <div class="fixed inset-0 z-50 flex items-center justify-center overflow-x-hidden overflow-y-auto outline-none focus:outline-none">
                <div class="fixed inset-0 bg-gray-900 bg-opacity-50 transition-opacity" onclick={ctx.link().callback(|_| Msg::CloseModal)}></div>
                <div class="relative w-full max-w-4xl mx-auto my-6 z-50">
                    <div class="relative flex flex-col w-full bg-white border-0 rounded-xl shadow-2xl outline-none focus:outline-none p-6 max-h-[90vh] overflow-y-auto">
                        <h3 class="text-2xl font-semibold text-gray-800 mb-4">{title}</h3>
                        <form onsubmit={onsubmit}>
                            <div class="grid grid-cols-3 gap-4 mb-4">
                                <div class="col-span-1">
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"供应商名称 *"}</label>
                                    <input name="supplier_name" type="text" class="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-indigo-500" value={name} readonly={is_readonly} required=true />
                                </div>
                                <div class="col-span-1">
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"简称 *"}</label>
                                    <input name="supplier_short_name" type="text" class="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-indigo-500" value={short_name} readonly={is_readonly} required=true />
                                </div>
                                <div class="col-span-1">
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"统一信用代码 *"}</label>
                                    <input name="credit_code" type="text" class="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-indigo-500" value={credit_code} readonly={is_readonly} required=true />
                                </div>
                                
                                <div class="col-span-2">
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"注册地址 *"}</label>
                                    <input name="registered_address" type="text" class="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-indigo-500" value={reg_address} readonly={is_readonly} required=true />
                                </div>
                                <div class="col-span-1">
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"法人代表 *"}</label>
                                    <input name="legal_representative" type="text" class="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-indigo-500" value={legal_rep} readonly={is_readonly} required=true />
                                </div>
                                
                                <div class="col-span-1">
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"注册资本 *"}</label>
                                    <input name="registered_capital" type="text" class="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-indigo-500" value={reg_cap} readonly={is_readonly} required=true />
                                </div>
                                <div class="col-span-1">
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"成立日期 *"}</label>
                                    <input name="establishment_date" type="text" placeholder="2026-01-01" class="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-indigo-500" value={est_date} readonly={is_readonly} required=true />
                                </div>
                                <div class="col-span-1">
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"纳税人类型 *"}</label>
                                    <input name="taxpayer_type" type="text" class="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-indigo-500" value={tax_type} readonly={is_readonly} required=true />
                                </div>

                                <div class="col-span-1">
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"开户行 *"}</label>
                                    <input name="bank_name" type="text" class="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-indigo-500" value={bank_name} readonly={is_readonly} required=true />
                                </div>
                                <div class="col-span-1">
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"银行账号 *"}</label>
                                    <input name="bank_account" type="text" class="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-indigo-500" value={bank_account} readonly={is_readonly} required=true />
                                </div>
                                <div class="col-span-1">
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"公司联系电话 *"}</label>
                                    <input name="contact_phone" type="text" class="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-indigo-500" value={contact_phone} readonly={is_readonly} required=true />
                                </div>
                            </div>
                            <div class="flex justify-end gap-2 mt-6 pt-4 border-t">
                                <button type="button" class="px-4 py-2 text-gray-500 hover:bg-gray-100 rounded" onclick={ctx.link().callback(|_| Msg::CloseModal)}>{"取消"}</button>
                                {if !is_readonly {
                                    html! {
                                        <button type="submit" class="px-4 py-2 bg-indigo-600 text-white rounded hover:bg-indigo-700">{"保存"}</button>
                                    }
                                } else {
                                    html! {}
                                }}
                            </div>
                        </form>
                    </div>
                </div>
            </div>
        }
    }

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