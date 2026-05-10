// 客户信用管理页面

use crate::utils::permissions;
use crate::utils::toast_helper;
use yew::prelude::*;
use crate::components::permission_guard::PermissionGuard;
use crate::components::{
    confirm_dialog::ConfirmDialog,
    search_bar::SearchBar,
    pagination::Pagination,
    page_header::PageHeader,
    empty_state::EmptyState,
    loading_state::LoadingState,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use crate::models::customer_credit::{
    CustomerCredit, CreditQueryParams, CreditRatingRequest, CreditLimitAdjustmentRequest,
};
use crate::services::customer_credit_service::CustomerCreditService;

pub struct CustomerCreditPage {
    credits: Vec<CustomerCredit>,
    filtered_credits: Vec<CustomerCredit>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_rating_modal: bool,
    show_adjust_modal: bool,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    selected_credit: Option<CustomerCredit>,
    // 评级表单字段
    rating_level: String,
    rating_score: String,
    rating_limit: String,
    rating_days: String,
    rating_remark: String,
    // 调整表单字段
    adjust_type: String,
    adjust_amount: String,
    adjust_reason: String,
    // 表单错误
    form_error: Option<String>,
}

pub enum Msg {
    LoadCredits,
    CreditsLoaded(Vec<CustomerCredit>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenRatingModal(CustomerCredit),
    OpenAdjustModal(CustomerCredit),
    CloseRatingModal,
    CloseAdjustModal,
    SubmitRating,
    SubmitAdjustment,
    DeleteCredit(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    // 表单字段变更
    RatingLevelChanged(String),
    RatingScoreChanged(String),
    RatingLimitChanged(String),
    RatingDaysChanged(String),
    RatingRemarkChanged(String),
    AdjustTypeChanged(String),
    AdjustAmountChanged(String),
    AdjustReasonChanged(String),
}

impl Component for CustomerCreditPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            credits: Vec::new(),
            filtered_credits: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            show_rating_modal: false,
            show_adjust_modal: false,
            show_delete_confirm: false,
            deleting_id: None,
            selected_credit: None,
            rating_level: String::from("A"),
            rating_score: String::new(),
            rating_limit: String::new(),
            rating_days: String::new(),
            rating_remark: String::new(),
            adjust_type: String::from("increase"),
            adjust_amount: String::new(),
            adjust_reason: String::new(),
            form_error: None,
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
                    customer_id: None,
                    credit_level: None,
                    status: None,
                    page: Some(1),
                    page_size: Some(1000),
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
                self.loading = false;
                self.credits = credits;
                self.apply_filter();
                true
            }
            Msg::LoadError(e) => {
                self.error = Some(e);
                self.loading = false;
                true
            }
            Msg::Search(keyword) => {
                self.search_keyword = keyword;
                self.page = 0;
                self.apply_filter();
                true
            }
            Msg::ResetSearch => {
                self.search_keyword = String::new();
                self.page = 0;
                self.apply_filter();
                true
            }
            Msg::PageChanged(page) => {
                self.page = page;
                true
            }
            Msg::OpenRatingModal(credit) => {
                self.selected_credit = Some(credit.clone());
                self.rating_level = credit.credit_level.clone().unwrap_or_else(|| "A".to_string());
                self.rating_score = credit.credit_score.map(|s| s.to_string()).unwrap_or_default();
                self.rating_limit = credit.credit_limit.clone().unwrap_or_default();
                self.rating_days = credit.credit_days.map(|d| d.to_string()).unwrap_or_default();
                self.rating_remark = String::new();
                self.form_error = None;
                self.show_rating_modal = true;
                true
            }
            Msg::OpenAdjustModal(credit) => {
                self.selected_credit = Some(credit);
                self.adjust_type = String::from("increase");
                self.adjust_amount = String::new();
                self.adjust_reason = String::new();
                self.form_error = None;
                self.show_adjust_modal = true;
                true
            }
            Msg::CloseRatingModal => {
                self.show_rating_modal = false;
                self.selected_credit = None;
                self.form_error = None;
                true
            }
            Msg::CloseAdjustModal => {
                self.show_adjust_modal = false;
                self.selected_credit = None;
                self.form_error = None;
                true
            }
            Msg::SubmitRating => {
                // 表单验证
                if self.rating_level.is_empty() {
                    self.form_error = Some("信用等级不能为空".to_string());
                    return true;
                }
                if self.rating_score.is_empty() {
                    self.form_error = Some("信用分数不能为空".to_string());
                    return true;
                }
                if self.rating_limit.is_empty() {
                    self.form_error = Some("信用额度不能为空".to_string());
                    return true;
                }

                self.form_error = None;

                if let Some(credit) = &self.selected_credit {
                    let customer_id = credit.customer_id;
                    let level = self.rating_level.clone();
                    let score: i32 = self.rating_score.parse().unwrap_or(0);
                    let limit = self.rating_limit.clone();
                    let days: i32 = self.rating_days.parse().unwrap_or(0);
                    let remark = if self.rating_remark.is_empty() { None } else { Some(self.rating_remark.clone()) };
                    
                    let req = CreditRatingRequest {
                        customer_id,
                        credit_level: level,
                        credit_score: score,
                        credit_limit: limit,
                        credit_days: days,
                        remark,
                    };
                    
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match CustomerCreditService::set_credit_rating(req).await {
                            Ok(_) => {
                                toast_helper::show_success("评级设置成功");
                                link.send_message(Msg::CloseRatingModal);
                                link.send_message(Msg::LoadCredits);
                            }
                            Err(e) => {
                                toast_helper::show_error(&format!("评级设置失败: {}", e));
                            }
                        }
                    });
                }
                false
            }
            Msg::SubmitAdjustment => {
                // 表单验证
                if self.adjust_amount.is_empty() {
                    self.form_error = Some("调整金额不能为空".to_string());
                    return true;
                }
                if self.adjust_reason.is_empty() {
                    self.form_error = Some("调整原因不能为空".to_string());
                    return true;
                }

                self.form_error = None;

                if let Some(credit) = &self.selected_credit {
                    let customer_id = credit.customer_id;
                    let adj_type = self.adjust_type.clone();
                    let amount = self.adjust_amount.clone();
                    let reason = self.adjust_reason.clone();
                    
                    let req = CreditLimitAdjustmentRequest {
                        adjustment_type: adj_type,
                        amount,
                        reason,
                    };
                    
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match CustomerCreditService::adjust_credit_limit(customer_id, req).await {
                            Ok(_) => {
                                toast_helper::show_success("额度调整成功");
                                link.send_message(Msg::CloseAdjustModal);
                                link.send_message(Msg::LoadCredits);
                            }
                            Err(e) => {
                                toast_helper::show_error(&format!("额度调整失败: {}", e));
                            }
                        }
                    });
                }
                false
            }
            Msg::DeleteCredit(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match CustomerCreditService::deactivate_credit(id).await {
                            Ok(_) => {
                                toast_helper::show_success("停用成功");
                                link.send_message(Msg::Deleted);
                            }
                            Err(e) => {
                                toast_helper::show_error(&format!("停用失败: {}", e));
                                link.send_message(Msg::CancelDelete);
                            }
                        }
                    });
                }
                false
            }
            Msg::CancelDelete => {
                self.show_delete_confirm = false;
                self.deleting_id = None;
                true
            }
            Msg::Deleted => {
                self.show_delete_confirm = false;
                self.deleting_id = None;
                ctx.link().send_message(Msg::LoadCredits);
                false
            }
            Msg::RatingLevelChanged(v) => { self.rating_level = v; true }
            Msg::RatingScoreChanged(v) => { self.rating_score = v; true }
            Msg::RatingLimitChanged(v) => { self.rating_limit = v; true }
            Msg::RatingDaysChanged(v) => { self.rating_days = v; true }
            Msg::RatingRemarkChanged(v) => { self.rating_remark = v; true }
            Msg::AdjustTypeChanged(v) => { self.adjust_type = v; true }
            Msg::AdjustAmountChanged(v) => { self.adjust_amount = v; true }
            Msg::AdjustReasonChanged(v) => { self.adjust_reason = v; true }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="customer-credit-page">
                <PageHeader title={"客户信用管理".to_string()} subtitle={Some("管理客户信用额度与评级".to_string())}>
                    <></>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索客户ID或信用等级...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载客户信用数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadCredits)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_credits.is_empty() {
                    <EmptyState
                        icon={"💳".to_string()}
                        title={"暂无客户信用数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "暂无客户信用记录".to_string()
                        } else {
                            "没有匹配搜索条件的记录".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"客户ID"}</th>
                                    <th>{"信用等级"}</th>
                                    <th>{"信用分数"}</th>
                                    <th class="numeric">{"信用额度"}</th>
                                    <th class="numeric">{"已用额度"}</th>
                                    <th class="numeric">{"可用额度"}</th>
                                    <th>{"信用天数"}</th>
                                    <th>{"状态"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_credits().iter().map(|credit| {
                                    let credit_clone = credit.clone();
                                    let credit_clone2 = credit.clone();
                                    let credit_clone3 = credit.clone();
                                    html! {
                                        <tr>
                                            <td>{credit.customer_id}</td>
                                            <td>
                                                <span class={format!("credit-badge credit-{}", credit.credit_level.as_ref().unwrap_or(&"-".to_string()).to_lowercase())}>
                                                    {credit.credit_level.as_ref().unwrap_or(&"-".to_string())}
                                                </span>
                                            </td>
                                            <td>{credit.credit_score.unwrap_or(0)}</td>
                                            <td class="numeric">{credit.credit_limit.as_ref().unwrap_or(&"-".to_string())}</td>
                                            <td class="numeric">{credit.used_credit.as_ref().unwrap_or(&"-".to_string())}</td>
                                            <td class="numeric">{credit.available_credit.as_ref().unwrap_or(&"-".to_string())}</td>
                                            <td>{credit.credit_days.unwrap_or(0)}</td>
                                            <td>
                                                <span class={format!("status-badge status-{}", credit.status.as_ref().unwrap_or(&"-".to_string()))}>
                                                    {credit.status.as_ref().unwrap_or(&"-".to_string())}
                                                </span>
                                            </td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-primary"
                                                        onclick={link.callback(move |_| Msg::OpenRatingModal(credit_clone.clone()))}
                                                    >
                                                        {"评级"}
                                                    </button>
                                                    <button
                                                        class="btn btn-sm btn-secondary"
                                                        onclick={link.callback(move |_| Msg::OpenAdjustModal(credit_clone2.clone()))}
                                                    >
                                                        {"调整额度"}
                                                    </button>
                                                    <PermissionGuard resource="customer_credit" action="delete">
                                                        <button
                                                            class="btn btn-sm btn-danger"
                                                            onclick={link.callback(move |_| Msg::DeleteCredit(credit_clone3.customer_id))}
                                                        >
                                                            {"停用"}
                                                        </button>
                                                    </PermissionGuard>
                                                </div>
                                            </td>
                                        </tr>
                                    }
                                })}
                            </tbody>
                        </table>

                        <Pagination
                            current_page={self.page}
                            page_size={self.page_size}
                            total={self.filtered_credits.len() as u64}
                            on_page_change={link.callback(|page| Msg::PageChanged(page))}
                        />
                    </div>
                }

                // 评级弹窗
                if self.show_rating_modal {
                    {self.render_rating_modal(ctx)}
                }

                // 调整额度弹窗
                if self.show_adjust_modal {
                    {self.render_adjust_modal(ctx)}
                }

                // 删除确认对话框
                <ConfirmDialog
                    title={"确认停用".to_string()}
                    message={"确定要停用此客户的信用额度吗？此操作不可撤销。".to_string()}
                    confirm_text={"停用".to_string()}
                    cancel_text={"取消".to_string()}
                    confirm_class={"btn-danger".to_string()}
                    on_confirm={link.callback(|_| Msg::ConfirmDelete)}
                    on_cancel={link.callback(|_| Msg::CancelDelete)}
                    visible={self.show_delete_confirm}
                />
            </div>
        }
    }
}

impl CustomerCreditPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_credits = self.credits.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_credits = self.credits.iter()
                .filter(|c| {
                    c.customer_id.to_string().contains(&keyword) ||
                    c.credit_level.as_ref().map(|l| l.to_lowercase().contains(&keyword)).unwrap_or(false) ||
                    c.status.as_ref().map(|s| s.to_lowercase().contains(&keyword)).unwrap_or(false)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_credits(&self) -> Vec<CustomerCredit> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_credits[start..end.min(self.filtered_credits.len())].to_vec()
    }

    fn render_rating_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        let on_level_change = link.batch_callback(|e: Event| {
            e.target().and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok()).map(|input| Msg::RatingLevelChanged(input.value()))
        });
        let on_score_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::RatingScoreChanged(input.value()))
        });
        let on_limit_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::RatingLimitChanged(input.value()))
        });
        let on_days_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::RatingDaysChanged(input.value()))
        });
        let on_remark_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::RatingRemarkChanged(input.value()))
        });

        html! {
            <div class="modal-overlay" onclick={link.callback(|_| Msg::CloseRatingModal)}>
                <div class="modal-content" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <div class="modal-header">
                        <h3>{"设置信用评级"}</h3>
                        <button class="close-btn" onclick={link.callback(|_| Msg::CloseRatingModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        if let Some(err) = &self.form_error {
                            <div class="form-error">{err}</div>
                        }
                        <div class="form-group">
                            <label>{"信用等级 *"}</label>
                            <select class="form-input" value={self.rating_level.clone()} onchange={on_level_change}>
                                <option value="A">{"A级"}</option>
                                <option value="B">{"B级"}</option>
                                <option value="C">{"C级"}</option>
                                <option value="D">{"D级"}</option>
                            </select>
                        </div>
                        <div class="form-group">
                            <label>{"信用分数 *"}</label>
                            <input
                                type="number"
                                class="form-input"
                                value={self.rating_score.clone()}
                                oninput={on_score_change}
                                placeholder="请输入信用分数"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"信用额度 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.rating_limit.clone()}
                                oninput={on_limit_change}
                                placeholder="请输入信用额度"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"信用天数"}</label>
                            <input
                                type="number"
                                class="form-input"
                                value={self.rating_days.clone()}
                                oninput={on_days_change}
                                placeholder="请输入信用天数"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"备注"}</label>
                            <textarea
                                class="form-input"
                                value={self.rating_remark.clone()}
                                oninput={on_remark_change}
                                placeholder="请输入备注信息"
                                rows="3"
                            />
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={link.callback(|_| Msg::CloseRatingModal)}>
                            {"取消"}
                        </button>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::SubmitRating)}>
                            {"保存评级"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_adjust_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        let on_type_change = link.batch_callback(|e: Event| {
            e.target().and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok()).map(|input| Msg::AdjustTypeChanged(input.value()))
        });
        let on_amount_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::AdjustAmountChanged(input.value()))
        });
        let on_reason_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::AdjustReasonChanged(input.value()))
        });

        html! {
            <div class="modal-overlay" onclick={link.callback(|_| Msg::CloseAdjustModal)}>
                <div class="modal-content" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <div class="modal-header">
                        <h3>{"调整信用额度"}</h3>
                        <button class="close-btn" onclick={link.callback(|_| Msg::CloseAdjustModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        if let Some(err) = &self.form_error {
                            <div class="form-error">{err}</div>
                        }
                        <div class="form-group">
                            <label>{"调整类型 *"}</label>
                            <select class="form-input" value={self.adjust_type.clone()} onchange={on_type_change}>
                                <option value="increase">{"增加额度"}</option>
                                <option value="decrease">{"减少额度"}</option>
                            </select>
                        </div>
                        <div class="form-group">
                            <label>{"调整金额 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.adjust_amount.clone()}
                                oninput={on_amount_change}
                                placeholder="请输入调整金额"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"调整原因 *"}</label>
                            <textarea
                                class="form-input"
                                value={self.adjust_reason.clone()}
                                oninput={on_reason_change}
                                placeholder="请输入调整原因"
                                rows="3"
                            />
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={link.callback(|_| Msg::CloseAdjustModal)}>
                            {"取消"}
                        </button>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::SubmitAdjustment)}>
                            {"确认调整"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}
