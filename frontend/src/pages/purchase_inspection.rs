// 采购检验页面

use crate::utils::permissions;
use yew::prelude::*;
use crate::components::permission_guard::PermissionGuard;
use crate::utils::dom_helper;
use crate::services::crud_service::CrudService;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::models::purchase_inspection::{
    PurchaseInspection, PurchaseInspectionQuery, CreatePurchaseInspectionRequest, CompleteInspectionRequest,
};
use crate::services::purchase_inspection_service::PurchaseInspectionService;

/// 采购检验页面组件
pub struct PurchaseInspectionPage {
    /// 检验单列表
    inspections: Vec<PurchaseInspection>,
    /// 加载状态
    loading: bool,
    /// 错误信息
    error: Option<String>,
    /// 当前页码
    page: u64,
    /// 每页数量
    page_size: u64,
    /// 总记录数
    total: u64,
    /// 筛选状态
    filter_status: String,
    /// 模态框模式
    modal_mode: ModalMode,
    /// 选中的检验单
    selected_inspection: Option<PurchaseInspection>,
    /// 是否显示模态框
    show_modal: bool,
}

/// 模态框模式
#[derive(Clone, PartialEq)]
pub enum ModalMode {
    /// 查看模式
    View,
    /// 创建模式
    Create,
    /// 完成检验模式
    Complete,
}

/// 消息类型
pub enum Msg {
    /// 加载检验单列表
    LoadInspections,
    /// 检验单列表加载成功
    InspectionsLoaded(Vec<PurchaseInspection>),
    /// 加载错误
    LoadError(String),
    /// 设置筛选状态
    SetFilterStatus(String),
    /// 查看检验单详情
    ViewInspection(i32),
    /// 打开创建模态框
    OpenCreateModal,
    /// 打开完成检验模态框
    OpenCompleteModal(i32),
    /// 关闭模态框
    CloseModal,
    ShowModalWithData(ModalMode, PurchaseInspection),
    /// 创建检验单
    CreateInspection(CreatePurchaseInspectionRequest),
    /// 完成检验
    CompleteInspection(CompleteInspectionRequest),
    /// 操作成功
    OperationSuccess(String),
}

impl Component for PurchaseInspectionPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            inspections: Vec::new(),
            loading: true,
            error: None,
            page: 1,
            page_size: 20,
            total: 0,
            filter_status: String::from("全部"),
            modal_mode: ModalMode::View,
            selected_inspection: None,
            show_modal: false,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadInspections);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadInspections => {
                self.loading = true;
                let query = PurchaseInspectionQuery {
                    page: Some(self.page),
                    page_size: Some(self.page_size),
                    status: if self.filter_status == "全部" { None } else { Some(self.filter_status.clone()) },
                    supplier_id: None,
                };
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseInspectionService::list_with_query(&query).await {
                        Ok(inspections) => link.send_message(Msg::InspectionsLoaded(inspections.items)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::InspectionsLoaded(inspections) => {
                self.inspections = inspections;
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
                ctx.link().send_message(Msg::LoadInspections);
                false
            }
            Msg::ViewInspection(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseInspectionService::get(id).await {
                        Ok(_inspection) => {
                            link.send_message(Msg::OpenCompleteModal(id));
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::OpenCreateModal => {
                self.modal_mode = ModalMode::Create;
                self.selected_inspection = None;
                self.show_modal = true;
                true
            }
            Msg::OpenCompleteModal(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseInspectionService::get(id).await {
                        Ok(inspection) => {
                            link.send_message(Msg::ShowModalWithData(ModalMode::Complete, inspection));
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                self.modal_mode = ModalMode::Complete;
                self.show_modal = true;
                false
            }

            Msg::ShowModalWithData(mode, inspection) => {
                self.modal_mode = mode;
                self.selected_inspection = Some(inspection);
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                self.selected_inspection = None;
                true
            }
            Msg::CreateInspection(req) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseInspectionService::create(req).await {
                        Ok(_) => {
                            link.send_message(Msg::CloseModal);
                            link.send_message(Msg::OperationSuccess("创建成功".to_string()));
                            link.send_message(Msg::LoadInspections);
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }
            Msg::CompleteInspection(req) => {
                let link = ctx.link().clone();
                if let Some(inspection) = &self.selected_inspection {
                    let id = inspection.id;
                    spawn_local(async move {
                        match PurchaseInspectionService::complete(id, req).await {
                            Ok(_) => {
                                link.send_message(Msg::CloseModal);
                                link.send_message(Msg::OperationSuccess("检验完成".to_string()));
                                link.send_message(Msg::LoadInspections);
                            }
                            Err(e) => link.send_message(Msg::LoadError(e)),
                        }
                    });
                }
                false
            }
            Msg::OperationSuccess(msg) => {
                if let Some(win) = web_sys::window() { win.alert_with_message(&msg).ok(); }
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_status_change = ctx.link().batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlSelectElement>().ok()?;
            Some(Msg::SetFilterStatus(target.value()))
        });

        let on_create_click = ctx.link().callback(|_| Msg::OpenCreateModal);

        html! {
            <div class="purchase-inspection-page">
                <div class="page-header">
                    <h1>{"采购检验"}</h1>
                    <button class="btn-primary" onclick={on_create_click}>
                        {"新建检验单"}
                    </button>
                </div>

                <div class="filter-bar">
                    <div class="filter-item">
                        <label>{"检验状态："}</label>
                        <select value={self.filter_status.clone()} onchange={on_status_change}>
                            <option value="全部">{"全部"}</option>
                            <option value="PENDING">{"待检验"}</option>
                            <option value="PASSED">{"合格"}</option>
                            <option value="FAILED">{"不合格"}</option>
                        </select>
                    </div>
                </div>

                {self.render_content(ctx)}
                
                if self.show_modal {
                    {self.render_modal(ctx)}
                }
            </div>
        }
    }
}

impl PurchaseInspectionPage {
    /// 渲染页面内容
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
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadInspections)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        if self.inspections.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"🔍"}</div>
                    <p>{"暂无采购检验单"}</p>
                </div>
            };
        }

        html! {
            <div class="table-responsive">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>{"检验单号"}</th>
                            <th>{"供应商"}</th>
                            <th>{"检验日期"}</th>
                            <th>{"检验结果"}</th>
                            <th>{"合格数量"}</th>
                            <th>{"不合格数量"}</th>
                            <th>{"备注"}</th>
                            <th>{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.inspections.iter().map(|inspection| {
                            let inspection_id = inspection.id;
                            let result_text = match inspection.result.as_str() {
                                "PENDING" => "待检验",
                                "PASSED" => "合格",
                                "FAILED" => "不合格",
                                _ => &inspection.result,
                            };
                            let result_class = match inspection.result.as_str() {
                                "PENDING" => "status-pending",
                                "PASSED" => "status-passed",
                                "FAILED" => "status-failed",
                                _ => "",
                            };
                            html! {
                                <tr>
                                    <td>{&inspection.inspection_no}</td>
                                    <td>{inspection.supplier_name.as_deref().unwrap_or("-")}</td>
                                    <td>{&inspection.inspection_date}</td>
                                    <td class={result_class}>{result_text}</td>
                                    <td class="numeric">{&inspection.qualified_quantity}</td>
                                    <td class="numeric">{&inspection.unqualified_quantity}</td>
                                    <td>{inspection.remarks.as_deref().unwrap_or("-")}</td>
                                    <td>
                                        <button class="btn-link" onclick={ctx.link().callback(move |_| Msg::ViewInspection(inspection_id))}>
                                            {"查看"}
                                        </button>
                                        if inspection.result == "PENDING" {
                                            <button class="btn-link" onclick={ctx.link().callback(move |_| Msg::OpenCompleteModal(inspection_id))}>
                                                {"完成检验"}
                                            </button>
                                        }
                                    </td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
            </div>
        }
    }

    /// 渲染模态框
    fn render_modal(&self, ctx: &Context<PurchaseInspectionPage>) -> Html {
        let on_close = ctx.link().callback(|_| Msg::CloseModal);
        
        match self.modal_mode {
            ModalMode::Create => html! {
                <div class="modal-overlay">
                    <div class="modal-content">
                        <div class="modal-header">
                            <h2>{"新建采购检验单"}</h2>
                            <button class="close-btn" onclick={on_close.clone()}>{"×"}</button>
                        </div>
                        <div class="modal-body">
                            <div class="form-group">
                                <label>{"入库单 ID"}</label>
                                <input type="number" id="create-receipt-id" />
                            </div>
                            <div class="form-group">
                                <label>{"供应商 ID"}</label>
                                <input type="number" id="create-supplier-id" />
                            </div>
                            <div class="form-group">
                                <label>{"检验日期"}</label>
                                <input type="date" id="create-date" />
                            </div>
                        </div>
                        <div class="modal-footer">
                            <button class="btn-secondary" onclick={on_close.clone()}>{"取消"}</button>
                            <PermissionGuard resource="purchase_inspection" action="create">
<button class="btn-primary" onclick={ctx.link().callback(|_| {
                                let receipt_id = dom_helper::get_numeric_value("create-receipt-id").unwrap_or(0.0) as i32;
                                let supplier_id = dom_helper::get_numeric_value("create-supplier-id").unwrap_or(0.0) as i32;
                                let date = dom_helper::get_input_value("create-date").unwrap_or_default();
                                
                                Msg::CreateInspection(CreatePurchaseInspectionRequest {
                                    receipt_id,
                                    order_id: None,
                                    supplier_id,
                                    inspection_date: if date.is_empty() { "2023-01-01".to_string() } else { date },
                                    inspector_id: None,
                                    inspection_type: None,
                                    notes: None,
                                })
                            })}>{"保存"}</button>
</PermissionGuard>
                        </div>
                    </div>
                </div>
            },
            ModalMode::Complete => {
                let id = self.selected_inspection.as_ref().map(|i| i.id).unwrap_or(0);
                html! {
                    <div class="modal-overlay">
                        <div class="modal-content">
                            <div class="modal-header">
                                <h2>{"完成检验"}</h2>
                                <button class="close-btn" onclick={on_close.clone()}>{"×"}</button>
                            </div>
                            <div class="modal-body">
                                <div class="form-group">
                                    <label>{"合格数量"}</label>
                                    <input type="number" id="complete-pass-qty" />
                                </div>
                                <div class="form-group">
                                    <label>{"不合格数量"}</label>
                                    <input type="number" id="complete-reject-qty" />
                                </div>
                                <div class="form-group">
                                    <label>{"检验结果"}</label>
                                    <select id="complete-result">
                                        <option value="PASSED">{"合格"}</option>
                                        <option value="FAILED">{"不合格"}</option>
                                    </select>
                                </div>
                            </div>
                            <div class="modal-footer">
                                <button class="btn-secondary" onclick={on_close.clone()}>{"取消"}</button>
                                <button class="btn-primary" onclick={ctx.link().callback(move |_| {
                                    let pass_qty = dom_helper::get_input_value("complete-pass-qty").unwrap_or_default();
                                    let reject_qty = dom_helper::get_input_value("complete-reject-qty").unwrap_or_default();
                                    let result = dom_helper::get_select_value("complete-result").unwrap_or_default();
                                    
                                    Msg::CompleteInspection(CompleteInspectionRequest {
                                        pass_quantity: if pass_qty.is_empty() { "0".to_string() } else { pass_qty },
                                        reject_quantity: if reject_qty.is_empty() { "0".to_string() } else { reject_qty },
                                        inspection_result: result,
                                    })
                                })}>{"确认完成"}</button>
                            </div>
                        </div>
                    </div>
                }
            },
            ModalMode::View => {
                let inspection = self.selected_inspection.as_ref().unwrap();
                html! {
                    <div class="modal-overlay">
                        <div class="modal-content">
                            <div class="modal-header">
                                <h2>{"检验单详情"}</h2>
                                <button class="close-btn" onclick={on_close.clone()}>{"×"}</button>
                            </div>
                            <div class="modal-body">
                                <p><strong>{"检验单号: "}</strong>{&inspection.inspection_no}</p>
                                <p><strong>{"状态: "}</strong>{&inspection.result}</p>
                                <p><strong>{"合格数量: "}</strong>{&inspection.qualified_quantity}</p>
                                <p><strong>{"不合格数量: "}</strong>{&inspection.unqualified_quantity}</p>
                                <p><strong>{"备注: "}</strong>{inspection.remarks.as_deref().unwrap_or("-")}</p>
                            </div>
                            <div class="modal-footer">
                                <button class="btn-primary" onclick={on_close}>{"关闭"}</button>
                            </div>
                        </div>
                    </div>
                }
            }
        }
    }
}
