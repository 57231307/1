//! 客户信用管理页面

use crate::components::main_layout::MainLayout;
use crate::models::customer_credit::{
    CreditLimitAdjustmentRequest, CreditQueryParams, CreditRatingRequest, CustomerCredit,
};
use crate::services::customer_credit_service::CustomerCreditService;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use web_sys::window;

pub struct CustomerCreditPage {
    credits: Vec<CustomerCredit>,
    loading: bool,
    error: Option<String>,
    filter_customer_id: Option<i32>,
    filter_credit_level: String,
    filter_status: String,
    page: i64,
    page_size: i64,
    show_rating_modal: bool,
    show_adjust_modal: bool,
    selected_credit: Option<CustomerCredit>,
    rating_level: String,
    rating_score: String,
    rating_limit: String,
    rating_days: String,
    rating_remark: String,
    adjust_type: String,
    adjust_amount: String,
    adjust_reason: String,
}

pub enum Msg {
    LoadCredits,
    CreditsLoaded(Vec<CustomerCredit>),
    LoadError(String),
    SetFilterCustomerId(Option<i32>),
    SetFilterCreditLevel(String),
    SetFilterStatus(String),
    ChangePage(i64),
    ToggleRatingModal,
    ToggleAdjustModal,
    SelectCredit(CustomerCredit),
    SetRatingLevel(String),
    SetRatingScore(String),
    SetRatingLimit(String),
    SetRatingDays(String),
    SetRatingRemark(String),
    SetAdjustType(String),
    SetAdjustAmount(String),
    SetAdjustReason(String),
    SubmitRating,
    SubmitAdjustment,
    OccupyCredit(i32, f64),
    ReleaseCredit(i32, f64),
}

impl Component for CustomerCreditPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            credits: Vec::new(),
            loading: false,
            error: None,
            filter_customer_id: None,
            filter_credit_level: String::new(),
            filter_status: String::new(),
            page: 1,
            page_size: 20,
            show_rating_modal: false,
            show_adjust_modal: false,
            selected_credit: None,
            rating_level: String::new(),
            rating_score: String::new(),
            rating_limit: String::new(),
            rating_days: String::new(),
            rating_remark: String::new(),
            adjust_type: String::from("increase"),
            adjust_amount: String::new(),
            adjust_reason: String::new(),
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadCredits);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadCredits => {
                self.loading = true;
                self.error = None;
                let params = CreditQueryParams {
                    customer_id: self.filter_customer_id,
                    credit_level: if self.filter_credit_level.is_empty() {
                        None
                    } else {
                        Some(self.filter_credit_level.clone())
                    },
                    status: if self.filter_status.is_empty() {
                        None
                    } else {
                        Some(self.filter_status.clone())
                    },
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match CustomerCreditService::list_credits(params).await {
                        Ok(response) => link.send_message(Msg::CreditsLoaded(response.items)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::CreditsLoaded(credits) => {
                self.credits = credits;
                self.loading = false;
                true
            }
            Msg::LoadError(e) => {
                self.error = Some(e);
                self.loading = false;
                true
            }
            Msg::SetFilterCustomerId(id) => {
                self.filter_customer_id = id;
                true
            }
            Msg::SetFilterCreditLevel(level) => {
                self.filter_credit_level = level;
                true
            }
            Msg::SetFilterStatus(status) => {
                self.filter_status = status;
                true
            }
            Msg::ChangePage(page) => {
                self.page = page;
                ctx.link().send_message(Msg::LoadCredits);
                false
            }
            Msg::ToggleRatingModal => {
                self.show_rating_modal = !self.show_rating_modal;
                true
            }
            Msg::ToggleAdjustModal => {
                self.show_adjust_modal = !self.show_adjust_modal;
                true
            }
            Msg::SelectCredit(credit) => {
                self.selected_credit = Some(credit);
                true
            }
            Msg::SetRatingLevel(level) => {
                self.rating_level = level;
                true
            }
            Msg::SetRatingScore(score) => {
                self.rating_score = score;
                true
            }
            Msg::SetRatingLimit(limit) => {
                self.rating_limit = limit;
                true
            }
            Msg::SetRatingDays(days) => {
                self.rating_days = days;
                true
            }
            Msg::SetRatingRemark(remark) => {
                self.rating_remark = remark;
                true
            }
            Msg::SetAdjustType(adjust_type) => {
                self.adjust_type = adjust_type;
                true
            }
            Msg::SetAdjustAmount(amount) => {
                self.adjust_amount = amount;
                true
            }
            Msg::SetAdjustReason(reason) => {
                self.adjust_reason = reason;
                true
            }
            Msg::SubmitRating => {
                if let Some(credit) = &self.selected_credit {
                    let customer_id = credit.customer_id;
                    let level = self.rating_level.clone();
                    let score: i32 = self.rating_score.parse().unwrap_or(0);
                    let limit: f64 = self.rating_limit.parse().unwrap_or(0.0);
                    let days: i32 = self.rating_days.parse().unwrap_or(0);
                    let remark = if self.rating_remark.is_empty() {
                        None
                    } else {
                        Some(self.rating_remark.clone())
                    };

                    let req = CreditRatingRequest {
                        customer_id,
                        credit_level: level,
                        credit_score: score,
                        credit_limit: limit.to_string(),
                        credit_days: days,
                        remark,
                    };

                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match CustomerCreditService::set_credit_rating(req).await {
                            Ok(_) => {
                                link.send_message(Msg::ToggleRatingModal);
                                link.send_message(Msg::LoadCredits);
                            }
                            Err(e) => link.send_message(Msg::LoadError(e)),
                        }
                    });
                }
                false
            }
            Msg::SubmitAdjustment => {
                if let Some(credit) = &self.selected_credit {
                    let customer_id = credit.customer_id;
                    let adj_type = self.adjust_type.clone();
                    let amount: f64 = self.adjust_amount.parse().unwrap_or(0.0);
                    let reason = self.adjust_reason.clone();

                    let req = CreditLimitAdjustmentRequest {
                        adjustment_type: adj_type,
                        amount: amount.to_string(),
                        reason,
                    };

                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match CustomerCreditService::adjust_credit_limit(customer_id, req).await {
                            Ok(_) => {
                                link.send_message(Msg::ToggleAdjustModal);
                                link.send_message(Msg::LoadCredits);
                            }
                            Err(e) => link.send_message(Msg::LoadError(e)),
                        }
                    });
                }
                false
            }
            Msg::OccupyCredit(customer_id, amount) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match CustomerCreditService::occupy_credit(customer_id, amount.to_string())
                        .await
                    {
                        Ok(_) => link.send_message(Msg::LoadCredits),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ReleaseCredit(customer_id, amount) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match CustomerCreditService::release_credit(customer_id, amount.to_string())
                        .await
                    {
                        Ok(_) => link.send_message(Msg::LoadCredits),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <MainLayout current_page={"客户信用管理"}>
<div class="customer-credit-page">
                <div class="page-header">
                    <h1>{"客户信用管理"}</h1>
                </div>

                <div class="filter-bar">
                    <div class="filter-group">
                        <label>{"信用等级："}</label>
                        <select onchange={link.callback(|e: Event| {
                            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
                            Msg::SetFilterCreditLevel(select.value())
                        })}>
                            <option value="">{"全部"}</option>
                            <option value="A">{"A级"}</option>
                            <option value="B">{"B级"}</option>
                            <option value="C">{"C级"}</option>
                            <option value="D">{"D级"}</option>
                        </select>
                    </div>

                    <div class="filter-group">
                        <label>{"状态："}</label>
                        <select onchange={link.callback(|e: Event| {
                            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
                            Msg::SetFilterStatus(select.value())
                        })}>
                            <option value="">{"全部"}</option>
                            <option value="active">{"启用"}</option>
                            <option value="inactive">{"停用"}</option>
                        </select>
                    </div>

                    <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadCredits)}>
                        {"查询"}
                    </button>
                </div>

                if self.loading {
                    <div class="loading">{"加载中..."}</div>
                }

                if let Some(e) = &self.error {
                    <div class="error">{e}</div>
                }

                <div class="table-container">
                    <div class="table-responsive overflow-x-auto w-full pb-4 shadow-sm sm:rounded-lg">
<table class="data-table w-full">
                        <thead>
                            <tr>
                                <th class="numeric-cell text-right">{"客户ID"}</th>
                                <th>{"色卡编号"}</th>
                                <th>{"花型"}</th>
                                <th>{"信用等级"}</th>
                                <th class="numeric-cell text-right">{"信用分数"}</th>
                                <th class="numeric-cell text-right">{"信用额度"}</th>
                                <th class="numeric-cell text-right">{"已用额度"}</th>
                                <th class="numeric-cell text-right">{"可用额度"}</th>
                                <th class="numeric-cell text-right">{"信用天数"}</th>
                                <th>{"关联源单据"}</th>
                                                <th>{"状态"}</th>
                                <th>{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {for self.credits.iter().map(|credit| {
                                let credit_clone = credit.clone();
                                let credit_clone2 = credit.clone();
                                html! {
                                    <tr>
                                        <td class="numeric-cell text-right">{credit.customer_id}</td>
                                        <td>{"-"}</td>
                                        <td>{"🎨"}</td>
                                        <td>{credit.credit_level.as_ref().unwrap_or(&"-".to_string())}</td>
                                        <td class="numeric-cell text-right">{credit.credit_score.unwrap_or(0)}</td>
                                        <td class="numeric-cell text-right">{credit.credit_limit.clone().map(|v| format!("{:.2}", v)).unwrap_or("-".to_string())}</td>
                                        <td class="numeric-cell text-right">{credit.used_credit.clone().map(|v| format!("{:.2}", v)).unwrap_or("-".to_string())}</td>
                                        <td class="numeric-cell text-right">{credit.available_credit.clone().map(|v| format!("{:.2}", v)).unwrap_or("-".to_string())}</td>
                                        <td class="numeric-cell text-right">{credit.credit_days.unwrap_or(0)}</td>
                                        <td>
                                            <span class="status-badge">{credit.status.as_ref().unwrap_or(&"-".to_string())}</span>
                                        </td>
                                        <td class="actions">
                                            <button class="btn btn-sm" onclick={link.callback(move |_| Msg::SelectCredit(credit_clone.clone()))}>
                                                {"评级"}
                                            </button>
                                            <button class="btn btn-sm" onclick={link.callback(move |_| Msg::SelectCredit(credit_clone2.clone()))}>
                                                {"调整"}
                                            </button>
                                        </td>
                                    </tr>
                                }
                            })}
                        </tbody>
                    </table>
</div>
                </div>

                {self.view_pagination(ctx)}

                if self.show_rating_modal {
                    {self.view_rating_modal(ctx)}
                }

                if self.show_adjust_modal {
                    {self.view_adjust_modal(ctx)}
                }
            </div>
        
</MainLayout>}
    }
}

impl CustomerCreditPage {
    fn view_pagination(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let current_page = self.page;

        html! {
            <div class="pagination">
                <button class="btn" disabled={current_page <= 1} onclick={link.callback(|_| Msg::ChangePage(1))}>
                    {"首页"}
                </button>
                <button class="btn" disabled={current_page <= 1} onclick={link.callback(move |_| Msg::ChangePage(current_page - 1))}>
                    {"上一页"}
                </button>
                <span>{format!("第 {} 页", current_page)}</span>
                <button class="btn" onclick={link.callback(move |_| Msg::ChangePage(current_page + 1))}>
                    {"下一页"}
                </button>
            </div>
        }
    }
}

impl CustomerCreditPage {
    fn view_rating_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="modal-overlay">
                <div class="modal">
                    <div class="modal-header">
                        <h3>{"设置信用评级"}</h3>
                        <button class="btn-close" onclick={link.callback(|_| Msg::ToggleRatingModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        <div class="form-group">
                            <label>{"信用等级："}</label>
                            <select onchange={link.callback(|e: Event| {
                                let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
                                Msg::SetRatingLevel(select.value())
                            })}>
                                <option value="A">{"A级"}</option>
                                <option value="B">{"B级"}</option>
                                <option value="C">{"C级"}</option>
                                <option value="D">{"D级"}</option>
                            </select>
                        </div>
                        <div class="form-group">
                            <label>{"信用分数："}</label>
                            <input type="number" value={self.rating_score.clone()} oninput={link.callback(|e: InputEvent| {
                                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                Msg::SetRatingScore(input.value())
                            })} />
                        </div>
                        <div class="form-group">
                            <label>{"信用额度："}</label>
                            <input type="number" step="0.01" value={self.rating_limit.clone()} oninput={link.callback(|e: InputEvent| {
                                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                Msg::SetRatingLimit(input.value())
                            })} />
                        </div>
                        <div class="form-group">
                            <label>{"信用天数："}</label>
                            <input type="number" value={self.rating_days.clone()} oninput={link.callback(|e: InputEvent| {
                                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                Msg::SetRatingDays(input.value())
                            })} />
                        </div>
                        <div class="form-group">
                            <label>{"备注："}</label>
                            <textarea value={self.rating_remark.clone()} oninput={link.callback(|e: InputEvent| {
                                let input: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
                                Msg::SetRatingRemark(input.value())
                            })} />
                        </div>
                        <div class="form-group">
                            <label>{"色卡编号："}</label>
                            <input type="text" class="form-control" value="" placeholder="仅作展示" disabled=true />
                        </div>
                        <div class="form-group">
                            <label>{"花型："}</label>
                            <div class="form-control" style="background: #f5f5f5;">{"🎨"}</div>
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={link.callback(|_| Msg::ToggleRatingModal)}>{"取消"}</button>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::SubmitRating)}>{"确定"}</button>
                    </div>
                </div>
            </div>
        }
    }

    fn view_adjust_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="modal-overlay">
                <div class="modal">
                    <div class="modal-header">
                        <h3>{"调整信用额度"}</h3>
                        <button class="btn-close" onclick={link.callback(|_| Msg::ToggleAdjustModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        <div class="form-group">
                            <label>{"调整类型："}</label>
                            <select onchange={link.callback(|e: Event| {
                                let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
                                Msg::SetAdjustType(select.value())
                            })}>
                                <option value="increase">{"增加"}</option>
                                <option value="decrease">{"减少"}</option>
                            </select>
                        </div>
                        <div class="form-group">
                            <label>{"调整金额："}</label>
                            <input type="number" step="0.01" value={self.adjust_amount.clone()} oninput={link.callback(|e: InputEvent| {
                                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                Msg::SetAdjustAmount(input.value())
                            })} />
                        </div>
                        <div class="form-group">
                            <label>{"调整原因："}</label>
                            <textarea value={self.adjust_reason.clone()} oninput={link.callback(|e: InputEvent| {
                                let input: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
                                Msg::SetAdjustReason(input.value())
                            })} />
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={link.callback(|_| Msg::ToggleAdjustModal)}>{"取消"}</button>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::SubmitAdjustment)}>{"确定"}</button>
                    </div>
                </div>
            </div>
        }
    }
}
