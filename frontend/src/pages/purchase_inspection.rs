//! 采购检验页面

use yew::prelude::*;
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
    /// 单个检验单加载成功
    InspectionLoaded(PurchaseInspection),
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
                    match PurchaseInspectionService::list(query).await {
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
            Msg::InspectionLoaded(inspection) => {
                self.selected_inspection = Some(inspection);
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
                self.modal_mode = ModalMode::View;
                self.show_modal = true;
                self.loading = true;
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseInspectionService::get(id).await {
                        Ok(inspection) => {
                            link.send_message(Msg::InspectionLoaded(inspection));
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                true
            }
            Msg::OpenCreateModal => {
                self.modal_mode = ModalMode::Create;
                self.selected_inspection = None;
                self.show_modal = true;
                true
            }
            Msg::OpenCompleteModal(id) => {
                self.modal_mode = ModalMode::Complete;
                self.show_modal = true;
                self.loading = true;
                let link = ctx.link().clone();
                spawn_local(async move {
                    match PurchaseInspectionService::get(id).await {
                        Ok(inspection) => {
                            link.send_message(Msg::InspectionLoaded(inspection));
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
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
                web_sys::window().unwrap().alert_with_message(&msg).ok();
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_status_change = ctx.link().callback(|e: Event| {
            let target = e.target().unwrap().dyn_into::<web_sys::HtmlSelectElement>().unwrap();
            Msg::SetFilterStatus(target.value())
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
}