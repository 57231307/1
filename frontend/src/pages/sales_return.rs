//! 销售退货管理页面

use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use crate::models::sales_return::{CreateSalesReturnRequest, CreateSalesReturnItemRequest, SalesReturn, SalesReturnQuery};
use crate::services::sales_return_service::SalesReturnService;

/// 销售退货页面状态管理
pub struct SalesReturnPage {
    printing_return: Option<crate::models::sales_return::SalesReturn>,
    print_trigger: bool,
    show_modal: bool,
    new_return_no: String,
    new_customer_id: String,
    new_product_id: String,
    new_quantity: String,
    new_reason: String,
    returns: Vec<SalesReturn>,
    loading: bool,
    error: Option<String>,
    filter_status: String,
    page: u64,
    page_size: u64,
}

/// 消息枚举
pub enum Msg {
    LoadReturns,
    ReturnsLoaded(Vec<SalesReturn>),
    LoadError(String),
    SetFilterStatus(String),
    ViewReturn(i32),
    SubmitReturn(i32),
    ApproveReturn(i32),
    ChangePage(u64),
}

impl Component for SalesReturnPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            returns: Vec::new(),
            loading: true,
            printing_return: None,
            print_trigger: false,
            show_modal: false,
            new_return_no: String::new(),
            new_customer_id: String::new(),
            new_product_id: String::new(),
            new_quantity: String::new(),
            new_reason: String::new(),
            error: None,
            filter_status: String::from("全部"),
            page: 1,
            page_size: 20,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadReturns);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadReturns => {
                self.loading = true;
                let query = SalesReturnQuery {
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                    status: if self.filter_status == "全部" { None } else { Some(self.filter_status.clone()) },
                    customer_id: None,
                    return_no: None,
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match SalesReturnService::list(query).await {
                        Ok(returns) => link.send_message(Msg::ReturnsLoaded(returns.data)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ReturnsLoaded(returns) => {
                self.returns = returns;
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
                ctx.link().send_message(Msg::LoadReturns);
                false
            }
            Msg::ViewReturn(id) => {
                let _ = web_sys::window().unwrap().location().set_href(&format!("/sales-returns/{}", id));
                false
            }
            Msg::SubmitReturn(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    if let Err(e) = SalesReturnService::submit(id).await {
                        link.send_message(Msg::LoadError(e));
                    } else {
                        link.send_message(Msg::LoadReturns);
                    }
                });
                false
            }
            Msg::ApproveReturn(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    if let Err(e) = SalesReturnService::approve(id).await {
                        link.send_message(Msg::LoadError(e));
                    } else {
                        link.send_message(Msg::LoadReturns);
                    }
                });
                false
            }
            Msg::ChangePage(page) => {
                self.page = page;
                ctx.link().send_message(Msg::LoadReturns);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="page-container">
                <div class="page-header">
                    <h2>{ "销售退货管理" }</h2>
                    <div class="header-actions">
                        <button class="btn btn-primary" onclick={Callback::from(|_| {
                            let _ = web_sys::window().unwrap().location().set_href("/sales-returns/new");
                        })}>
                            <i class="fas fa-plus"></i> { "新建退货单" }
                        </button>
                    </div>
                </div>

                { self.render_filters(ctx) }
                
                if let Some(ref err) = self.error {
                    <div class="alert alert-danger">{ err }</div>
                }

                if self.loading {
                    <div class="loading-state">
                        <i class="fas fa-spinner fa-spin"></i> { "加载中..." }
                    </div>
                } else {
                    <div class="table-responsive">
                        <table class="table">
                            <thead>
                                <tr>
                                    <th>{ "退货单号" }</th>
                                    <th>{ "客户ID" }</th>
                                    <th>{ "退货日期" }</th>
                                    <th>{ "状态" }</th>
                                    <th>{ "退货金额" }</th>
                                    <th>{ "退货原因" }</th>
                                    <th>{ "创建时间" }</th>
                                    <th>{ "操作" }</th>
                                </tr>
                            </thead>
                            <tbody>
                                { for self.returns.iter().map(|r| self.render_return_row(r, link)) }
                            </tbody>
                        </table>
                    </div>
                }
            </div>
        }
    }
}

impl SalesReturnPage {
    fn render_filters(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let statuses = vec!["全部", "DRAFT", "SUBMITTED", "APPROVED", "REJECTED", "COMPLETED"];
        
        html! {
            <div class="filters-section">
                <div class="filter-group">
                    <label>{ "状态：" }</label>
                    <select 
                        class="form-control"
                        value={self.filter_status.clone()}
                        onchange={link.callback(|e: Event| {
                            use wasm_bindgen::JsCast;
                            let select = e.target().unwrap().unchecked_into::<web_sys::HtmlSelectElement>();
                            Msg::SetFilterStatus(select.value())
                        })}
                    >
                        { for statuses.iter().map(|s| html! { <option value={*s}>{ s }</option> }) }
                    </select>
                </div>
            </div>
        }
    }

    fn render_return_row(&self, return_order: &SalesReturn, link: &html::Scope<Self>) -> Html {
        let id = return_order.id;
        
        html! {
            <tr>
                <td>{ &return_order.return_no }</td>
                <td>{ return_order.customer_id }</td>
                <td>{ &return_order.return_date }</td>
                <td>
                    <span class={format!("status-badge status-{}", return_order.status.to_lowercase())}>
                        { &return_order.status }
                    </span>
                </td>
                <td>{ format!("¥{:.2}", return_order.total_amount) }</td>
                <td>{ &return_order.reason }</td>
                <td>{ return_order.created_at.split('T').next().unwrap_or("") }</td>
                <td class="actions">
                    <button class="btn btn-sm btn-info" onclick={link.callback(move |_| Msg::ViewReturn(id))}>
                        <i class="fas fa-eye"></i> { "查看" }
                    </button>
                    if return_order.status == "DRAFT" {
                        <button class="btn btn-sm btn-primary" onclick={link.callback(move |_| Msg::SubmitReturn(id))}>
                            <i class="fas fa-paper-plane"></i> { "提交" }
                        </button>
                    }
                    if return_order.status == "SUBMITTED" {
                        <button class="btn btn-sm btn-success" onclick={link.callback(move |_| Msg::ApproveReturn(id))}>
                            <i class="fas fa-check"></i> { "审批" }
                        </button>
                    }
                </td>
            </tr>
        }
    }
}
