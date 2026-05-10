// 凭证管理页面

use crate::utils::toast_helper;
use yew::prelude::*;
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
use crate::services::voucher_service::VoucherService;
use crate::services::crud_service::CrudService;
use crate::models::voucher::{Voucher, VoucherQueryParams, CreateVoucherRequest};

pub struct VoucherPage {
    vouchers: Vec<Voucher>,
    filtered_vouchers: Vec<Voucher>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_voucher: Option<Voucher>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    // 表单字段
    form_type: String,
    form_date: String,
    form_batch_no: String,
    form_color_no: String,
    form_error: Option<String>,
}

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    Create,
    Edit,
}

pub enum Msg {
    LoadData,
    DataLoaded(Vec<Voucher>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    OpenEditModal(Voucher),
    CloseModal,
    SubmitForm,
    FormSubmitted,
    DeleteVoucher(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    SubmitVoucher(i32),
    ReviewVoucher(i32),
    PostVoucher(i32),
    // 表单字段变更
    FormTypeChanged(String),
    FormDateChanged(String),
    FormBatchNoChanged(String),
    FormColorNoChanged(String),
}

impl Component for VoucherPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            vouchers: Vec::new(),
            filtered_vouchers: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            show_modal: false,
            modal_mode: ModalMode::Create,
            editing_voucher: None,
            show_delete_confirm: false,
            deleting_id: None,
            form_type: String::new(),
            form_date: chrono::Local::now().format("%Y-%m-%d").to_string(),
            form_batch_no: String::new(),
            form_color_no: String::new(),
            form_error: None,
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
                    let params = VoucherQueryParams {
                        voucher_type: None,
                        status: None,
                        start_date: None,
                        end_date: None,
                        batch_no: None,
                        color_no: None,
                        page: Some(1),
                        page_size: Some(1000),
                    };
                    match VoucherService::list_vouchers(params).await {
                        Ok(res) => link.send_message(Msg::DataLoaded(res.data)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::DataLoaded(data) => {
                self.loading = false;
                self.vouchers = data;
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
                self.editing_voucher = None;
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(voucher) => {
                self.form_type = voucher.voucher_type.clone();
                self.form_date = voucher.voucher_date.clone();
                self.form_batch_no = voucher.batch_no.clone().unwrap_or_default();
                self.form_color_no = voucher.color_no.clone().unwrap_or_default();
                self.form_error = None;
                self.editing_voucher = Some(voucher);
                self.modal_mode = ModalMode::Edit;
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.editing_voucher = None;
                self.form_error = None;
                true
            }
            Msg::SubmitForm => {
                if self.form_type.is_empty() {
                    self.form_error = Some("凭证类型不能为空".to_string());
                    return true;
                }
                if self.form_date.is_empty() {
                    self.form_error = Some("凭证日期不能为空".to_string());
                    return true;
                }

                self.form_error = None;

                let req = CreateVoucherRequest {
                    voucher_type: self.form_type.clone(),
                    voucher_date: self.form_date.clone(),
                    source_type: None,
                    source_module: None,
                    source_bill_id: None,
                    source_bill_no: None,
                    batch_no: if self.form_batch_no.is_empty() { None } else { Some(self.form_batch_no.clone()) },
                    color_no: if self.form_color_no.is_empty() { None } else { Some(self.form_color_no.clone()) },
                    items: vec![],
                };

                let link = ctx.link().clone();

                if self.modal_mode == ModalMode::Edit {
                    if let Some(voucher) = &self.editing_voucher {
                        let id = voucher.id;
                        spawn_local(async move {
                            match VoucherService::create_voucher(req).await {
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
                        match VoucherService::create_voucher(req).await {
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
                self.editing_voucher = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::DeleteVoucher(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match crate::services::api::ApiService::delete(&format!("/vouchers/{}", id)).await {
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
            Msg::SubmitVoucher(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match VoucherService::submit_voucher(id).await {
                        Ok(_) => {
                            toast_helper::show_success("提交成功");
                            link.send_message(Msg::LoadData);
                        }
                        Err(e) => {
                            toast_helper::show_error(&format!("提交失败: {}", e));
                        }
                    }
                });
                false
            }
            Msg::ReviewVoucher(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match VoucherService::review_voucher(id).await {
                        Ok(_) => {
                            toast_helper::show_success("审核成功");
                            link.send_message(Msg::LoadData);
                        }
                        Err(e) => {
                            toast_helper::show_error(&format!("审核失败: {}", e));
                        }
                    }
                });
                false
            }
            Msg::PostVoucher(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match VoucherService::post_voucher(id).await {
                        Ok(_) => {
                            toast_helper::show_success("过账成功");
                            link.send_message(Msg::LoadData);
                        }
                        Err(e) => {
                            toast_helper::show_error(&format!("过账失败: {}", e));
                        }
                    }
                });
                false
            }
            Msg::FormTypeChanged(v) => { self.form_type = v; true }
            Msg::FormDateChanged(v) => { self.form_date = v; true }
            Msg::FormBatchNoChanged(v) => { self.form_batch_no = v; true }
            Msg::FormColorNoChanged(v) => { self.form_color_no = v; true }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="voucher-page">
                <PageHeader title={"凭证管理".to_string()} subtitle={Some("管理财务凭证信息".to_string())}>
                    <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                        {"+ 新建凭证"}
                    </button>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索凭证号、类型或制单人...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载凭证数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadData)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_vouchers.is_empty() {
                    <EmptyState
                        icon={"📝".to_string()}
                        title={"暂无凭证数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一张凭证".to_string()
                        } else {
                            "没有匹配搜索条件的凭证".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"凭证号"}</th>
                                    <th>{"类型"}</th>
                                    <th>{"日期"}</th>
                                    <th class="numeric">{"总借方"}</th>
                                    <th class="numeric">{"总贷方"}</th>
                                    <th>{"状态"}</th>
                                    <th>{"制单人"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_vouchers().iter().map(|v| {
                                    let v_clone = v.clone();
                                    let id = v.id;
                                    let status = v.status.clone();
                                    let status_class = match status.as_str() {
                                        "DRAFT" => "status-badge status-draft",
                                        "SUBMITTED" => "status-badge status-warning",
                                        "REVIEWED" => "status-badge status-primary",
                                        "POSTED" => "status-badge status-success",
                                        _ => "status-badge",
                                    };
                                    let status_text = match status.as_str() {
                                        "DRAFT" => "草稿",
                                        "SUBMITTED" => "已提交",
                                        "REVIEWED" => "已审核",
                                        "POSTED" => "已过账",
                                        _ => &status,
                                    };
                                    html! {
                                        <tr>
                                            <td>{&v.voucher_no}</td>
                                            <td>{&v.voucher_type}</td>
                                            <td>{&v.voucher_date}</td>
                                            <td class="numeric">{&v.total_debit}</td>
                                            <td class="numeric">{&v.total_credit}</td>
                                            <td>
                                                <span class={status_class}>{status_text}</span>
                                            </td>
                                            <td>{v.creator_name.clone().unwrap_or_else(|| "-".to_string())}</td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    if status == "DRAFT" {
                                                        <button
                                                            class="btn btn-sm btn-secondary"
                                                            onclick={link.callback(move |_| Msg::OpenEditModal(v_clone.clone()))}
                                                        >
                                                            {"编辑"}
                                                        </button>
                                                        <button
                                                            class="btn btn-sm btn-primary"
                                                            onclick={link.callback(move |_| Msg::SubmitVoucher(id))}
                                                        >
                                                            {"提交"}
                                                        </button>
                                                    }
                                                    if status == "SUBMITTED" {
                                                        <button
                                                            class="btn btn-sm btn-primary"
                                                            onclick={link.callback(move |_| Msg::ReviewVoucher(id))}
                                                        >
                                                            {"审核"}
                                                        </button>
                                                    }
                                                    if status == "REVIEWED" {
                                                        <button
                                                            class="btn btn-sm btn-success"
                                                            onclick={link.callback(move |_| Msg::PostVoucher(id))}
                                                        >
                                                            {"过账"}
                                                        </button>
                                                    }
                                                    <button
                                                        class="btn btn-sm btn-danger"
                                                        onclick={link.callback(move |_| Msg::DeleteVoucher(id))}
                                                    >
                                                        {"删除"}
                                                    </button>
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
                            total={self.filtered_vouchers.len() as u64}
                            on_page_change={link.callback(|page| Msg::PageChanged(page))}
                        />
                    </div>
                }

                // 新建/编辑弹窗
                if self.show_modal {
                    {self.render_form_modal(ctx)}
                }

                // 删除确认对话框
                <ConfirmDialog
                    title={"确认删除".to_string()}
                    message={"确定要删除这张凭证吗？此操作不可撤销。".to_string()}
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

impl VoucherPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_vouchers = self.vouchers.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_vouchers = self.vouchers.iter()
                .filter(|v| {
                    v.voucher_no.to_lowercase().contains(&keyword) ||
                    v.voucher_type.to_lowercase().contains(&keyword) ||
                    v.creator_name.as_ref().map(|s| s.to_lowercase().contains(&keyword)).unwrap_or(false) ||
                    v.status.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_vouchers(&self) -> Vec<Voucher> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_vouchers[start..end.min(self.filtered_vouchers.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_type = String::new();
        self.form_date = chrono::Local::now().format("%Y-%m-%d").to_string();
        self.form_batch_no = String::new();
        self.form_color_no = String::new();
        self.form_error = None;
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let is_edit = self.modal_mode == ModalMode::Edit;
        let title = if is_edit { "编辑凭证" } else { "新建凭证" };

        let on_type_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormTypeChanged(input.value()))
        });
        let on_date_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormDateChanged(input.value()))
        });
        let on_batch_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormBatchNoChanged(input.value()))
        });
        let on_color_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormColorNoChanged(input.value()))
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
                            <label>{"凭证类型 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_type.clone()}
                                oninput={on_type_change}
                                placeholder="如：记账、收款、付款"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"凭证日期 *"}</label>
                            <input
                                type="date"
                                class="form-input"
                                value={self.form_date.clone()}
                                oninput={on_date_change}
                            />
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"批次号"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_batch_no.clone()}
                                    oninput={on_batch_change}
                                    placeholder="请输入批次号"
                                />
                            </div>
                            <div class="form-group">
                                <label>{"色号"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_color_no.clone()}
                                    oninput={on_color_change}
                                    placeholder="请输入色号"
                                />
                            </div>
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={link.callback(|_| Msg::CloseModal)}>
                            {"取消"}
                        </button>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::SubmitForm)}>
                            {if is_edit { "保存修改" } else { "创建凭证" }}
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}
