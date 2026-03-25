//! 采购退货管理页面

use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::services::purchase_return_service::{
    PurchaseReturnService, PurchaseReturn, PurchaseReturnQuery,
};

/// 采购退货页面状态管理
pub struct PurchaseReturnPage {
    returns: Vec<PurchaseReturn>,
    loading: bool,
    error: Option<String>,
    filter_status: String,
    page: u64,
    page_size: u64,
}

/// 消息枚举
pub enum Msg {
    LoadReturns,
    ReturnsLoaded(Vec<PurchaseReturn>),
    LoadError(String),
    SetFilterStatus(String),
    ViewReturn(i32),
    DeleteReturn(i32),
    SubmitReturn(i32),
    ApproveReturn(i32),
    RejectReturn(i32),
    ChangePage(u64),
}

impl Component for PurchaseReturnPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            returns: Vec::new(),
            loading: true,
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
                let query = PurchaseReturnQuery {
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                    status: if self.filter_status == "全部" { None } else { Some(self.filter_status.clone()) },
                    supplier_id: None,
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseReturnService::list(query).await {
                        Ok(returns) => link.send_message(Msg::ReturnsLoaded(returns)),
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
                web_sys::window().unwrap().location().set_href(&format!("/purchase-returns/{}", id)).ok();
                false
            }
            Msg::DeleteReturn(_id) => {
                // 删除功能暂未实现
                false
            }
            Msg::SubmitReturn(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseReturnService::submit(id).await {
                        Ok(_) => link.send_message(Msg::LoadReturns),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ApproveReturn(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseReturnService::approve(id).await {
                        Ok(_) => link.send_message(Msg::LoadReturns),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::RejectReturn(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseReturnService::reject(id, "不符合要求".to_string()).await {
                        Ok(_) => link.send_message(Msg::LoadReturns),
                        Err(e) => link.send_message(Msg::LoadError(e)),
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
        let on_status_change = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<web_sys::HtmlSelectElement>().unwrap();
            Msg::SetFilterStatus(target.value())
        });

        html! {
            <div class="purchase-return-page">
                <div class="page-header">
                    <h1>{"退货管理"}</h1>
                </div>

                <div class="filter-bar">
                    <div class="filter-item">
                        <label>{"退货状态："}</label>
                        <select value={self.filter_status.clone()} onchange={on_status_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="草稿">{"草稿"}</option>
                            <option value="待审批">{"待审批"}</option>
                            <option value="已审批">{"已审批"}</option>
                            <option value="已拒绝">{"已拒绝"}</option>
                        </select>
                    </div>
                </div>

                {self.render_content(ctx)}
            </div>
        }
    }
}

impl PurchaseReturnPage {
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
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadReturns)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        if self.returns.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"📦"}</div>
                    <p>{"暂无采购退货单"}</p>
                </div>
            };
        }

        html! {
            <div class="table-responsive">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>{"退货单号"}</th>
                            <th>{"关联订单"}</th>
                            <th>{"供应商"}</th>
                            <th>{"退货日期"}</th>
                            <th>{"退货状态"}</th>
                            <th>{"退货数量"}</th>
                            <th>{"退货金额"}</th>
                            <th>{"仓库"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.returns.iter().map(|ret| {
                            let status = ret.status.clone();
                            html! {
                                <tr>
                                    <td>{&ret.return_no}</td>
                                    <td>{ret.order_no.as_deref().unwrap_or("-")}</td>
                                    <td>{ret.supplier_name.as_deref().unwrap_or("-")}</td>
                                    <td>{&ret.return_date}</td>
                                    <td>{status}</td>
                                    <td class="numeric">{&ret.total_quantity}</td>
                                    <td class="numeric">{&ret.total_amount}</td>
                                    <td>{ret.warehouse_name.as_deref().unwrap_or("-")}</td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
            </div>
        }
    }
}