// 成本归集页面
// 提供成本归集数据的查询、创建和管理功能

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
use wasm_bindgen_futures::spawn_local;
use crate::models::cost_collection::{
    CostCollection, CostCollectionQuery, CreateCostCollectionRequest,
};
use crate::services::cost_collection_service::CostCollectionService;

/// 成本归集页面状态
pub struct CostCollectionPage {
    collections: Vec<CostCollection>,
    filtered_collections: Vec<CostCollection>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    show_modal: bool,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    viewing_collection: Option<CostCollection>,
    // 创建表单数据
    create_form: CreateCostCollectionForm,
    form_error: Option<String>,
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
    LoadCollections,
    CollectionsLoaded(Result<Vec<CostCollection>, String>),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    ShowCreateModal,
    CloseCreateModal,
    CloseDetail,
    ViewCollection(CostCollection),
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
    // 删除
    DeleteCollection(i32),
    ConfirmDelete,
    CancelDelete,
    Deleted,
    // 错误处理
    Error(String),
}

impl CostCollectionPage {
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
            filtered_collections: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            show_modal: false,
            show_delete_confirm: false,
            deleting_id: None,
            viewing_collection: None,
            create_form: Self::default_create_form(),
            form_error: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadCollections => {
                self.loading = true;
                self.error = None;
                let link = ctx.link().clone();
                let params = CostCollectionQuery {
                    batch_no: None,
                    color_no: None,
                    page: Some(1),
                    page_size: Some(1000),
                };
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
                        self.apply_filter();
                        self.error = None;
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
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
            Msg::ShowCreateModal => {
                self.show_modal = true;
                self.create_form = Self::default_create_form();
                self.form_error = None;
                true
            }
            Msg::CloseCreateModal => {
                self.show_modal = false;
                self.create_form = Self::default_create_form();
                self.form_error = None;
                true
            }
            Msg::CloseDetail => {
                self.viewing_collection = None;
                true
            }
            Msg::ViewCollection(collection) => {
                self.viewing_collection = Some(collection);
                true
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
                if self.create_form.direct_material.is_empty() {
                    self.form_error = Some("直接材料不能为空".to_string());
                    return true;
                }
                if self.create_form.direct_labor.is_empty() {
                    self.form_error = Some("直接人工不能为空".to_string());
                    return true;
                }
                if self.create_form.manufacturing_overhead.is_empty() {
                    self.form_error = Some("制造费用不能为空".to_string());
                    return true;
                }
                if self.create_form.processing_fee.is_empty() {
                    self.form_error = Some("加工费不能为空".to_string());
                    return true;
                }
                if self.create_form.dyeing_fee.is_empty() {
                    self.form_error = Some("染色费不能为空".to_string());
                    return true;
                }

                self.form_error = None;

                let link = ctx.link().clone();
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
                match result {
                    Ok(_) => {
                        toast_helper::show_success("成本归集创建成功");
                        self.show_modal = false;
                        self.create_form = Self::default_create_form();
                        ctx.link().send_message(Msg::LoadCollections);
                    }
                    Err(e) => {
                        self.form_error = Some(format!("创建失败: {}", e));
                    }
                }
                true
            }
            Msg::DeleteCollection(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        // 使用通用API删除
                        match crate::services::api::ApiService::delete(&format!("/cost-collections/{}", id)).await {
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
                ctx.link().send_message(Msg::LoadCollections);
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
            <div class="cost-collection-page">
                <PageHeader title={"成本归集".to_string()} subtitle={Some("按批次、色号等维度进行成本归集管理".to_string())}>
                    <button class="btn btn-primary" onclick={ctx.link().callback(|_| Msg::ShowCreateModal)}>
                        {"+ 新建成本归集"}
                    </button>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索批次号、色号或归集编号...".to_string()}
                        on_search={ctx.link().callback(|keyword| Msg::Search(keyword))}
                        on_reset={ctx.link().callback(|_| Msg::ResetSearch)}
                    />
                </div>

                {self.render_content(ctx)}

                // 详情弹窗
                if let Some(ref collection) = self.viewing_collection {
                    {self.render_detail(ctx, collection)}
                }

                // 创建弹窗
                if self.show_modal {
                    {self.render_create_modal(ctx)}
                }

                // 删除确认对话框
                <ConfirmDialog
                    title={"确认删除".to_string()}
                    message={"确定要删除这条成本归集记录吗？此操作不可撤销。".to_string()}
                    confirm_text={"删除".to_string()}
                    cancel_text={"取消".to_string()}
                    confirm_class={"btn-danger".to_string()}
                    on_confirm={ctx.link().callback(|_| Msg::ConfirmDelete)}
                    on_cancel={ctx.link().callback(|_| Msg::CancelDelete)}
                    visible={self.show_delete_confirm}
                />
            </div>
        }
    }
}

impl CostCollectionPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_collections = self.collections.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_collections = self.collections.iter()
                .filter(|c| {
                    c.collection_no.to_lowercase().contains(&keyword) ||
                    c.batch_no.as_ref().map(|s| s.to_lowercase().contains(&keyword)).unwrap_or(false) ||
                    c.color_no.as_ref().map(|s| s.to_lowercase().contains(&keyword)).unwrap_or(false) ||
                    c.workshop.as_ref().map(|s| s.to_lowercase().contains(&keyword)).unwrap_or(false)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_collections(&self) -> Vec<CostCollection> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_collections[start..end.min(self.filtered_collections.len())].to_vec()
    }

    fn render_content(&self, ctx: &Context<Self>) -> Html {
        if self.loading {
            return html! { <LoadingState message={"正在加载成本归集数据...".to_string()} /> };
        }

        if let Some(error) = &self.error {
            return html! {
                <div class="error-container">
                    <div class="error-icon">{"⚠️"}</div>
                    <p class="error-message">{error}</p>
                    <button class="btn btn-primary" onclick={ctx.link().callback(|_| Msg::LoadCollections)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        if self.filtered_collections.is_empty() {
            return html! {
                <EmptyState
                    icon={"📊".to_string()}
                    title={"暂无成本归集记录".to_string()}
                    description={if self.search_keyword.is_empty() {
                        "点击上方按钮创建第一条成本归集记录".to_string()
                    } else {
                        "没有匹配搜索条件的记录".to_string()
                    }}
                />
            };
        }

        html! {
            <div class="table-container">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>{"归集编号"}</th>
                            <th>{"归集日期"}</th>
                            <th>{"批次号"}</th>
                            <th>{"色号"}</th>
                            <th>{"车间"}</th>
                            <th class="numeric">{"直接材料"}</th>
                            <th class="numeric">{"直接人工"}</th>
                            <th class="numeric">{"制造费用"}</th>
                            <th class="numeric">{"加工费"}</th>
                            <th class="numeric">{"染色费"}</th>
                            <th class="text-center">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.paginated_collections().iter().map(|collection| {
                            let collection_clone = collection.clone();
                            let id = collection.id;
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
                                    <td class="text-center">
                                        <div class="action-buttons">
                                            <button
                                                class="btn btn-sm btn-secondary"
                                                onclick={ctx.link().callback(move |_| Msg::ViewCollection(collection_clone.clone()))}
                                            >
                                                {"详情"}
                                            </button>
                                            <button
                                                class="btn btn-sm btn-danger"
                                                onclick={ctx.link().callback(move |_| Msg::DeleteCollection(id))}
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
                    total={self.filtered_collections.len() as u64}
                    on_page_change={ctx.link().callback(|page| Msg::PageChanged(page))}
                />
            </div>
        }
    }

    fn render_detail(&self, ctx: &Context<Self>, collection: &CostCollection) -> Html {
        html! {
            <div class="modal-overlay" onclick={ctx.link().callback(|_| Msg::CloseDetail)}>
                <div class="modal-content" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <div class="modal-header">
                        <h3>{"成本归集详情"}</h3>
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
                                <span class="label">{"创建时间"}</span>
                                <span class="value">{&collection.created_at}</span>
                            </div>
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseDetail)}>
                            {"关闭"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }

    fn render_create_modal(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="modal-overlay" onclick={ctx.link().callback(|_| Msg::CloseCreateModal)}>
                <div class="modal-content modal-lg" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <div class="modal-header">
                        <h3>{"新建成本归集"}</h3>
                        <button class="close-btn" onclick={ctx.link().callback(|_| Msg::CloseCreateModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        if let Some(ref err) = self.form_error {
                            <div class="form-error">{err}</div>
                        }
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"归集日期 *"}</label>
                                <input
                                    type="date"
                                    class="form-input"
                                    value={self.create_form.collection_date.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateCollectionDate(target.value())
                                    })}
                                />
                            </div>
                            <div class="form-group">
                                <label>{"成本对象类型"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    placeholder="如：工单、批次"
                                    value={self.create_form.cost_object_type.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateCostObjectType(target.value())
                                    })}
                                />
                            </div>
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"成本对象ID"}</label>
                                <input
                                    type="number"
                                    class="form-input"
                                    placeholder="成本对象ID"
                                    value={self.create_form.cost_object_id.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateCostObjectId(target.value())
                                    })}
                                />
                            </div>
                            <div class="form-group">
                                <label>{"成本对象编号"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    placeholder="成本对象编号"
                                    value={self.create_form.cost_object_no.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateCostObjectNo(target.value())
                                    })}
                                />
                            </div>
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"批次号"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    placeholder="请输入批次号"
                                    value={self.create_form.batch_no.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateBatchNoForm(target.value())
                                    })}
                                />
                            </div>
                            <div class="form-group">
                                <label>{"色号"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    placeholder="请输入色号"
                                    value={self.create_form.color_no.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateColorNoForm(target.value())
                                    })}
                                />
                            </div>
                            <div class="form-group">
                                <label>{"车间"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    placeholder="请输入车间"
                                    value={self.create_form.workshop.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateWorkshop(target.value())
                                    })}
                                />
                            </div>
                        </div>

                        <h4>{"成本构成"}</h4>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"直接材料 *"}</label>
                                <input
                                    type="number"
                                    step="0.01"
                                    class="form-input"
                                    placeholder="0.00"
                                    value={self.create_form.direct_material.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateDirectMaterial(target.value())
                                    })}
                                />
                            </div>
                            <div class="form-group">
                                <label>{"直接人工 *"}</label>
                                <input
                                    type="number"
                                    step="0.01"
                                    class="form-input"
                                    placeholder="0.00"
                                    value={self.create_form.direct_labor.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateDirectLabor(target.value())
                                    })}
                                />
                            </div>
                            <div class="form-group">
                                <label>{"制造费用 *"}</label>
                                <input
                                    type="number"
                                    step="0.01"
                                    class="form-input"
                                    placeholder="0.00"
                                    value={self.create_form.manufacturing_overhead.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateManufacturingOverhead(target.value())
                                    })}
                                />
                            </div>
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"加工费 *"}</label>
                                <input
                                    type="number"
                                    step="0.01"
                                    class="form-input"
                                    placeholder="0.00"
                                    value={self.create_form.processing_fee.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateProcessingFee(target.value())
                                    })}
                                />
                            </div>
                            <div class="form-group">
                                <label>{"染色费 *"}</label>
                                <input
                                    type="number"
                                    step="0.01"
                                    class="form-input"
                                    placeholder="0.00"
                                    value={self.create_form.dyeing_fee.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateDyeingFee(target.value())
                                    })}
                                />
                            </div>
                        </div>

                        <h4>{"产量信息"}</h4>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"产量(米)"}</label>
                                <input
                                    type="number"
                                    step="0.01"
                                    class="form-input"
                                    placeholder="可选"
                                    value={self.create_form.output_quantity_meters.clone()}
                                    oninput={ctx.link().callback(|e: InputEvent| {
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateOutputQuantityMeters(target.value())
                                    })}
                                />
                            </div>
                            <div class="form-group">
                                <label>{"产量(公斤)"}</label>
                                <input
                                    type="number"
                                    step="0.01"
                                    class="form-input"
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
                        <button class="btn btn-secondary" onclick={ctx.link().callback(|_| Msg::CloseCreateModal)}>
                            {"取消"}
                        </button>
                        <button class="btn btn-primary" onclick={ctx.link().callback(|_| Msg::SubmitCreate)}>
                            {"创建"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }

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

    fn format_optional_decimal(&self, value: &Option<serde_json::Value>) -> String {
        match value {
            Some(v) => self.format_decimal(v),
            None => "-".to_string(),
        }
    }
}
