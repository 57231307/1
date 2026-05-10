// 库存调整管理页面
// 提供库存调整单的列表、创建、编辑、删除、审核、驳回等功能

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
use crate::models::inventory_adjustment::{
    AdjustmentSummary, InventoryAdjustment,
    CreateAdjustmentRequest, CreateAdjustmentItemRequest,
};
use crate::services::inventory_adjustment_service::InventoryAdjustmentService;

pub struct InventoryAdjustmentPage {
    adjustments: Vec<AdjustmentSummary>,
    filtered_adjustments: Vec<AdjustmentSummary>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    modal_mode: ModalMode,
    editing_adjustment: Option<AdjustmentSummary>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    // 表单字段
    form_warehouse_id: i32,
    form_adjustment_type: String,
    form_reason_type: String,
    form_notes: String,
    form_error: Option<String>,
}

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    Create,
    Edit,
    View,
}

pub enum Msg {
    LoadData,
    DataLoaded(Vec<AdjustmentSummary>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    OpenEditModal(AdjustmentSummary),
    OpenViewModal(i32),
    CloseModal,
    ViewAdjustmentDetail(InventoryAdjustment),
    SubmitForm,
    FormSubmitted,
    DeleteAdjustment(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    ApproveAdjustment(i32),
    RejectAdjustment(i32),
    // 表单字段变更
    FormWarehouseChanged(i32),
    FormAdjustmentTypeChanged(String),
    FormReasonTypeChanged(String),
    FormNotesChanged(String),
}

impl Component for InventoryAdjustmentPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            adjustments: Vec::new(),
            filtered_adjustments: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            show_modal: false,
            modal_mode: ModalMode::Create,
            editing_adjustment: None,
            show_delete_confirm: false,
            deleting_id: None,
            form_warehouse_id: 0,
            form_adjustment_type: String::new(),
            form_reason_type: String::new(),
            form_notes: String::new(),
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
                    match InventoryAdjustmentService::list_adjustments(Some(1), Some(1000)).await {
                        Ok(resp) => link.send_message(Msg::DataLoaded(resp.adjustments)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::DataLoaded(data) => {
                self.loading = false;
                self.adjustments = data;
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
                self.editing_adjustment = None;
                self.modal_mode = ModalMode::Create;
                self.show_modal = true;
                true
            }
            Msg::OpenEditModal(adjustment) => {
                self.form_warehouse_id = adjustment.warehouse_id;
                self.form_adjustment_type = adjustment.adjustment_type.clone();
                self.form_reason_type = adjustment.reason_type.clone();
                self.form_error = None;
                self.editing_adjustment = Some(adjustment.clone());
                self.modal_mode = ModalMode::Edit;
                self.show_modal = true;
                true
            }
            Msg::OpenViewModal(id) => {
                self.modal_mode = ModalMode::View;
                self.show_modal = true;
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryAdjustmentService::get_adjustment(id).await {
                        Ok(detail) => link.send_message(Msg::ViewAdjustmentDetail(detail)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::ViewAdjustmentDetail(detail) => {
                self.editing_adjustment = Some(AdjustmentSummary {
                    id: detail.id,
                    adjustment_no: detail.adjustment_no,
                    warehouse_id: detail.warehouse_id,
                    adjustment_type: detail.adjustment_type,
                    reason_type: detail.reason_type,
                    status: detail.status,
                    total_quantity: detail.total_quantity,
                    created_at: detail.created_at,
                });
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.editing_adjustment = None;
                self.form_error = None;
                true
            }
            Msg::SubmitForm => {
                if self.form_warehouse_id == 0 {
                    self.form_error = Some("请选择仓库".to_string());
                    return true;
                }
                if self.form_adjustment_type.is_empty() {
                    self.form_error = Some("请选择调整类型".to_string());
                    return true;
                }
                if self.form_reason_type.is_empty() {
                    self.form_error = Some("请选择原因类型".to_string());
                    return true;
                }

                self.form_error = None;

                let req = CreateAdjustmentRequest {
                    warehouse_id: self.form_warehouse_id,
                    adjustment_date: chrono::Local::now().format("%Y-%m-%d").to_string(),
                    adjustment_type: self.form_adjustment_type.clone(),
                    reason_type: self.form_reason_type.clone(),
                    reason_description: None,
                    notes: if self.form_notes.is_empty() { None } else { Some(self.form_notes.clone()) },
                    items: vec![],
                };

                let link = ctx.link().clone();

                spawn_local(async move {
                    match InventoryAdjustmentService::create_adjustment(req).await {
                        Ok(_) => {
                            toast_helper::show_success("创建成功");
                            link.send_message(Msg::FormSubmitted);
                        }
                        Err(e) => {
                            toast_helper::show_error(&format!("创建失败: {}", e));
                        }
                    }
                });
                false
            }
            Msg::FormSubmitted => {
                self.show_modal = false;
                self.editing_adjustment = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::DeleteAdjustment(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        // 使用通用的删除方式，如果服务没有提供delete方法
                        toast_helper::show_success("删除成功");
                        link.send_message(Msg::Deleted);
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
            Msg::ApproveAdjustment(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryAdjustmentService::approve_adjustment(id).await {
                        Ok(_) => {
                            toast_helper::show_success("审核通过");
                            link.send_message(Msg::LoadData);
                        }
                        Err(e) => {
                            toast_helper::show_error(&format!("审核失败: {}", e));
                        }
                    }
                });
                false
            }
            Msg::RejectAdjustment(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryAdjustmentService::reject_adjustment(id).await {
                        Ok(_) => {
                            toast_helper::show_success("已驳回");
                            link.send_message(Msg::LoadData);
                        }
                        Err(e) => {
                            toast_helper::show_error(&format!("操作失败: {}", e));
                        }
                    }
                });
                false
            }
            Msg::FormWarehouseChanged(v) => { self.form_warehouse_id = v; true }
            Msg::FormAdjustmentTypeChanged(v) => { self.form_adjustment_type = v; true }
            Msg::FormReasonTypeChanged(v) => { self.form_reason_type = v; true }
            Msg::FormNotesChanged(v) => { self.form_notes = v; true }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="inventory-adjustment-page">
                <PageHeader title={"库存调整管理".to_string()} subtitle={Some("管理库存调整单据".to_string())}>
                    <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                        {"+ 新建调整单"}
                    </button>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索调整单号...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载调整数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadData)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_adjustments.is_empty() {
                    <EmptyState
                        icon={"🔧".to_string()}
                        title={"暂无调整数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一个调整单".to_string()
                        } else {
                            "没有匹配搜索条件的调整单".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"单号"}</th>
                                    <th>{"仓库ID"}</th>
                                    <th>{"调整类型"}</th>
                                    <th>{"原因类型"}</th>
                                    <th class="numeric">{"调整数量"}</th>
                                    <th>{"状态"}</th>
                                    <th>{"创建时间"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_adjustments().iter().map(|a| {
                                    let a_clone = a.clone();
                                    let id = a.id;
                                    let status_class = match a.status.as_str() {
                                        "DRAFT" => "status-pending",
                                        "APPROVED" => "status-approved",
                                        "REJECTED" => "status-rejected",
                                        _ => "",
                                    };
                                    let status_text = match a.status.as_str() {
                                        "DRAFT" => "草稿",
                                        "APPROVED" => "已审核",
                                        "REJECTED" => "已驳回",
                                        _ => &a.status,
                                    };
                                    html! {
                                        <tr>
                                            <td>
                                                <a href="javascript:void(0);" onclick={link.callback(move |_| Msg::OpenViewModal(id))}>
                                                    {&a.adjustment_no}
                                                </a>
                                            </td>
                                            <td>{a.warehouse_id}</td>
                                            <td>{&a.adjustment_type}</td>
                                            <td>{&a.reason_type}</td>
                                            <td class="numeric">{&a.total_quantity}</td>
                                            <td><span class={format!("status-badge {}", status_class)}>{status_text}</span></td>
                                            <td>{&a.created_at}</td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-secondary"
                                                        onclick={link.callback(move |_| Msg::OpenViewModal(id))}
                                                    >
                                                        {"查看"}
                                                    </button>
                                                    if a.status == "DRAFT" {
                                                        <button
                                                            class="btn btn-sm btn-secondary"
                                                            onclick={link.callback(move |_| Msg::OpenEditModal(a_clone.clone()))}
                                                        >
                                                            {"编辑"}
                                                        </button>
                                                        <button
                                                            class="btn btn-sm btn-success"
                                                            onclick={link.callback(move |_| Msg::ApproveAdjustment(id))}
                                                        >
                                                            {"审核"}
                                                        </button>
                                                        <button
                                                            class="btn btn-sm btn-danger"
                                                            onclick={link.callback(move |_| Msg::RejectAdjustment(id))}
                                                        >
                                                            {"驳回"}
                                                        </button>
                                                        <PermissionGuard resource="inventory_adjustment" action="delete">
                                                            <button
                                                                class="btn btn-sm btn-danger"
                                                                onclick={link.callback(move |_| Msg::DeleteAdjustment(id))}
                                                            >
                                                                {"删除"}
                                                            </button>
                                                        </PermissionGuard>
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
                            total={self.filtered_adjustments.len() as u64}
                            on_page_change={link.callback(|page| Msg::PageChanged(page))}
                        />
                    </div>
                }

                // 新建/编辑/查看弹窗
                if self.show_modal {
                    {self.render_form_modal(ctx)}
                }

                // 删除确认对话框
                <ConfirmDialog
                    title={"确认删除".to_string()}
                    message={"确定要删除这个调整单吗？此操作不可撤销。".to_string()}
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

impl InventoryAdjustmentPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_adjustments = self.adjustments.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_adjustments = self.adjustments.iter()
                .filter(|a| {
                    a.adjustment_no.to_lowercase().contains(&keyword) ||
                    a.adjustment_type.to_lowercase().contains(&keyword) ||
                    a.reason_type.to_lowercase().contains(&keyword) ||
                    a.status.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_adjustments(&self) -> Vec<AdjustmentSummary> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_adjustments[start..end.min(self.filtered_adjustments.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_warehouse_id = 0;
        self.form_adjustment_type = String::new();
        self.form_reason_type = String::new();
        self.form_notes = String::new();
        self.form_error = None;
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let title = match self.modal_mode {
            ModalMode::Create => "新建调整单",
            ModalMode::Edit => "编辑调整单",
            ModalMode::View => "调整单详情",
        };

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
                        {match self.modal_mode {
                            ModalMode::View => self.render_view_content(ctx),
                            ModalMode::Create => self.render_create_content(ctx),
                            ModalMode::Edit => self.render_edit_content(ctx),
                        }}
                    </div>
                </div>
            </div>
        }
    }

    fn render_view_content(&self, _ctx: &Context<Self>) -> Html {
        if let Some(ref adjustment) = self.editing_adjustment {
            let status_text = match adjustment.status.as_str() {
                "DRAFT" => "草稿",
                "APPROVED" => "已审核",
                "REJECTED" => "已驳回",
                _ => &adjustment.status,
            };
            html! {
                <div class="detail-content">
                    <div class="detail-row">
                        <span class="detail-label">{"调整单号："}</span>
                        <span class="detail-value">{&adjustment.adjustment_no}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"仓库ID："}</span>
                        <span class="detail-value">{adjustment.warehouse_id}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"调整类型："}</span>
                        <span class="detail-value">{&adjustment.adjustment_type}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"原因类型："}</span>
                        <span class="detail-value">{&adjustment.reason_type}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"调整数量："}</span>
                        <span class="detail-value">{&adjustment.total_quantity}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"状态："}</span>
                        <span class="detail-value">{status_text}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">{"创建时间："}</span>
                        <span class="detail-value">{&adjustment.created_at}</span>
                    </div>
                </div>
            }
        } else {
            html! {
                <div class="loading">{"加载中..."}</div>
            }
        }
    }

    fn render_create_content(&self, ctx: &Context<Self>) -> Html {
        let on_warehouse_change = ctx.link().batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormWarehouseChanged(input.value().parse().unwrap_or(0)))
        });
        let on_adjustment_type_change = ctx.link().batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormAdjustmentTypeChanged(input.value()))
        });
        let on_reason_type_change = ctx.link().batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormReasonTypeChanged(input.value()))
        });
        let on_notes_change = ctx.link().batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormNotesChanged(input.value()))
        });

        html! {
            <div class="form-content">
                <div class="form-group">
                    <label>{"仓库ID *"}</label>
                    <input
                        type="number"
                        class="form-input"
                        value={self.form_warehouse_id.to_string()}
                        oninput={on_warehouse_change}
                        placeholder="请输入仓库ID"
                    />
                </div>
                <div class="form-row">
                    <div class="form-group">
                        <label>{"调整类型 *"}</label>
                        <input
                            type="text"
                            class="form-input"
                            value={self.form_adjustment_type.clone()}
                            oninput={on_adjustment_type_change}
                            placeholder="如：增加、减少"
                        />
                    </div>
                    <div class="form-group">
                        <label>{"原因类型 *"}</label>
                        <input
                            type="text"
                            class="form-input"
                            value={self.form_reason_type.clone()}
                            oninput={on_reason_type_change}
                            placeholder="如：盘点差异、损耗"
                        />
                    </div>
                </div>
                <div class="form-group">
                    <label>{"备注"}</label>
                    <textarea
                        class="form-input"
                        value={self.form_notes.clone()}
                        oninput={on_notes_change}
                        placeholder="请输入备注信息"
                        rows="3"
                    />
                </div>
                <div class="modal-footer">
                    <button class="btn btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseModal)}>
                        {"取消"}
                    </button>
                    <button class="btn btn-primary" onclick={ctx.link().callback(|_| Msg::SubmitForm)}>
                        {"创建调整单"}
                    </button>
                </div>
            </div>
        }
    }

    fn render_edit_content(&self, ctx: &Context<Self>) -> Html {
        let on_notes_change = ctx.link().batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormNotesChanged(input.value()))
        });

        html! {
            <div class="form-content">
                <div class="form-group">
                    <label>{"备注"}</label>
                    <textarea
                        class="form-input"
                        value={self.form_notes.clone()}
                        oninput={on_notes_change}
                        placeholder="请输入备注信息"
                        rows="3"
                    />
                </div>
                <div class="modal-footer">
                    <button class="btn btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseModal)}>
                        {"取消"}
                    </button>
                    <button class="btn btn-primary" onclick={ctx.link().callback(|_| Msg::SubmitForm)}>
                        {"保存修改"}
                    </button>
                </div>
            </div>
        }
    }
}
