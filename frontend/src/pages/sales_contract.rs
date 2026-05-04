use crate::utils::permissions;
use crate::utils::toast_helper;
/// 销售合同管理页面

use yew::prelude::*;
use crate::components::permission_guard::PermissionGuard;
use wasm_bindgen::JsCast;
use crate::models::sales_contract::{
    SalesContract, SalesContractQueryParams, CreateSalesContractRequest, ExecuteSalesContractRequest,
};
use crate::services::sales_contract_service::SalesContractService;
use crate::services::crud_service::CrudService;

#[derive(Debug, Clone, PartialEq)]
pub enum ContractStatus {
    /// 草稿
    Draft,
    /// 已审核
    Approved,
    /// 执行中
    Executing,
    /// 已完成
    Completed,
    /// 已取消
    Cancelled,
}

impl ContractStatus {
    /// 从字符串转换为状态枚举
    pub fn from_str(s: &str) -> Self {
        match s {
            "draft" => ContractStatus::Draft,
            "approved" => ContractStatus::Approved,
            "executing" => ContractStatus::Executing,
            "completed" => ContractStatus::Completed,
            "cancelled" => ContractStatus::Cancelled,
            _ => ContractStatus::Draft,
        }
    }

    /// 获取状态显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            ContractStatus::Draft => "草稿",
            ContractStatus::Approved => "已审核",
            ContractStatus::Executing => "执行中",
            ContractStatus::Completed => "已完成",
            ContractStatus::Cancelled => "已取消",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SalesContractState {
    pub printing_contract: Option<crate::models::sales_contract::SalesContract>,
    pub print_trigger: bool,
    /// 合同列表
    pub contracts: Vec<SalesContract>,
    /// 加载状态
    pub loading: bool,
    /// 错误信息
    pub error: Option<String>,
    /// 当前页码
    pub page: i64,
    /// 每页数量
    pub page_size: i64,
    /// 总数
    pub total: i64,
    /// 搜索关键词
    pub keyword: String,
    /// 筛选状态
    pub status_filter: Option<String>,
    /// 是否显示创建弹窗
    pub show_create_modal: bool,
    /// 是否显示执行弹窗
    pub show_execute_modal: bool,
    /// 是否显示取消弹窗
    pub show_cancel_modal: bool,
    /// 当前操作的合同ID
    pub current_contract_id: Option<i32>,
}

impl Default for SalesContractState {
    fn default() -> Self {
        Self {
            contracts: Vec::new(),
            loading: false,
            error: None,
            page: 1,
            page_size: 10,
            total: 0,
            keyword: String::new(),
            status_filter: None,
            show_create_modal: false,
            printing_contract: None,
            print_trigger: false,
            show_execute_modal: false,
            show_cancel_modal: false,
            current_contract_id: None,
        }
    }
}

pub struct SalesContractPage {
    state: SalesContractState,
}

pub enum Msg {
    /// 加载合同列表
    LoadContracts,
    /// 设置合同列表
    SetContracts(Vec<SalesContract>, i64),
    /// 设置加载状态
    SetLoading(bool),
    /// 设置错误信息
    SetError(Option<String>),
    /// 翻页
    ChangePage(i64),
    /// 改变每页数量
    ChangePageSize(i64),
    /// 搜索关键词改变
    SearchKeyword(String),
    /// 筛选状态改变
    FilterStatus(Option<String>),
    /// 显示创建弹窗
    ShowCreateModal,
    /// 隐藏创建弹窗
    HideCreateModal,
    /// 创建合同成功
    CreateContract(CreateSalesContractRequest),
    /// 审核合同
    ApproveContract(i32),
    /// 显示执行弹窗
    ShowExecuteModal(i32),
    /// 隐藏执行弹窗
    HideExecuteModal,
    /// 执行合同
    ExecuteContract(i32, ExecuteSalesContractRequest),
    /// 显示取消弹窗
    ShowCancelModal(i32),
    /// 隐藏取消弹窗
    HideCancelModal,
    /// 取消合同
    CancelContract(i32, String),
}

impl Component for SalesContractPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            state: SalesContractState::default(),
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadContracts);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadContracts => {
                self.state.loading = true;
                self.state.error = None;
                let params = SalesContractQueryParams {
                    keyword: if self.state.keyword.is_empty() { None } else { Some(self.state.keyword.clone()) },
                    status: self.state.status_filter.clone(),
                    page: Some(self.state.page),
                    page_size: Some(self.state.page_size),
                    customer_id: None,
                };
                let link = ctx.link().clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match SalesContractService::list_contracts(params).await {
                        Ok(response) => {
                            link.send_message(Msg::SetContracts(response.items, response.total));
                        }
                        Err(e) => {
                            link.send_message(Msg::SetError(Some(e)));
                        }
                    }
                });
                false
            }
            Msg::SetContracts(contracts, total) => {
                self.state.contracts = contracts;
                self.state.total = total;
                self.state.loading = false;
                true
            }
            Msg::SetLoading(loading) => {
                self.state.loading = loading;
                true
            }
            Msg::SetError(error) => {
                self.state.error = error;
                self.state.loading = false;
                true
            }
            Msg::ChangePage(page) => {
                self.state.page = page;
                ctx.link().send_message(Msg::LoadContracts);
                false
            }
            Msg::ChangePageSize(page_size) => {
                self.state.page_size = page_size;
                self.state.page = 1;
                ctx.link().send_message(Msg::LoadContracts);
                false
            }
            Msg::SearchKeyword(keyword) => {
                self.state.keyword = keyword;
                self.state.page = 1;
                ctx.link().send_message(Msg::LoadContracts);
                false
            }
            Msg::FilterStatus(status) => {
                self.state.status_filter = status;
                self.state.page = 1;
                ctx.link().send_message(Msg::LoadContracts);
                false
            }
            Msg::ShowCreateModal => {
                self.state.show_create_modal = true;
                true
            }
            Msg::HideCreateModal => {
                self.state.show_create_modal = false;
                true
            }
            Msg::CreateContract(req) => {
                self.state.loading = true;
                let link = ctx.link().clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match SalesContractService::create_contract(req).await {
                        Ok(_) => {
                            link.send_message(Msg::HideCreateModal);
                            link.send_message(Msg::LoadContracts);
                        }
                        Err(e) => {
                            link.send_message(Msg::SetError(Some(e)));
                        }
                    }
                });
                false
            }
            Msg::ApproveContract(id) => {
                self.state.loading = true;
                let link = ctx.link().clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match SalesContractService::approve_contract(id).await {
                        Ok(_) => {
                            link.send_message(Msg::LoadContracts);
                        }
                        Err(e) => {
                            link.send_message(Msg::SetError(Some(e)));
                        }
                    }
                });
                false
            }
            Msg::ShowExecuteModal(id) => {
                self.state.current_contract_id = Some(id);
                self.state.show_execute_modal = true;
                true
            }
            Msg::HideExecuteModal => {
                self.state.show_execute_modal = false;
                self.state.current_contract_id = None;
                true
            }
            Msg::ExecuteContract(id, req) => {
                self.state.loading = true;
                let link = ctx.link().clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match SalesContractService::execute_contract(id, req).await {
                        Ok(_) => {
                            link.send_message(Msg::HideExecuteModal);
                            link.send_message(Msg::LoadContracts);
                        }
                        Err(e) => {
                            link.send_message(Msg::SetError(Some(e)));
                        }
                    }
                });
                false
            }
            Msg::ShowCancelModal(id) => {
                self.state.current_contract_id = Some(id);
                self.state.show_cancel_modal = true;
                true
            }
            Msg::HideCancelModal => {
                self.state.show_cancel_modal = false;
                self.state.current_contract_id = None;
                true
            }
            Msg::CancelContract(id, reason) => {
                self.state.loading = true;
                let link = ctx.link().clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match SalesContractService::cancel_contract(id, reason).await {
                        Ok(_) => {
                            link.send_message(Msg::HideCancelModal);
                            link.send_message(Msg::LoadContracts);
                        }
                        Err(e) => {
                            link.send_message(Msg::SetError(Some(e)));
                        }
                    }
                });
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="sales-contract-page">
                <div class="page-header">
                    <h1>{"销售合同管理"}</h1>
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::ShowCreateModal)}>
                        {"新建合同"}
                    </button>
                </div>

                // 搜索和筛选区域
                <div class="search-bar">
                    <input
                        type="text"
                        placeholder="搜索合同编号或名称..."
                        value={self.state.keyword.clone()}
                        oninput={ctx.link().batch_callback(|e: InputEvent| {
                            let target = e.target()?;
                            if let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() {
                                Some(Msg::SearchKeyword(input.value()))
                            } else {
                                Some(Msg::SearchKeyword(String::new()))
                            }
                        })}
                    />
                    <select
                        value={self.state.status_filter.clone().unwrap_or_default()}
                        onchange={ctx.link().batch_callback(|e: Event| {
                            let target = e.target()?;
                            if let Ok(select) = target.dyn_into::<web_sys::HtmlSelectElement>() {
                                let value = select.value();
                                Some(Msg::FilterStatus(if value.is_empty() { None } else { Some(value) }))
                            } else {
                                Some(Msg::FilterStatus(None))
                            }
                        })}
                    >
                        <option value="">{"全部状态"}</option>
                        <option value="draft">{"草稿"}</option>
                        <option value="approved">{"已审核"}</option>
                        <option value="executing">{"执行中"}</option>
                        <option value="completed">{"已完成"}</option>
                        <option value="cancelled">{"已取消"}</option>
                    </select>
                    <button onclick={ctx.link().callback(|_| Msg::LoadContracts)}>{"刷新"}</button>
                </div>

                // 加载状态
                if self.state.loading {
                    <div class="loading">{"加载中..."}</div>
                }

                // 错误信息
                if let Some(error) = &self.state.error {
                    <div class="error-message">{error}</div>
                }

                // 合同列表
                <div class="contract-table">
                    <table>
                        <thead>
                            <tr>
                                <th>{"合同编号"}</th>
                                <th>{"合同名称"}</th>
                                <th>{"客户"}</th>
                                <th>{"总金额"}</th>
                                <th>{"交货日期"}</th>
                                <th>{"状态"}</th>
                                <th>{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {for self.state.contracts.iter().map(|contract| {
                                let status = ContractStatus::from_str(&contract.status);
                                let contract_id = contract.id;
                                let contract_id2 = contract.id;
                                let contract_id3 = contract.id;
                                html! {
                                    <tr>
                                        <td>{&contract.contract_no}</td>
                                        <td>{&contract.contract_name}</td>
                                        <td>{contract.customer_name.as_deref().unwrap_or("-")}</td>
                                        <td>{format!("{:.2}", contract.total_amount)}</td>
                                        <td>{&contract.delivery_date}</td>
                                        <td>{status.display_name()}</td>
                                        <td>
                                            <div class="action-buttons">
                                                if status == ContractStatus::Draft {
                                                    <button onclick={ctx.link().callback(move |_| Msg::ApproveContract(contract_id))}>
                                                        {"审核"}
                                                    </button>
                                                }
                                                if status == ContractStatus::Approved || status == ContractStatus::Executing {
                                                    <button onclick={ctx.link().callback(move |_| Msg::ShowExecuteModal(contract_id2))}>
                                                        {"执行"}
                                                    </button>
                                                }
                                                if status == ContractStatus::Draft || status == ContractStatus::Approved {
                                                    <button onclick={ctx.link().callback(move |_| Msg::ShowCancelModal(contract_id3))}>
                                                        {"取消"}
                                                    </button>
                                                }
                                            </div>
                                        </td>
                                    </tr>
                                }
                            })}
                        </tbody>
                    </table>
                </div>

                // 分页
                <div class="pagination">
                    <span>{format!("共 {} 条记录", self.state.total)}</span>
                    <button
                        disabled={self.state.page <= 1}
                        onclick={ctx.link().callback(|_| Msg::ChangePage(0))}
                    >
                        {"上一页"}
                    </button>
                    <span>{format!("第 {} 页", self.state.page)}</span>
                    <button
                        disabled={self.state.page * self.state.page_size >= self.state.total}
                        onclick={ctx.link().callback(|_| Msg::ChangePage(2))}
                    >
                        {"下一页"}
                    </button>
                    <select
                        value={self.state.page_size.to_string()}
                        onchange={ctx.link().batch_callback(|e: Event| {
                            let target = e.target()?;
                            if let Ok(select) = target.dyn_into::<web_sys::HtmlSelectElement>() {
                                if let Ok(size) = select.value().parse::<i64>() {
                                    Some(Msg::ChangePageSize(size))
                                } else {
                                    Some(Msg::ChangePageSize(10))
                                }
                            } else {
                                Some(Msg::ChangePageSize(10))
                            }
                        })}
                    >
                        <option value="10">{"10条/页"}</option>
                        <option value="20">{"20条/页"}</option>
                        <option value="50">{"50条/页"}</option>
                    </select>
                </div>

                // 创建合同弹窗
                if self.state.show_create_modal {
                    <CreateContractModal
                        on_close={ctx.link().callback(|_| Msg::HideCreateModal)}
                        on_submit={ctx.link().callback(|req| Msg::CreateContract(req))}
                    />
                }

                // 执行合同弹窗
                if self.state.show_execute_modal {
                    <ExecuteContractModal
                        contract_id={self.state.current_contract_id.unwrap_or(0)}
                        on_close={ctx.link().callback(|_| Msg::HideExecuteModal)}
                        on_submit={ctx.link().callback(|(id, req)| Msg::ExecuteContract(id, req))}
                    />
                }

                // 取消合同弹窗
                if self.state.show_cancel_modal {
                    <CancelContractModal
                        contract_id={self.state.current_contract_id.unwrap_or(0)}
                        on_close={ctx.link().callback(|_| Msg::HideCancelModal)}
                        on_submit={ctx.link().callback(|(id, reason)| Msg::CancelContract(id, reason))}
                    />
                }
            </div>
        }
    }
}

// ============ 子组件 ============

/// 创建合同弹窗组件
#[derive(Properties, PartialEq)]
pub struct CreateContractModalProps {
    pub on_close: Callback<()>,
    pub on_submit: Callback<CreateSalesContractRequest>,
}

#[derive(Clone, PartialEq)]
pub struct CreateContractModalState {
    contract_no: String,
    contract_name: String,
    customer_id: i32,
    total_amount: String,
    payment_terms: String,
    delivery_date: String,
    remark: String,
}

#[derive(Clone, PartialEq)]
pub struct CreateContractModal {
    state: CreateContractModalState,
}

pub enum CreateContractMsg {
    UpdateContractNo(String),
    UpdateContractName(String),
    UpdateCustomerId(String),
    UpdateTotalAmount(String),
    UpdatePaymentTerms(String),
    UpdateDeliveryDate(String),
    UpdateRemark(String),
    Submit,
}

impl Component for CreateContractModal {
    type Message = CreateContractMsg;
    type Properties = CreateContractModalProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            state: CreateContractModalState {
                contract_no: String::new(),
                contract_name: String::new(),
                customer_id: 0,
                total_amount: String::new(),
                payment_terms: String::new(),
                delivery_date: String::new(),
                remark: String::new(),
            },
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            CreateContractMsg::UpdateContractNo(val) => {
                self.state.contract_no = val;
                true
            }
            CreateContractMsg::UpdateContractName(val) => {
                self.state.contract_name = val;
                true
            }
            CreateContractMsg::UpdateCustomerId(val) => {
                if let Ok(id) = val.parse::<i32>() {
                    self.state.customer_id = id;
                }
                true
            }
            CreateContractMsg::UpdateTotalAmount(val) => {
                self.state.total_amount = val;
                true
            }
            CreateContractMsg::UpdatePaymentTerms(val) => {
                self.state.payment_terms = val;
                true
            }
            CreateContractMsg::UpdateDeliveryDate(val) => {
                self.state.delivery_date = val;
                true
            }
            CreateContractMsg::UpdateRemark(val) => {
                self.state.remark = val;
                true
            }
            CreateContractMsg::Submit => {
                let req = CreateSalesContractRequest {
                    contract_no: self.state.contract_no.clone(),
                    contract_name: self.state.contract_name.clone(),
                    customer_id: self.state.customer_id,
                    total_amount: self.state.total_amount.clone(),
                    payment_terms: Some(self.state.payment_terms.clone()),
                    delivery_date: self.state.delivery_date.clone(),
                    remark: if self.state.remark.is_empty() { None } else { Some(self.state.remark.clone()) },
                };
                ctx.props().on_submit.emit(req);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let props = ctx.props();
        
        let on_input = |msg: fn(String) -> CreateContractMsg| {
            ctx.link().batch_callback(move |e: InputEvent| {
                let target = e.target()?;
                let input = target.unchecked_into::<web_sys::HtmlInputElement>();
                Some(msg(input.value()))
            })
        };

        html! {
            <div class="modal-overlay">
                <div class="modal">
                    <div class="modal-header">
                        <h2>{"新建销售合同"}</h2>
                        <button onclick={props.on_close.reform(|_| ())}>{"关闭"}</button>
                    </div>
                    <div class="modal-body">
                        <div class="form-group">
                            <label>{"合同编号"}</label>
                            <input type="text" value={self.state.contract_no.clone()} oninput={on_input(CreateContractMsg::UpdateContractNo)} />
                        </div>
                        <div class="form-group">
                            <label>{"合同名称"}</label>
                            <input type="text" value={self.state.contract_name.clone()} oninput={on_input(CreateContractMsg::UpdateContractName)} />
                        </div>
                        <div class="form-group">
                            <label>{"客户ID"}</label>
                            <input type="number" value={self.state.customer_id.to_string()} oninput={on_input(CreateContractMsg::UpdateCustomerId)} />
                        </div>
                        <div class="form-group">
                            <label>{"总金额"}</label>
                            <input type="number" step="0.01" value={self.state.total_amount.clone()} oninput={on_input(CreateContractMsg::UpdateTotalAmount)} />
                        </div>
                        <div class="form-group">
                            <label>{"付款条款"}</label>
                            <input type="text" value={self.state.payment_terms.clone()} oninput={on_input(CreateContractMsg::UpdatePaymentTerms)} />
                        </div>
                        <div class="form-group">
                            <label>{"交货日期"}</label>
                            <input type="date" value={self.state.delivery_date.clone()} oninput={on_input(CreateContractMsg::UpdateDeliveryDate)} />
                        </div>
                        <div class="form-group">
                            <label>{"备注"}</label>
                            <input type="text" value={self.state.remark.clone()} oninput={on_input(CreateContractMsg::UpdateRemark)} />
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button onclick={props.on_close.reform(|_| ())}>{"取消"}</button>
                        <button class="btn-primary" onclick={ctx.link().callback(|_| CreateContractMsg::Submit)}>{"提交"}</button>
                    </div>
                </div>
            </div>
        }
    }
}

/// 执行合同弹窗组件
#[derive(Properties, PartialEq)]
pub struct ExecuteContractModalProps {
    pub contract_id: i32,
    pub on_close: Callback<()>,
    pub on_submit: Callback<(i32, ExecuteSalesContractRequest)>,
}




pub struct ExecuteContractModal {
    execution_type: String,
    execution_amount: String,
    related_bill_type: String,
    related_bill_id: String,
    remark: String,
}

pub enum ExecuteMsg {
    UpdateExecutionType(String),
    UpdateExecutionAmount(String),
    UpdateRelatedBillType(String),
    UpdateRelatedBillId(String),
    UpdateRemark(String),
    Submit,
}

impl Component for ExecuteContractModal {
    type Message = ExecuteMsg;
    type Properties = ExecuteContractModalProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            execution_type: "发货".to_string(),
            execution_amount: "0".to_string(),
            related_bill_type: String::new(),
            related_bill_id: String::new(),
            remark: String::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            ExecuteMsg::UpdateExecutionType(val) => self.execution_type = val,
            ExecuteMsg::UpdateExecutionAmount(val) => self.execution_amount = val,
            ExecuteMsg::UpdateRelatedBillType(val) => self.related_bill_type = val,
            ExecuteMsg::UpdateRelatedBillId(val) => self.related_bill_id = val,
            ExecuteMsg::UpdateRemark(val) => self.remark = val,
            ExecuteMsg::Submit => {
                use std::str::FromStr;
                let req = ExecuteSalesContractRequest {
                    execution_type: self.execution_type.clone(),
                    execution_amount: self.execution_amount.clone(),
                    related_bill_type: if self.related_bill_type.is_empty() { None } else { Some(self.related_bill_type.clone()) },
                    related_bill_id: i32::from_str(&self.related_bill_id).ok(),
                    remark: if self.remark.is_empty() { None } else { Some(self.remark.clone()) },
                };
                ctx.props().on_submit.emit((ctx.props().contract_id, req));
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let props = ctx.props();
        
        let on_input = |f: fn(String) -> ExecuteMsg| {
            ctx.link().callback(move |e: InputEvent| {
                use wasm_bindgen::JsCast;
                let input = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                f(input.value())
            })
        };

        let on_select = |f: fn(String) -> ExecuteMsg| {
            ctx.link().callback(move |e: Event| {
                use wasm_bindgen::JsCast;
                let select = e.target_unchecked_into::<web_sys::HtmlSelectElement>();
                f(select.value())
            })
        };

        html! {
            <div class="modal-overlay">
                <div class="modal">
                    <div class="modal-header">
                        <h2>{format!("执行合同 #{}", props.contract_id)}</h2>
                        <button onclick={props.on_close.reform(|_| ())}>{"关闭"}</button>
                    </div>
                    <div class="modal-body" style="display: flex; flex-direction: column; gap: 10px;">
                        <div>
                            <label>{"执行类型: "}</label>
                            <select onchange={on_select(ExecuteMsg::UpdateExecutionType)} value={self.execution_type.clone()}>
                                <option value="发货">{"发货/收货"}</option>
                                <option value="付款">{"付款/收款"}</option>
                            </select>
                        </div>
                        <div>
                            <label>{"执行金额: "}</label>
                            <input type="number" step="0.01" value={self.execution_amount.clone()} oninput={on_input(ExecuteMsg::UpdateExecutionAmount)} />
                        </div>
                        <div>
                            <label>{"关联单据类型: "}</label>
                            <input type="text" value={self.related_bill_type.clone()} oninput={on_input(ExecuteMsg::UpdateRelatedBillType)} placeholder="可选" />
                        </div>
                        <div>
                            <label>{"关联单据ID: "}</label>
                            <input type="number" value={self.related_bill_id.clone()} oninput={on_input(ExecuteMsg::UpdateRelatedBillId)} placeholder="可选" />
                        </div>
                        <div>
                            <label>{"备注: "}</label>
                            <input type="text" value={self.remark.clone()} oninput={on_input(ExecuteMsg::UpdateRemark)} placeholder="可选" />
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button onclick={props.on_close.reform(|_| ())}>{"取消"}</button>
                        <button class="primary" onclick={ctx.link().callback(|_| ExecuteMsg::Submit)}>{"确认执行"}</button>
                    </div>
                </div>
            </div>
        }
    }
}

/// 取消合同弹窗组件
#[derive(Properties, PartialEq)]
pub struct CancelContractModalProps {
    pub contract_id: i32,
    pub on_close: Callback<()>,
    pub on_submit: Callback<(i32, String)>,
}




pub struct CancelContractModalState {
    reason: String,
}

pub enum CancelMsg {
    UpdateReason(String),
    Submit,
}

pub struct CancelContractModal { pub reason: String }

impl Component for CancelContractModal {
    type Message = CancelMsg;
    type Properties = CancelContractModalProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self { reason: String::new() }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            CancelMsg::UpdateReason(val) => self.reason = val,
            CancelMsg::Submit => {
                ctx.props().on_submit.emit((ctx.props().contract_id, self.reason.clone()));
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let props = ctx.props();
        html! {
            <div class="modal-overlay">
                <div class="modal">
                    <div class="modal-header">
                        <h2>{format!("取消合同 #{}", props.contract_id)}</h2>
                        <button onclick={props.on_close.reform(|_| ())}>{"关闭"}</button>
                    </div>
                    <div class="modal-body" style="display: flex; flex-direction: column; gap: 10px;">
                        <div>
                            <label>{"取消原因: "}</label>
                            <textarea 
                                value={self.reason.clone()} 
                                oninput={ctx.link().callback(|e: InputEvent| {
                                    use wasm_bindgen::JsCast;
                                    CancelMsg::UpdateReason(e.target_unchecked_into::<web_sys::HtmlTextAreaElement>().value())
                                })} 
                                rows="3" 
                                style="width: 100%;"
                            ></textarea>
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button onclick={props.on_close.reform(|_| ())}>{"关闭"}</button>
                        <button class="danger" onclick={ctx.link().callback(|_| CancelMsg::Submit)}>{"确认取消"}</button>
                    </div>
                </div>
            </div>
        }
    }
}
