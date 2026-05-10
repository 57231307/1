use crate::utils::permissions;
use crate::utils::toast_helper;
/// 采购合同管理页面

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
use crate::models::purchase_contract::{
    PurchaseContract, PurchaseContractQueryParams, CreatePurchaseContractRequest, ExecutePurchaseContractRequest,
};
use crate::services::purchase_contract_service::PurchaseContractService;
use crate::services::crud_service::CrudService;

/// 采购合同状态枚举
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

pub struct PurchaseContractPage {
    contracts: Vec<PurchaseContract>,
    filtered_contracts: Vec<PurchaseContract>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_contract: Option<PurchaseContract>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    show_execute_modal: bool,
    executing_id: Option<i32>,
    show_cancel_modal: bool,
    cancelling_id: Option<i32>,
    // 表单字段
    form_contract_no: String,
    form_contract_name: String,
    form_supplier_id: String,
    form_total_amount: String,
    form_payment_terms: String,
    form_delivery_date: String,
    form_remark: String,
    form_error: Option<String>,
    // 执行表单
    form_execution_type: String,
    form_execution_amount: String,
    form_related_bill_type: String,
    form_related_bill_id: String,
    form_execution_remark: String,
    // 取消表单
    form_cancel_reason: String,
}

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    Create,
    Edit,
}

pub enum Msg {
    LoadData,
    DataLoaded(Vec<PurchaseContract>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    OpenEditModal(PurchaseContract),
    CloseModal,
    SubmitForm,
    FormSubmitted,
    DeleteContract(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    ApproveContract(i32),
    ShowExecuteModal(i32),
    HideExecuteModal,
    SubmitExecute,
    ShowCancelModal(i32),
    HideCancelModal,
    SubmitCancel,
    // 表单字段变更
    FormContractNoChanged(String),
    FormContractNameChanged(String),
    FormSupplierIdChanged(String),
    FormTotalAmountChanged(String),
    FormPaymentTermsChanged(String),
    FormDeliveryDateChanged(String),
    FormRemarkChanged(String),
    FormExecutionTypeChanged(String),
    FormExecutionAmountChanged(String),
    FormRelatedBillTypeChanged(String),
    FormRelatedBillIdChanged(String),
    FormExecutionRemarkChanged(String),
    FormCancelReasonChanged(String),
}

impl Component for PurchaseContractPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            contracts: Vec::new(),
            filtered_contracts: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            show_modal: false,
            modal_mode: ModalMode::Create,
            editing_contract: None,
            show_delete_confirm: false,
            deleting_id: None,
            show_execute_modal: false,
            executing_id: None,
            show_cancel_modal: false,
            cancelling_id: None,
            form_contract_no: String::new(),
            form_contract_name: String::new(),
            form_supplier_id: String::new(),
            form_total_amount: String::new(),
            form_payment_terms: String::new(),
            form_delivery_date: String::new(),
            form_remark: String::new(),
            form_error: None,
            form_execution_type: "发货".to_string(),
            form_execution_amount: String::new(),
            form_related_bill_type: String::new(),
            form_related_bill_id: String::new(),
            form_execution_remark: String::new(),
            form_cancel_reason: String::new(),
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadData);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadData => {
                self.loading = true;
                self.error = None;
                let link = ctx.link().clone();
                spawn_local(async move {
                    let params = PurchaseContractQueryParams {
                        keyword: None,
                        status: None,
                        supplier_id: None,
                        page: Some(1),
                        page_size: Some(1000),
                    };
                    match PurchaseContractService::list_contracts(params).await {
                        Ok(response) => link.send_message(Msg::DataLoaded(response.items)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::DataLoaded(data) => {
                self.loading = false;
                self.contracts = data;
                self.apply_filter();
                true
            }
            Msg::LoadError(err) => {
                self.error = Some(err);
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
            Msg::OpenCreateModal => {
                self.reset_form();
                self.editing_contract = None;
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(contract) => {
                self.form_contract_no = contract.contract_no.clone();
                self.form_contract_name = contract.contract_name.clone();
                self.form_supplier_id = contract.supplier_id.to_string();
                self.form_total_amount = contract.total_amount.clone();
                self.form_payment_terms = contract.payment_terms.clone().unwrap_or_default();
                self.form_delivery_date = contract.delivery_date.clone();
                self.form_remark = contract.remark.clone().unwrap_or_default();
                self.form_error = None;
                self.editing_contract = Some(contract);
                self.modal_mode = ModalMode::Edit;
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.editing_contract = None;
                self.form_error = None;
                true
            }
            Msg::SubmitForm => {
                if self.form_contract_no.is_empty() {
                    self.form_error = Some("合同编号不能为空".to_string());
                    return true;
                }
                if self.form_contract_name.is_empty() {
                    self.form_error = Some("合同名称不能为空".to_string());
                    return true;
                }
                if self.form_supplier_id.is_empty() {
                    self.form_error = Some("供应商不能为空".to_string());
                    return true;
                }
                if self.form_total_amount.is_empty() {
                    self.form_error = Some("总金额不能为空".to_string());
                    return true;
                }
                if self.form_delivery_date.is_empty() {
                    self.form_error = Some("交货日期不能为空".to_string());
                    return true;
                }

                self.form_error = None;

                let supplier_id = self.form_supplier_id.parse::<i32>().unwrap_or(0);
                let req = CreatePurchaseContractRequest {
                    contract_no: self.form_contract_no.clone(),
                    contract_name: self.form_contract_name.clone(),
                    supplier_id,
                    total_amount: self.form_total_amount.clone(),
                    payment_terms: if self.form_payment_terms.is_empty() { None } else { Some(self.form_payment_terms.clone()) },
                    delivery_date: self.form_delivery_date.clone(),
                    remark: if self.form_remark.is_empty() { None } else { Some(self.form_remark.clone()) },
                };

                let link = ctx.link().clone();

                if self.modal_mode == ModalMode::Edit {
                    if let Some(contract) = &self.editing_contract {
                        let id = contract.id;
                        let update_req = crate::models::purchase_contract::CreatePurchaseContractRequest {
                            contract_no: self.form_contract_no.clone(),
                            contract_name: self.form_contract_name.clone(),
                            supplier_id,
                            total_amount: self.form_total_amount.clone(),
                            payment_terms: if self.form_payment_terms.is_empty() { None } else { Some(self.form_payment_terms.clone()) },
                            delivery_date: self.form_delivery_date.clone(),
                            remark: if self.form_remark.is_empty() { None } else { Some(self.form_remark.clone()) },
                        };
                        spawn_local(async move {
                            match PurchaseContractService::create_contract(update_req).await {
                                Ok(_) => {
                                    toast_helper::show_success("更新成功");
                                    link.send_message(Msg::FormSubmitted);
                                }
                                Err(e) => {
                                    toast_helper::show_error(&format!("更新失败: {}", e));
                                }
                            }
                        });
                    }
                } else {
                    spawn_local(async move {
                        match PurchaseContractService::create_contract(req).await {
                            Ok(_) => {
                                toast_helper::show_success("创建成功");
                                link.send_message(Msg::FormSubmitted);
                            }
                            Err(e) => {
                                toast_helper::show_error(&format!("创建失败: {}", e));
                            }
                        }
                    });
                }
                false
            }
            Msg::FormSubmitted => {
                self.show_modal = false;
                self.editing_contract = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::DeleteContract(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match PurchaseContractService::cancel_contract(id, "用户删除".to_string()).await {
                            Ok(_) => {
                                toast_helper::show_success("删除成功");
                                link.send_message(Msg::Deleted);
                            }
                            Err(e) => {
                                toast_helper::show_error(&format!("删除失败: {}", e));
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
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::ApproveContract(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseContractService::approve_contract(id).await {
                        Ok(_) => {
                            toast_helper::show_success("审核通过");
                            link.send_message(Msg::LoadData);
                        }
                        Err(e) => toast_helper::show_error(&format!("审核失败: {}", e)),
                    }
                });
                false
            }
            Msg::ShowExecuteModal(id) => {
                self.executing_id = Some(id);
                self.show_execute_modal = true;
                self.form_execution_type = "发货".to_string();
                self.form_execution_amount = String::new();
                self.form_related_bill_type = String::new();
                self.form_related_bill_id = String::new();
                self.form_execution_remark = String::new();
                true
            }
            Msg::HideExecuteModal => {
                self.show_execute_modal = false;
                self.executing_id = None;
                true
            }
            Msg::SubmitExecute => {
                if let Some(id) = self.executing_id {
                    let req = ExecutePurchaseContractRequest {
                        execution_type: self.form_execution_type.clone(),
                        execution_amount: self.form_execution_amount.clone(),
                        related_bill_type: if self.form_related_bill_type.is_empty() { None } else { Some(self.form_related_bill_type.clone()) },
                        related_bill_id: self.form_related_bill_id.parse::<i32>().ok(),
                        remark: if self.form_execution_remark.is_empty() { None } else { Some(self.form_execution_remark.clone()) },
                    };
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match PurchaseContractService::execute_contract(id, req).await {
                            Ok(_) => {
                                toast_helper::show_success("执行成功");
                                link.send_message(Msg::HideExecuteModal);
                                link.send_message(Msg::LoadData);
                            }
                            Err(e) => toast_helper::show_error(&format!("执行失败: {}", e)),
                        }
                    });
                }
                false
            }
            Msg::ShowCancelModal(id) => {
                self.cancelling_id = Some(id);
                self.show_cancel_modal = true;
                self.form_cancel_reason = String::new();
                true
            }
            Msg::HideCancelModal => {
                self.show_cancel_modal = false;
                self.cancelling_id = None;
                true
            }
            Msg::SubmitCancel => {
                if let Some(id) = self.cancelling_id {
                    let link = ctx.link().clone();
                    let reason = self.form_cancel_reason.clone();
                    spawn_local(async move {
                        match PurchaseContractService::cancel_contract(id, reason).await {
                            Ok(_) => {
                                toast_helper::show_success("取消成功");
                                link.send_message(Msg::HideCancelModal);
                                link.send_message(Msg::LoadData);
                            }
                            Err(e) => toast_helper::show_error(&format!("取消失败: {}", e)),
                        }
                    });
                }
                false
            }
            Msg::FormContractNoChanged(v) => { self.form_contract_no = v; true }
            Msg::FormContractNameChanged(v) => { self.form_contract_name = v; true }
            Msg::FormSupplierIdChanged(v) => { self.form_supplier_id = v; true }
            Msg::FormTotalAmountChanged(v) => { self.form_total_amount = v; true }
            Msg::FormPaymentTermsChanged(v) => { self.form_payment_terms = v; true }
            Msg::FormDeliveryDateChanged(v) => { self.form_delivery_date = v; true }
            Msg::FormRemarkChanged(v) => { self.form_remark = v; true }
            Msg::FormExecutionTypeChanged(v) => { self.form_execution_type = v; true }
            Msg::FormExecutionAmountChanged(v) => { self.form_execution_amount = v; true }
            Msg::FormRelatedBillTypeChanged(v) => { self.form_related_bill_type = v; true }
            Msg::FormRelatedBillIdChanged(v) => { self.form_related_bill_id = v; true }
            Msg::FormExecutionRemarkChanged(v) => { self.form_execution_remark = v; true }
            Msg::FormCancelReasonChanged(v) => { self.form_cancel_reason = v; true }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="purchase-contract-page">
                <PageHeader title={"采购合同管理".to_string()} subtitle={Some("管理所有采购合同信息".to_string())}>
                    <PermissionGuard resource="purchase_contract" action="create">
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                            {"+ 新建采购合同"}
                        </button>
                    </PermissionGuard>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索合同编号或名称...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载采购合同数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadData)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_contracts.is_empty() {
                    <EmptyState
                        icon={"📄".to_string()}
                        title={"暂无采购合同数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一个采购合同".to_string()
                        } else {
                            "没有匹配搜索条件的合同".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"合同编号"}</th>
                                    <th>{"合同名称"}</th>
                                    <th>{"供应商"}</th>
                                    <th>{"交货日期"}</th>
                                    <th>{"状态"}</th>
                                    <th class="numeric">{"总金额"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_contracts().iter().map(|contract| {
                                    let contract_clone = contract.clone();
                                    let id = contract.id;
                                    let status = ContractStatus::from_str(&contract.status);
                                    html! {
                                        <tr>
                                            <td>{&contract.contract_no}</td>
                                            <td>{&contract.contract_name}</td>
                                            <td>{contract.supplier_name.as_deref().unwrap_or("-")}</td>
                                            <td>{&contract.delivery_date}</td>
                                            <td>{status.display_name()}</td>
                                            <td class="numeric">{format!("{:.2}", contract.total_amount.parse::<f64>().unwrap_or(0.0))}</td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    if status == ContractStatus::Draft {
                                                        <PermissionGuard resource="purchase_contract" action="update">
                                                            <button
                                                                class="btn btn-sm btn-secondary"
                                                                onclick={link.callback(move |_| Msg::OpenEditModal(contract_clone.clone()))}
                                                            >
                                                                {"编辑"}
                                                            </button>
                                                        </PermissionGuard>
                                                        <PermissionGuard resource="purchase_contract" action="approve">
                                                            <button
                                                                class="btn btn-sm btn-success"
                                                                onclick={link.callback(move |_| Msg::ApproveContract(id))}
                                                            >
                                                                {"审核"}
                                                            </button>
                                                        </PermissionGuard>
                                                        <PermissionGuard resource="purchase_contract" action="delete">
                                                            <button
                                                                class="btn btn-sm btn-danger"
                                                                onclick={link.callback(move |_| Msg::DeleteContract(id))}
                                                            >
                                                                {"删除"}
                                                            </button>
                                                        </PermissionGuard>
                                                    }
                                                    if status == ContractStatus::Approved || status == ContractStatus::Executing {
                                                        <button
                                                            class="btn btn-sm btn-primary"
                                                            onclick={link.callback(move |_| Msg::ShowExecuteModal(id))}
                                                        >
                                                            {"执行"}
                                                        </button>
                                                        <button
                                                            class="btn btn-sm btn-warning"
                                                            onclick={link.callback(move |_| Msg::ShowCancelModal(id))}
                                                        >
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

                        <Pagination
                            current_page={self.page}
                            page_size={self.page_size}
                            total={self.filtered_contracts.len() as u64}
                            on_page_change={link.callback(|page| Msg::PageChanged(page))}
                        />
                    </div>
                }

                if self.show_modal {
                    {self.render_form_modal(ctx)}
                }

                if self.show_execute_modal {
                    {self.render_execute_modal(ctx)}
                }

                if self.show_cancel_modal {
                    {self.render_cancel_modal(ctx)}
                }

                <ConfirmDialog
                    title={"确认删除".to_string()}
                    message={"确定要删除这个采购合同吗？此操作不可撤销。".to_string()}
                    confirm_text={"删除".to_string()}
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

impl PurchaseContractPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_contracts = self.contracts.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_contracts = self.contracts.iter()
                .filter(|c| {
                    c.contract_no.to_lowercase().contains(&keyword) ||
                    c.contract_name.to_lowercase().contains(&keyword) ||
                    c.supplier_name.as_ref().map(|s| s.to_lowercase().contains(&keyword)).unwrap_or(false)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_contracts(&self) -> Vec<PurchaseContract> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_contracts[start..end.min(self.filtered_contracts.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_contract_no = String::new();
        self.form_contract_name = String::new();
        self.form_supplier_id = String::new();
        self.form_total_amount = String::new();
        self.form_payment_terms = String::new();
        self.form_delivery_date = String::new();
        self.form_remark = String::new();
        self.form_error = None;
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let is_edit = self.modal_mode == ModalMode::Edit;
        let title = if is_edit { "编辑采购合同" } else { "新建采购合同" };

        let on_contract_no_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormContractNoChanged(input.value()))
        });
        let on_contract_name_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormContractNameChanged(input.value()))
        });
        let on_supplier_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormSupplierIdChanged(input.value()))
        });
        let on_total_amount_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormTotalAmountChanged(input.value()))
        });
        let on_payment_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormPaymentTermsChanged(input.value()))
        });
        let on_delivery_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormDeliveryDateChanged(input.value()))
        });
        let on_remark_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormRemarkChanged(input.value()))
        });

        html! {
            <div class="modal-overlay" onclick={link.callback(|_| Msg::CloseModal)}>
                <div class="modal-content" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <div class="modal-header">
                        <h3>{title}</h3>
                        <button class="close-btn" onclick={link.callback(|_| Msg::CloseModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        if let Some(err) = &self.form_error {
                            <div class="form-error">{err}</div>
                        }
                        <div class="form-group">
                            <label>{"合同编号 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_contract_no.clone()}
                                oninput={on_contract_no_change}
                                placeholder="请输入合同编号"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"合同名称 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_contract_name.clone()}
                                oninput={on_contract_name_change}
                                placeholder="请输入合同名称"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"供应商ID *"}</label>
                            <input
                                type="number"
                                class="form-input"
                                value={self.form_supplier_id.clone()}
                                oninput={on_supplier_change}
                                placeholder="请输入供应商ID"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"总金额 *"}</label>
                            <input
                                type="number"
                                step="0.01"
                                class="form-input"
                                value={self.form_total_amount.clone()}
                                oninput={on_total_amount_change}
                                placeholder="请输入总金额"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"付款条款"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_payment_terms.clone()}
                                oninput={on_payment_change}
                                placeholder="请输入付款条款"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"交货日期 *"}</label>
                            <input
                                type="date"
                                class="form-input"
                                value={self.form_delivery_date.clone()}
                                oninput={on_delivery_change}
                            />
                        </div>
                        <div class="form-group">
                            <label>{"备注"}</label>
                            <textarea
                                class="form-input"
                                value={self.form_remark.clone()}
                                oninput={on_remark_change}
                                placeholder="请输入备注信息"
                                rows="3"
                            />
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={link.callback(|_| Msg::CloseModal)}>
                            {"取消"}
                        </button>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::SubmitForm)}>
                            {if is_edit { "保存修改" } else { "创建合同" }}
                        </button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_execute_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        let on_type_change = link.callback(|e: Event| {
            let select = e.target_unchecked_into::<web_sys::HtmlSelectElement>();
            Msg::FormExecutionTypeChanged(select.value())
        });
        let on_amount_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormExecutionAmountChanged(input.value()))
        });
        let on_bill_type_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormRelatedBillTypeChanged(input.value()))
        });
        let on_bill_id_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormRelatedBillIdChanged(input.value()))
        });
        let on_remark_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormExecutionRemarkChanged(input.value()))
        });

        html! {
            <div class="modal-overlay" onclick={link.callback(|_| Msg::HideExecuteModal)}>
                <div class="modal-content" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <div class="modal-header">
                        <h3>{"执行合同"}</h3>
                        <button class="close-btn" onclick={link.callback(|_| Msg::HideExecuteModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        <div class="form-group">
                            <label>{"执行类型"}</label>
                            <select class="form-input" value={self.form_execution_type.clone()} onchange={on_type_change}>
                                <option value="发货">{"发货/收货"}</option>
                                <option value="付款">{"付款/收款"}</option>
                            </select>
                        </div>
                        <div class="form-group">
                            <label>{"执行金额"}</label>
                            <input
                                type="number"
                                step="0.01"
                                class="form-input"
                                value={self.form_execution_amount.clone()}
                                oninput={on_amount_change}
                                placeholder="请输入执行金额"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"关联单据类型"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_related_bill_type.clone()}
                                oninput={on_bill_type_change}
                                placeholder="可选"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"关联单据ID"}</label>
                            <input
                                type="number"
                                class="form-input"
                                value={self.form_related_bill_id.clone()}
                                oninput={on_bill_id_change}
                                placeholder="可选"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"备注"}</label>
                            <textarea
                                class="form-input"
                                value={self.form_execution_remark.clone()}
                                oninput={on_remark_change}
                                placeholder="请输入备注"
                                rows="3"
                            />
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={link.callback(|_| Msg::HideExecuteModal)}>
                            {"取消"}
                        </button>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::SubmitExecute)}>
                            {"确认执行"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_cancel_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let on_reason_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormCancelReasonChanged(input.value()))
        });

        html! {
            <div class="modal-overlay" onclick={link.callback(|_| Msg::HideCancelModal)}>
                <div class="modal-content" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <div class="modal-header">
                        <h3>{"取消合同"}</h3>
                        <button class="close-btn" onclick={link.callback(|_| Msg::HideCancelModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        <div class="form-group">
                            <label>{"取消原因"}</label>
                            <textarea
                                class="form-input"
                                value={self.form_cancel_reason.clone()}
                                oninput={on_reason_change}
                                placeholder="请输入取消原因"
                                rows="3"
                            />
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={link.callback(|_| Msg::HideCancelModal)}>
                            {"取消"}
                        </button>
                        <button class="btn btn-danger" onclick={link.callback(|_| Msg::SubmitCancel)}>
                            {"确认取消"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}
