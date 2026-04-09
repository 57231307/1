//! 客户管理页面

use crate::components::main_layout::MainLayout;
use crate::models::customer::{
    CreateCustomerRequest, Customer, CustomerListResponse, CustomerQuery, UpdateCustomerRequest,
};
use crate::services::customer_service::CustomerService;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use web_sys::window;

pub struct CustomerPage {
    customers: Vec<Customer>,
    loading: bool,
    error: Option<String>,
    show_modal: bool,
    modal_mode: ModalMode,
    current_customer: Option<Customer>,
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
    LoadCustomers,
    CustomersLoaded(CustomerListResponse),
    LoadError(String),
    SetFilterStatus(String),
    SetFilterType(String),
    SetKeyword(String),
    OpenModal(ModalMode, Option<Customer>),
    CloseModal,
    CreateCustomer(CreateCustomerRequest),
    UpdateCustomer(i32, UpdateCustomerRequest),
    DeleteCustomer(i32),
    ChangePage(u64),
}

impl Component for CustomerPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            customers: Vec::new(),
            loading: true,
            error: None,
            show_modal: false,
            modal_mode: ModalMode::View,
            current_customer: None,
            filter_status: String::from("全部"),
            filter_type: String::from("全部"),
            keyword: String::new(),
            page: 1,
            page_size: 20,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadCustomers);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadCustomers => {
                self.loading = true;
                let query = CustomerQuery {
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                    status: if self.filter_status == "全部" {
                        None
                    } else {
                        Some(self.filter_status.clone())
                    },
                    customer_type: if self.filter_type == "全部" {
                        None
                    } else {
                        Some(self.filter_type.clone())
                    },
                    keyword: if self.keyword.is_empty() {
                        None
                    } else {
                        Some(self.keyword.clone())
                    },
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match CustomerService::list(query).await {
                        Ok(customers) => link.send_message(Msg::CustomersLoaded(customers)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::CustomersLoaded(customer_list) => {
                self.customers = if !customer_list.data.is_empty() {
                    customer_list.data
                } else {
                    customer_list.items
                };
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
                ctx.link().send_message(Msg::LoadCustomers);
                false
            }
            Msg::SetFilterType(filter_type) => {
                self.filter_type = filter_type;
                ctx.link().send_message(Msg::LoadCustomers);
                false
            }
            Msg::SetKeyword(keyword) => {
                self.keyword = keyword;
                ctx.link().send_message(Msg::LoadCustomers);
                false
            }
            Msg::OpenModal(mode, customer) => {
                self.modal_mode = mode;
                self.current_customer = customer;
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.current_customer = None;
                true
            }
            Msg::CreateCustomer(req) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match CustomerService::create(req).await {
                        Ok(_) => link.send_message(Msg::LoadCustomers),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                self.show_modal = false;
                false
            }
            Msg::UpdateCustomer(id, req) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match CustomerService::update(id, req).await {
                        Ok(_) => link.send_message(Msg::LoadCustomers),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                self.show_modal = false;
                false
            }
            Msg::DeleteCustomer(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match CustomerService::delete(id).await {
                        Ok(_) => link.send_message(Msg::LoadCustomers),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ChangePage(page) => {
                self.page = page;
                ctx.link().send_message(Msg::LoadCustomers);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_status_change = ctx.link().callback(|e: Event| {
            let target = e
                .target()
                .unwrap()
                .dyn_into::<web_sys::HtmlSelectElement>()
                .unwrap();
            Msg::SetFilterStatus(target.value())
        });

        let on_type_change = ctx.link().callback(|e: Event| {
            let target = e
                .target()
                .unwrap()
                .dyn_into::<web_sys::HtmlSelectElement>()
                .unwrap();
            Msg::SetFilterType(target.value())
        });

        let on_keyword_change = ctx.link().callback(|e: Event| {
            let target = e
                .target()
                .unwrap()
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap();
            Msg::SetKeyword(target.value())
        });

        html! {
            <MainLayout current_page={"客户管理"}>
<div class="customer-page">
                <div class="page-header">
                    <h1>{"👥 客户管理"}</h1>
                    <div class="header-actions">
                        <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::OpenModal(ModalMode::Create, None))}>
                            {"新建客户"}
                        </button>
                    </div>
                </div>

                <div class="filter-bar">
                    <div class="filter-item">
                        <label>{"客户状态："}</label>
                        <select value={self.filter_status.clone()} onchange={on_status_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="正常">{"正常"}</option>
                            <option value="禁用">{"禁用"}</option>
                        </select>
                    </div>
                    <div class="filter-item">
                        <label>{"客户类型："}</label>
                        <select value={self.filter_type.clone()} onchange={on_type_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="批发">{"批发"}</option>
                            <option value="零售">{"零售"}</option>
                            <option value="代理">{"代理"}</option>
                        </select>
                    </div>
                    <div class="filter-item">
                        <label>{"搜索："}</label>
                        <input type="text" placeholder="客户名称/编号"
                            value={self.keyword.clone()}
                            onchange={on_keyword_change}
                        />
                    </div>
                </div>

                {self.render_content(ctx)}
                {self.render_modal(ctx)}
            </div>
        
</MainLayout>}
    }
}

impl CustomerPage {
    fn render_modal(&self, ctx: &Context<Self>) -> Html {
        if !self.show_modal {
            return html! {};
        }

        let title = match self.modal_mode {
            ModalMode::Create => "新建客户",
            ModalMode::Edit => "编辑客户",
            ModalMode::View => "客户详情",
        };

        let is_readonly = self.modal_mode == ModalMode::View;

        let code = self
            .current_customer
            .as_ref()
            .map(|c| c.customer_code.clone())
            .unwrap_or_default();
        let name = self
            .current_customer
            .as_ref()
            .map(|c| c.customer_name.clone())
            .unwrap_or_default();
        let contact = self
            .current_customer
            .as_ref()
            .and_then(|c| c.contact_person.clone())
            .unwrap_or_default();
        let phone = self
            .current_customer
            .as_ref()
            .and_then(|c| c.contact_phone.clone())
            .unwrap_or_default();

        let onsubmit = ctx.link().callback(move |e: SubmitEvent| {
            e.prevent_default();
            let form = e.target_unchecked_into::<web_sys::HtmlFormElement>();

            let code_input = form
                .elements()
                .named_item("customer_code")
                .unwrap()
                .unchecked_into::<web_sys::HtmlInputElement>();
            let name_input = form
                .elements()
                .named_item("customer_name")
                .unwrap()
                .unchecked_into::<web_sys::HtmlInputElement>();
            let contact_input = form
                .elements()
                .named_item("contact_person")
                .unwrap()
                .unchecked_into::<web_sys::HtmlInputElement>();
            let phone_input = form
                .elements()
                .named_item("contact_phone")
                .unwrap()
                .unchecked_into::<web_sys::HtmlInputElement>();

            let req = CreateCustomerRequest {
                customer_code: code_input.value(),
                customer_name: name_input.value(),
                contact_person: if contact_input.value().is_empty() {
                    None
                } else {
                    Some(contact_input.value())
                },
                contact_phone: if phone_input.value().is_empty() {
                    None
                } else {
                    Some(phone_input.value())
                },
                contact_email: None,
                address: None,
                city: None,
                province: None,
                postal_code: None,
                credit_limit: None,
                payment_terms: None,
                tax_id: None,
                bank_name: None,
                bank_account: None,
                customer_type: Some("普通".to_string()),
                notes: None,
                customer_industry: None,
                main_products: None,
                annual_purchase: None,
                quality_requirement: None,
                inspection_standard: None,
            };
            Msg::CreateCustomer(req)
        });

        html! {
            <div class="fixed inset-0 z-50 flex items-center justify-center overflow-x-hidden overflow-y-auto outline-none focus:outline-none">
                <div class="fixed inset-0 bg-gray-900 bg-opacity-50 transition-opacity" onclick={ctx.link().callback(|_| Msg::CloseModal)}></div>
                <div class="relative w-full max-w-2xl mx-auto my-6 z-50">
                    <div class="relative flex flex-col w-full bg-white border-0 rounded-xl shadow-2xl outline-none focus:outline-none">
                        <div class="flex items-start justify-between p-5 border-b border-solid border-gray-200 rounded-t">
                            <h3 class="text-2xl font-semibold text-gray-800">{title}</h3>
                            <button class="p-1 ml-auto bg-transparent border-0 text-gray-500 float-right text-3xl leading-none font-semibold outline-none focus:outline-none hover:text-gray-800" onclick={ctx.link().callback(|_| Msg::CloseModal)}>
                                <span class="block w-6 h-6 text-2xl outline-none focus:outline-none">{"×"}</span>
                            </button>
                        </div>
                        <form onsubmit={onsubmit}>
                            <div class="relative p-6 flex-auto grid grid-cols-2 gap-4">
                                <div class="col-span-1">
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"客户编码 *"}</label>
                                    <input name="customer_code" type="text" class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-indigo-500" value={code} readonly={is_readonly} required=true />
                                </div>
                                <div class="col-span-1">
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"客户名称 *"}</label>
                                    <input name="customer_name" type="text" class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-indigo-500" value={name} readonly={is_readonly} required=true />
                                </div>
                                <div class="col-span-1">
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"主营业务(如印染/织造)"}</label>
                                    <input name="main_products" type="text" class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-indigo-500" value={self.current_customer.as_ref().and_then(|c| c.main_products.clone()).unwrap_or_default()} readonly={is_readonly} />
                                </div>
                                <div class="col-span-1">
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"信用评级(A/B/C)"}</label>
                                    <select name="credit_rating" class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-indigo-500" disabled={is_readonly}>
                                        <option value="A">{"A"}</option>
                                        <option value="B">{"B"}</option>
                                        <option value="C">{"C"}</option>
                                    </select>
                                </div>
                                <div class="col-span-1">
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"联系人"}</label>
                                    <input name="contact_person" type="text" class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-indigo-500" value={contact} readonly={is_readonly} />
                                </div>
                                <div class="col-span-1">
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"联系电话"}</label>
                                    <input name="contact_phone" type="text" class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-indigo-500" value={phone} readonly={is_readonly} />
                                </div>
                            </div>
                            <div class="flex items-center justify-end p-6 border-t border-solid border-gray-200 rounded-b">
                                <button type="button" class="text-gray-500 bg-transparent font-bold uppercase px-6 py-2 text-sm outline-none focus:outline-none mr-1 mb-1 ease-linear transition-all duration-150 hover:bg-gray-100 rounded" onclick={ctx.link().callback(|_| Msg::CloseModal)}>
                                    {"取消"}
                                </button>
                                {if !is_readonly {
                                    html! {
                                        <button type="submit" class="bg-indigo-600 text-white active:bg-indigo-700 font-bold uppercase text-sm px-6 py-3 rounded shadow hover:shadow-lg outline-none focus:outline-none mr-1 mb-1 ease-linear transition-all duration-150">
                                            {"保存"}
                                        </button>
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
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadCustomers)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        if self.customers.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"👥"}</div>
                    <p>{"暂无客户数据"}</p>
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::OpenModal(ModalMode::Create, None))}>
                        {"添加第一个客户"}
                    </button>
                </div>
            };
        }

        html! {
            <div class="table-responsive">
                <div class="overflow-x-auto w-full pb-4">
<table class="data-table w-full">
                    <thead>
                        <tr>
                            <th>{"客户编号"}</th>
                            <th>{"客户名称"}</th>
                            <th>{"主营业务"}</th>
                            <th>{"信用评级"}</th>
                            <th>{"联系人"}</th>
                            <th>{"联系电话"}</th>
                            <th>{"客户类型"}</th>
                            <th class="numeric-cell text-right">{"信用额度"}</th>
                            <th>{"状态"}</th>
                            <th>{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.customers.iter().map(|customer| {
                            let customer_clone = customer.clone();
                            let customer_id = customer.id;
                            html! {
                                <tr>
                                    <td>{&customer.customer_code}</td>
                                    <td>{&customer.customer_name}</td>
                                    <td>{customer.main_products.as_deref().unwrap_or("-")}</td>
                                    <td>{"A"}</td>
                                    <td>{customer.contact_person.as_deref().unwrap_or("-")}</td>
                                    <td>{customer.contact_phone.as_deref().unwrap_or("-")}</td>
                                    <td>{customer.customer_type.as_deref().unwrap_or("-")}</td>
                                    <td class="numeric-cell text-right">{customer.credit_limit.as_deref().unwrap_or("-")}</td>
                                    <td>
                                        <span class={format!("status-badge status-{}", self.get_status_class(&customer.status))}>
                                            {&customer.status}
                                        </span>
                                    </td>
                                    <td>
                                        <div class="action-buttons">
                                            <button class="btn-sm btn-info" onclick={ctx.link().callback(move |_| Msg::OpenModal(ModalMode::View, Some(customer_clone.clone())))}>
                                                {"查看"}
                                            </button>
                                            <button class="btn-sm btn-danger" onclick={ctx.link().callback(move |_| Msg::DeleteCustomer(customer_id))}>
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
            </div>
        }
    }

    fn get_status_class(&self, status: &str) -> &str {
        match status {
            "正常" => "success",
            "禁用" => "danger",
            _ => "default",
        }
    }
}
