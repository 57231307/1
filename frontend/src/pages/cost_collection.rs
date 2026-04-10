// 成本归集页面
// 提供成本归集数据的查询、创建和管理功能

use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use crate::models::cost_collection::{
    CostCollection, CostCollectionQuery, CreateCostCollectionRequest,
};
use crate::services::cost_collection_service::CostCollectionService;
use crate::components::main_layout::MainLayout;

/// 成本归集页面状态
pub struct CostCollectionPage {
    // 成本归集列表
    collections: Vec<CostCollection>,
    // 当前选中的成本归集
    selected_collection: Option<CostCollection>,
    // 加载状态
    loading: bool,
    // 查询参数
    query_params: CostCollectionQuery,
    // 错误信息
    error: Option<String>,
    // 是否显示创建弹窗
    show_create_modal: bool,
    // 创建表单数据
    create_form: CreateCostCollectionForm,
    // 创建提交状态
    creating: bool,
    // 创建成功消息
    create_success: Option<String>,
}

/// 创建表单数据
#[derive(Debug, Clone, Default)]
struct CreateCostCollectionForm {
    collection_date: String,
    cost_object_type: String,
    cost_object_id: String,
    cost_object_no: String,
    batch_no: String,
    color_no: String,
    workshop: String,
    direct_material: String,
    direct_labor: String,
    manufacturing_overhead: String,
    processing_fee: String,
    dyeing_fee: String,
    output_quantity_meters: String,
    output_quantity_kg: String,
}

/// 页面消息
pub enum Msg {
    // 加载成本归集列表
    LoadCollections,
    CollectionsLoaded(Result<Vec<CostCollection>, String>),
    // 查看详情
    ViewCollection(CostCollection),
    // 关闭详情
    CloseDetail,
    // 更新查询参数
    UpdateBatchNo(String),
    UpdateColorNo(String),
    // 执行查询
    QueryCollections,
    // 显示创建弹窗
    ShowCreateModal,
    // 关闭创建弹窗
    CloseCreateModal,
    // 更新创建表单
    UpdateCollectionDate(String),
    UpdateCostObjectType(String),
    UpdateCostObjectId(String),
    UpdateCostObjectNo(String),
    UpdateBatchNoForm(String),
    UpdateColorNoForm(String),
    UpdateWorkshop(String),
    UpdateDirectMaterial(String),
    UpdateDirectLabor(String),
    UpdateManufacturingOverhead(String),
    UpdateProcessingFee(String),
    UpdateDyeingFee(String),
    UpdateOutputQuantityMeters(String),
    UpdateOutputQuantityKg(String),
    // 提交创建
    SubmitCreate,
    CreateSuccess(Result<CostCollection, String>),
    // 清除成功消息
    ClearSuccess,
    // 错误处理
    Error(String),
}

impl CostCollectionPage {
    // 初始化默认查询参数
    fn default_query_params() -> CostCollectionQuery {
        CostCollectionQuery {
            batch_no: None,
            color_no: None,
            page: Some(1),
            page_size: Some(20),
        }
    }

    // 初始化默认创建表单
    fn default_create_form() -> CreateCostCollectionForm {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        CreateCostCollectionForm {
            collection_date: today,
            ..Default::default()
        }
    }
}

impl Component for CostCollectionPage {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::LoadCollections);
        Self {
            collections: Vec::new(),
            selected_collection: None,
            loading: true,
            query_params: Self::default_query_params(),
            error: None,
            show_create_modal: false,
            create_form: Self::default_create_form(),
            creating: false,
            create_success: None,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadCollections => {
                self.loading = true;
                let link = _ctx.link().clone();
                let params = self.query_params.clone();
                spawn_local(async move {
                    let result = CostCollectionService::list(params).await;
                    link.send_message(Msg::CollectionsLoaded(result));
                });
                false
            }
            Msg::CollectionsLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(data) => {
                        self.collections = data;
                        self.error = None;
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
                true
            }
            Msg::ViewCollection(collection) => {
                self.selected_collection = Some(collection);
                true
            }
            Msg::CloseDetail => {
                self.selected_collection = None;
                true
            }
            Msg::UpdateBatchNo(batch_no) => {
                self.query_params.batch_no = if batch_no.is_empty() { None } else { Some(batch_no) };
                false
            }
            Msg::UpdateColorNo(color_no) => {
                self.query_params.color_no = if color_no.is_empty() { None } else { Some(color_no) };
                false
            }
            Msg::QueryCollections => {
                self.loading = true;
                let link = _ctx.link().clone();
                let params = self.query_params.clone();
                spawn_local(async move {
                    let result = CostCollectionService::list(params).await;
                    link.send_message(Msg::CollectionsLoaded(result));
                });
                false
            }
            Msg::ShowCreateModal => {
                self.show_create_modal = true;
                self.create_form = Self::default_create_form();
                self.create_success = None;
                true
            }
            Msg::CloseCreateModal => {
                self.show_create_modal = false;
                self.create_form = Self::default_create_form();
                false
            }
            Msg::UpdateCollectionDate(date) => {
                self.create_form.collection_date = date;
                false
            }
            Msg::UpdateCostObjectType(value) => {
                self.create_form.cost_object_type = value;
                false
            }
            Msg::UpdateCostObjectId(value) => {
                self.create_form.cost_object_id = value;
                false
            }
            Msg::UpdateCostObjectNo(value) => {
                self.create_form.cost_object_no = value;
                false
            }
            Msg::UpdateBatchNoForm(value) => {
                self.create_form.batch_no = value;
                false
            }
            Msg::UpdateColorNoForm(value) => {
                self.create_form.color_no = value;
                false
            }
            Msg::UpdateWorkshop(value) => {
                self.create_form.workshop = value;
                false
            }
            Msg::UpdateDirectMaterial(value) => {
                self.create_form.direct_material = value;
                false
            }
            Msg::UpdateDirectLabor(value) => {
                self.create_form.direct_labor = value;
                false
            }
            Msg::UpdateManufacturingOverhead(value) => {
                self.create_form.manufacturing_overhead = value;
                false
            }
            Msg::UpdateProcessingFee(value) => {
                self.create_form.processing_fee = value;
                false
            }
            Msg::UpdateDyeingFee(value) => {
                self.create_form.dyeing_fee = value;
                false
            }
            Msg::UpdateOutputQuantityMeters(value) => {
                self.create_form.output_quantity_meters = value;
                false
            }
            Msg::UpdateOutputQuantityKg(value) => {
                self.create_form.output_quantity_kg = value;
                false
            }
            Msg::SubmitCreate => {
                self.creating = true;
                let link = _ctx.link().clone();
                let req = CreateCostCollectionRequest {
                    collection_date: self.create_form.collection_date.clone(),
                    cost_object_type: if self.create_form.cost_object_type.is_empty() { None } else { Some(self.create_form.cost_object_type.clone()) },
                    cost_object_id: self.create_form.cost_object_id.parse().ok(),
                    cost_object_no: if self.create_form.cost_object_no.is_empty() { None } else { Some(self.create_form.cost_object_no.clone()) },
                    batch_no: if self.create_form.batch_no.is_empty() { None } else { Some(self.create_form.batch_no.clone()) },
                    color_no: if self.create_form.color_no.is_empty() { None } else { Some(self.create_form.color_no.clone()) },
                    workshop: if self.create_form.workshop.is_empty() { None } else { Some(self.create_form.workshop.clone()) },
                    direct_material: self.create_form.direct_material.clone(),
                    direct_labor: self.create_form.direct_labor.clone(),
                    manufacturing_overhead: self.create_form.manufacturing_overhead.clone(),
                    processing_fee: self.create_form.processing_fee.clone(),
                    dyeing_fee: self.create_form.dyeing_fee.clone(),
                    output_quantity_meters: if self.create_form.output_quantity_meters.is_empty() { None } else { Some(self.create_form.output_quantity_meters.clone()) },
                    output_quantity_kg: if self.create_form.output_quantity_kg.is_empty() { None } else { Some(self.create_form.output_quantity_kg.clone()) },
                };
                spawn_local(async move {
                    let result = CostCollectionService::create(req).await;
                    link.send_message(Msg::CreateSuccess(result));
                });
                false
            }
            Msg::CreateSuccess(result) => {
                self.creating = false;
                match result {
                    Ok(collection) => {
                        self.create_success = Some(format!("成本归集创建成功：{}", collection.collection_no));
                        self.collections.insert(0, collection);
                        // 3秒后关闭弹窗
                        let link = _ctx.link().clone();
                        spawn_local(async move {
                            gloo_timers::future::TimeoutFuture::new(3000).await;
                            link.send_message(Msg::CloseCreateModal);
                            link.send_message(Msg::ClearSuccess);
                        });
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
                true
            }
            Msg::ClearSuccess => {
                self.create_success = None;
                false
            }
            Msg::Error(e) => {
                self.error = Some(e);
                self.loading = false;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <MainLayout current_page="cost_collection">
                <div class="cost-collection-page">
                    <div class="page-header">
                        <h1>{"成本归集"}</h1>
                        <p class="subtitle">{"按批次、色号等维度进行成本归集管理"}</p>
                    </div>

                    {self.render_content(ctx)}
                </div>
            </MainLayout>
        }
    }
}

impl CostCollectionPage {
    fn render_content(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                // 操作栏
                {self.render_actions(ctx)}

                // 错误提示
                if let Some(error) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{error}</p>
                        <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadCollections)}>
                            {"重新加载"}
                        </button>
                    </div>
                }

                // 成本归集列表
                {self.render_list(ctx)}

                // 详情弹窗
                if let Some(ref collection) = self.selected_collection {
                    {self.render_detail(ctx, collection)}
                }

                // 创建弹窗
                if self.show_create_modal {
                    {self.render_create_modal(ctx)}
                }
            </>
        }
    }

    // 渲染操作栏
    fn render_actions(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="actions-bar">
                <div class="actions-left">
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::ShowCreateModal)}>
                        {"新建成本归集"}
                    </button>
                </div>
                <div class="actions-right">
                    <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::LoadCollections)}>
                        {"刷新"}
                    </button>
                </div>
            </div>
        }
    }

    // 渲染查询表单
    fn render_query_form(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="query-form">
                <div class="form-row">
                    <div class="form-group">
                        <label for="batch-no">{"批次号"}</label>
                        <input
                            id="batch-no"
                            type="text"
                            class="form-control"
                            placeholder="请输入批次号"
                            value={self.query_params.batch_no.clone().unwrap_or_default()}
                            oninput={ctx.link().callback(|e: InputEvent| {
                                let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                Msg::UpdateBatchNo(target.value())
                            })}
                        />
                    </div>
                    <div class="form-group">
                        <label for="color-no">{"色号"}</label>
                        <input
                            id="color-no"
                            type="text"
                            class="form-control"
                            placeholder="请输入色号"
                            value={self.query_params.color_no.clone().unwrap_or_default()}
                            oninput={ctx.link().callback(|e: InputEvent| {
                                let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                Msg::UpdateColorNo(target.value())
                            })}
                        />
                    </div>
                </div>
                <div class="form-actions">
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::QueryCollections)}>
                        {"查询"}
                    </button>
                </div>
            </div>
        }
    }

    // 渲染列表
    fn render_list(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="card">
                <div class="card-header">
                    <h2>{"成本归集列表"}</h2>
                </div>
                <div class="card-body">
                    {self.render_query_form(ctx)}

                    if self.loading {
                        <div class="loading-container">
                            <div class="spinner"></div>
                            <p>{"加载中..."}</p>
                        </div>
                    } else if self.collections.is_empty() {
                        <div class="empty-state">
                            <div class="empty-icon">{"📭"}</div>
                            <p>{"暂无成本归集记录"}</p>
                        </div>
                    } else {
                        <div class="table-responsive">
                            <table class="data-table">
                                <thead>
                                    <tr>
                                        <th>{"归集编号"}</th>
                                        <th>{"归集日期"}</th>
                                        <th>{"批次号"}</th>
                                        <th>{"色号"}</th>
                                        <th>{"车间"}</th>
                                        <th>{"直接材料"}</th>
                                        <th>{"直接人工"}</th>
                                        <th>{"制造费用"}</th>
                                        <th>{"加工费"}</th>
                                        <th>{"染色费"}</th>
                                        <th>{"操作"}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {for self.collections.iter().map(|collection| {
                                        let collection_clone = collection.clone();
                                        html! {
                                            <tr>
                                                <td>{&collection.collection_no}</td>
                                                <td>{&collection.collection_date}</td>
                                                <td>{collection.batch_no.clone().unwrap_or_else(|| "-".to_string())}</td>
                                                <td>{collection.color_no.clone().unwrap_or_else(|| "-".to_string())}</td>
                                                <td>{collection.workshop.clone().unwrap_or_else(|| "-".to_string())}</td>
                                                <td class="numeric">{self.format_decimal(&collection.direct_material)}</td>
                                                <td class="numeric">{self.format_decimal(&collection.direct_labor)}</td>
                                                <td class="numeric">{self.format_decimal(&collection.manufacturing_overhead)}</td>
                                                <td class="numeric">{self.format_decimal(&collection.processing_fee)}</td>
                                                <td class="numeric">{self.format_decimal(&collection.dyeing_fee)}</td>
                                                <td>
                                                    <button
                                                        class="btn-sm btn-info"
                                                        onclick={ctx.link().callback(move |_| Msg::ViewCollection(collection_clone.clone()))}
                                                    >
                                                        {"详情"}
                                                    </button>
                                                </td>
                                            </tr>
                                        }
                                    })}
                                </tbody>
                            </table>
                        </div>
                    }
                </div>
            </div>
        }
    }

    // 渲染详情弹窗
    fn render_detail(&self, ctx: &Context<Self>, collection: &CostCollection) -> Html {
        html! {
            <div class="modal-overlay" onclick={ctx.link().callback(|_| Msg::CloseDetail)}>
                <div class="modal-content" onclick={|_| {}}>
                    <div class="modal-header">
                        <h2>{"成本归集详情"}</h2>
                        <button class="close-btn" onclick={ctx.link().callback(|_| Msg::CloseDetail)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        <div class="detail-grid">
                            <div class="detail-item">
                                <span class="label">{"归集编号"}</span>
                                <span class="value">{&collection.collection_no}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"归集日期"}</span>
                                <span class="value">{&collection.collection_date}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"成本对象类型"}</span>
                                <span class="value">{collection.cost_object_type.clone().unwrap_or_else(|| "-".to_string())}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"成本对象ID"}</span>
                                <span class="value">{collection.cost_object_id.map(|id| id.to_string()).unwrap_or_else(|| "-".to_string())}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"成本对象编号"}</span>
                                <span class="value">{collection.cost_object_no.clone().unwrap_or_else(|| "-".to_string())}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"批次号"}</span>
                                <span class="value">{collection.batch_no.clone().unwrap_or_else(|| "-".to_string())}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"色号"}</span>
                                <span class="value">{collection.color_no.clone().unwrap_or_else(|| "-".to_string())}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"车间"}</span>
                                <span class="value">{collection.workshop.clone().unwrap_or_else(|| "-".to_string())}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"直接材料"}</span>
                                <span class="value numeric">{self.format_decimal(&collection.direct_material)}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"直接人工"}</span>
                                <span class="value numeric">{self.format_decimal(&collection.direct_labor)}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"制造费用"}</span>
                                <span class="value numeric">{self.format_decimal(&collection.manufacturing_overhead)}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"加工费"}</span>
                                <span class="value numeric">{self.format_decimal(&collection.processing_fee)}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"染色费"}</span>
                                <span class="value numeric">{self.format_decimal(&collection.dyeing_fee)}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"产量(米)"}</span>
                                <span class="value">{self.format_optional_decimal(&collection.output_quantity_meters)}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"产量(公斤)"}</span>
                                <span class="value">{self.format_optional_decimal(&collection.output_quantity_kg)}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"创建人ID"}</span>
                                <span class="value">{collection.created_by.to_string()}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"创建时间"}</span>
                                <span class="value">{&collection.created_at}</span>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        }
    }

    // 渲染创建弹窗
    fn render_create_modal(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="modal-overlay" onclick={ctx.link().callback(|_| Msg::CloseCreateModal)}>
                <div class="modal-content modal-lg" onclick={|_| {}}>
                    <div class="modal-header">
                        <h2>{"新建成本归集"}</h2>
                        <button class="close-btn" onclick={ctx.link().callback(|_| Msg::CloseCreateModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        if let Some(ref success) = self.create_success {
                            <div class="success-message">
                                <span>{"✓ "}</span>{success}
                            </div>
                        }

                        <div class="form-grid">
                            <div class="form-group">
                                <label for="collection-date">{"归集日期"}</label>
                                <input
                                    id="collection-date"
                                    type="date"
                                    class="form-control"
                                    value={self.create_form.collection_date.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateCollectionDate(target.value())
                                    })}
                                />
                            </div>
                            <div class="form-group">
                                <label for="cost-object-type">{"成本对象类型"}</label>
                                <input
                                    id="cost-object-type"
                                    type="text"
                                    class="form-control"
                                    placeholder="如：工单、批次"
                                    value={self.create_form.cost_object_type.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateCostObjectType(target.value())
                                    })}
                                />
                            </div>
                            <div class="form-group">
                                <label for="cost-object-id">{"成本对象ID"}</label>
                                <input
                                    id="cost-object-id"
                                    type="number"
                                    class="form-control"
                                    placeholder="成本对象ID"
                                    value={self.create_form.cost_object_id.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateCostObjectId(target.value())
                                    })}
                                />
                            </div>
                            <div class="form-group">
                                <label for="cost-object-no">{"成本对象编号"}</label>
                                <input
                                    id="cost-object-no"
                                    type="text"
                                    class="form-control"
                                    placeholder="成本对象编号"
                                    value={self.create_form.cost_object_no.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateCostObjectNo(target.value())
                                    })}
                                />
                            </div>
                            <div class="form-group">
                                <label for="batch-no-form">{"批次号"}</label>
                                <input
                                    id="batch-no-form"
                                    type="text"
                                    class="form-control"
                                    placeholder="请输入批次号"
                                    value={self.create_form.batch_no.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateBatchNoForm(target.value())
                                    })}
                                />
                            </div>
                            <div class="form-group">
                                <label for="color-no-form">{"色号"}</label>
                                <input
                                    id="color-no-form"
                                    type="text"
                                    class="form-control"
                                    placeholder="请输入色号"
                                    value={self.create_form.color_no.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateColorNoForm(target.value())
                                    })}
                                />
                            </div>
                            <div class="form-group">
                                <label for="workshop">{"车间"}</label>
                                <input
                                    id="workshop"
                                    type="text"
                                    class="form-control"
                                    placeholder="请输入车间"
                                    value={self.create_form.workshop.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateWorkshop(target.value())
                                    })}
                                />
                            </div>
                        </div>

                        <h3>{"成本构成"}</h3>
                        <div class="form-grid">
                            <div class="form-group">
                                <label for="direct-material">{"直接材料"}</label>
                                <input
                                    id="direct-material"
                                    type="number"
                                    step="0.01"
                                    class="form-control"
                                    placeholder="0.00"
                                    value={self.create_form.direct_material.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateDirectMaterial(target.value())
                                    })}
                                />
                            </div>
                            <div class="form-group">
                                <label for="direct-labor">{"直接人工"}</label>
                                <input
                                    id="direct-labor"
                                    type="number"
                                    step="0.01"
                                    class="form-control"
                                    placeholder="0.00"
                                    value={self.create_form.direct_labor.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateDirectLabor(target.value())
                                    })}
                                />
                            </div>
                            <div class="form-group">
                                <label for="manufacturing-overhead">{"制造费用"}</label>
                                <input
                                    id="manufacturing-overhead"
                                    type="number"
                                    step="0.01"
                                    class="form-control"
                                    placeholder="0.00"
                                    value={self.create_form.manufacturing_overhead.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateManufacturingOverhead(target.value())
                                    })}
                                />
                            </div>
                            <div class="form-group">
                                <label for="processing-fee">{"加工费"}</label>
                                <input
                                    id="processing-fee"
                                    type="number"
                                    step="0.01"
                                    class="form-control"
                                    placeholder="0.00"
                                    value={self.create_form.processing_fee.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateProcessingFee(target.value())
                                    })}
                                />
                            </div>
                            <div class="form-group">
                                <label for="dyeing-fee">{"染色费"}</label>
                                <input
                                    id="dyeing-fee"
                                    type="number"
                                    step="0.01"
                                    class="form-control"
                                    placeholder="0.00"
                                    value={self.create_form.dyeing_fee.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateDyeingFee(target.value())
                                    })}
                                />
                            </div>
                        </div>

                        <h3>{"产量信息"}</h3>
                        <div class="form-grid">
                            <div class="form-group">
                                <label for="output-quantity-meters">{"产量(米)"}</label>
                                <input
                                    id="output-quantity-meters"
                                    type="number"
                                    step="0.01"
                                    class="form-control"
                                    placeholder="可选"
                                    value={self.create_form.output_quantity_meters.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateOutputQuantityMeters(target.value())
                                    })}
                                />
                            </div>
                            <div class="form-group">
                                <label for="output-quantity-kg">{"产量(公斤)"}</label>
                                <input
                                    id="output-quantity-kg"
                                    type="number"
                                    step="0.01"
                                    class="form-control"
                                    placeholder="可选"
                                    value={self.create_form.output_quantity_kg.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateOutputQuantityKg(target.value())
                                    })}
                                />
                            </div>
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseCreateModal)}>
                            {"取消"}
                        </button>
                        <button
                            class="btn-primary"
                            onclick={ctx.link().callback(|_| Msg::SubmitCreate)}
                            disabled={self.creating}
                        >
                            {if self.creating { "提交中..." } else { "创建" }}
                        </button>
                    </div>
                </div>
            </div>
        }
    }

    // 格式化数值
    fn format_decimal(&self, value: &serde_json::Value) -> String {
        if let Some(num) = value.as_f64() {
            format!("{:.2}", num)
        } else if let Some(num) = value.as_i64() {
            num.to_string()
        } else if let Some(num) = value.as_str() {
            num.to_string()
        } else {
            value.to_string()
        }
    }

    // 格式化可选数值
    fn format_optional_decimal(&self, value: &Option<serde_json::Value>) -> String {
        match value {
            Some(v) => self.format_decimal(v),
            None => "-".to_string(),
        }
    }
}