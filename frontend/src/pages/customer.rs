//! 客户管理页面

use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::services::customer_service::CustomerService;
use crate::models::customer::{
    Customer, CustomerQuery, CustomerListResponse,
    CreateCustomerRequest, UpdateCustomerRequest,
};

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
                    status: if self.filter_status == "全部" { None } else { Some(self.filter_status.clone()) },
                    customer_type: if self.filter_type == "全部" { None } else { Some(self.filter_type.clone()) },
                    keyword: if self.keyword.is_empty() { None } else { Some(self.keyword.clone()) },
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
            </div>
        }
    }
}

impl CustomerPage {
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
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>{"客户编号"}</th>
                            <th>{"客户名称"}</th>
                            <th>{"联系人"}</th>
                            <th>{"联系电话"}</th>
                            <th>{"客户类型"}</th>
                            <th>{"信用额度"}</th>
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
                                    <td>{customer.contact_person.as_deref().unwrap_or("-")}</td>
                                    <td>{customer.contact_phone.as_deref().unwrap_or("-")}</td>
                                    <td>{customer.customer_type.as_deref().unwrap_or("-")}</td>
                                    <td class="numeric">{customer.credit_limit.as_deref().unwrap_or("-")}</td>
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