// 辅助核算页面
// 提供辅助核算数据的查询、统计和分析功能

use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use crate::models::assist_accounting::{
    AssistDimension, AssistRecord, AssistSummary, AssistRecordListResponse, AssistRecordQueryParams, AssistSummaryQueryParams,
};
use crate::services::assist_accounting_service::AssistAccountingService;
use crate::components::main_layout::MainLayout;

/// 辅助核算页面状态
pub struct AssistAccountingPage {
    // 维度列表
    dimensions: Vec<AssistDimension>,
    // 记录列表
    records: Vec<AssistRecord>,
    // 汇总数据
    summaries: Vec<AssistSummary>,
    // 当前选中的维度
    selected_dimension: Option<AssistDimension>,
    // 当前查看的记录详情
    selected_record: Option<AssistRecord>,
    // 加载状态
    loading: bool,
    // 查询参数
    query_params: AssistRecordQueryParams,
    // 汇总查询参数
    summary_params: AssistSummaryQueryParams,
    // 错误信息
    error: Option<String>,
    // 当前标签页
    active_tab: String,
    // 维度加载状态
    dimensions_loading: bool,
    // 汇总加载状态
    summary_loading: bool,
}

/// 页面消息
pub enum Msg {
    // 加载维度列表
    LoadDimensions,
    DimensionsLoaded(Result<Vec<AssistDimension>, String>),
    // 加载记录列表
    LoadRecords,
    RecordsLoaded(Result<AssistRecordListResponse, String>),
    // 加载汇总数据
    LoadSummary,
    SummaryLoaded(Result<Vec<AssistSummary>, String>),
    // 选择维度
    SelectDimension(AssistDimension),
    // 查看记录详情
    ViewRecord(AssistRecord),
    // 关闭详情
    CloseDetail,
    // 更新查询参数
    UpdateAccountingPeriod(String),
    UpdateDimensionCode(String),
    UpdateBusinessType(String),
    UpdateWarehouseId(String),
    // 执行查询
    QueryRecords,
    // 切换标签页
    SetActiveTab(String),
    // 错误处理
    Error(String),
}

impl AssistAccountingPage {
    // 初始化默认查询参数
    fn default_query_params() -> AssistRecordQueryParams {
        AssistRecordQueryParams {
            accounting_period: None,
            dimension_code: None,
            business_type: None,
            warehouse_id: None,
            page: Some(0),
            page_size: Some(20),
        }
    }

    // 初始化默认汇总参数
    fn default_summary_params() -> AssistSummaryQueryParams {
        let current_month = chrono::Local::now().format("%Y-%m").to_string();
        AssistSummaryQueryParams {
            accounting_period: current_month,
            dimension_code: None,
        }
    }
}

impl Component for AssistAccountingPage {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::LoadDimensions);
        ctx.link().send_message(Msg::LoadRecords);
        ctx.link().send_message(Msg::LoadSummary);
        Self {
            dimensions: Vec::new(),
            records: Vec::new(),
            summaries: Vec::new(),
            selected_dimension: None,
            selected_record: None,
            loading: true,
            query_params: Self::default_query_params(),
            summary_params: Self::default_summary_params(),
            error: None,
            active_tab: "dimensions".to_string(),
            dimensions_loading: true,
            summary_loading: true,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadDimensions => {
                self.dimensions_loading = true;
                let link = ctx.link().clone();
                spawn_local(async move {
                    let result = AssistAccountingService::list_dimensions().await;
                    link.send_message(Msg::DimensionsLoaded(result));
                });
                false
            }
            Msg::DimensionsLoaded(result) => {
                self.dimensions_loading = false;
                match result {
                    Ok(data) => {
                        self.dimensions = data;
                        self.error = None;
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
                true
            }
            Msg::LoadRecords => {
                self.loading = true;
                let link = ctx.link().clone();
                let params = self.query_params.clone();
                spawn_local(async move {
                    let result = AssistAccountingService::query_records(params).await;
                    link.send_message(Msg::RecordsLoaded(result));
                });
                false
            }
            Msg::RecordsLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(data) => {
                        self.records = data.records;
                        self.query_params.page = Some(data.page);
                        self.query_params.page_size = Some(data.page_size);
                        self.error = None;
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
                true
            }
            Msg::LoadSummary => {
                self.summary_loading = true;
                let link = ctx.link().clone();
                let params = self.summary_params.clone();
                spawn_local(async move {
                    let result = AssistAccountingService::get_summary(params).await;
                    link.send_message(Msg::SummaryLoaded(result));
                });
                false
            }
            Msg::SummaryLoaded(result) => {
                self.summary_loading = false;
                match result {
                    Ok(data) => {
                        self.summaries = data;
                        self.error = None;
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
                true
            }
            Msg::SelectDimension(dimension) => {
                self.selected_dimension = Some(dimension);
                self.query_params.dimension_code = self.selected_dimension.as_ref().map(|d| d.dimension_code.clone());
                ctx.link().send_message(Msg::LoadRecords);
                false
            }
            Msg::ViewRecord(record) => {
                self.selected_record = Some(record);
                true
            }
            Msg::CloseDetail => {
                self.selected_record = None;
                true
            }
            Msg::UpdateAccountingPeriod(period) => {
                let period_clone = period.clone();
                self.query_params.accounting_period = if period.is_empty() { None } else { Some(period) };
                self.summary_params.accounting_period = if period_clone.is_empty() { chrono::Local::now().format("%Y-%m").to_string() } else { period_clone };
                false
            }
            Msg::UpdateDimensionCode(code) => {
                let code_clone = code.clone();
                self.query_params.dimension_code = if code.is_empty() { None } else { Some(code) };
                self.summary_params.dimension_code = if code_clone.is_empty() { None } else { Some(code_clone) };
                false
            }
            Msg::UpdateBusinessType(business_type) => {
                self.query_params.business_type = if business_type.is_empty() { None } else { Some(business_type) };
                false
            }
            Msg::UpdateWarehouseId(warehouse_id) => {
                self.query_params.warehouse_id = warehouse_id.parse::<i32>().ok();
                false
            }
            Msg::QueryRecords => {
                self.loading = true;
                let link = ctx.link().clone();
                let params = self.query_params.clone();
                spawn_local(async move {
                    let result = AssistAccountingService::query_records(params).await;
                    link.send_message(Msg::RecordsLoaded(result));
                });
                false
            }
            Msg::SetActiveTab(tab) => {
                self.active_tab = tab;
                true
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
            <MainLayout current_page="assist_accounting">
                <div class="assist-accounting-page">
                    <div class="page-header">
                        <h1>{"辅助核算"}</h1>
                        <p class="subtitle">{"按批次、色号、缸号、等级、仓库等维度进行辅助核算"}</p>
                    </div>

                    {self.render_content(ctx)}
                </div>
            </MainLayout>
        }
    }
}

impl AssistAccountingPage {
    fn render_content(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                // 标签页导航
                {self.render_tabs(ctx)}

                // 维度标签页
                if self.active_tab == "dimensions" {
                    {self.render_dimensions_tab(ctx)}
                }

                // 记录标签页
                if self.active_tab == "records" {
                    {self.render_records_tab(ctx)}
                }

                // 汇总标签页
                if self.active_tab == "summary" {
                    {self.render_summary_tab(ctx)}
                }

                // 记录详情弹窗
                if let Some(ref record) = self.selected_record {
                    {self.render_record_detail(ctx, record)}
                }
            </>
        }
    }

    // 渲染标签页
    fn render_tabs(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="tabs">
                <button
                    class={if self.active_tab == "dimensions" { "tab-btn active" } else { "tab-btn" }}
                    onclick={ctx.link().callback(|_| Msg::SetActiveTab("dimensions".to_string()))}
                >
                    {"核算维度"}
                </button>
                <button
                    class={if self.active_tab == "records" { "tab-btn active" } else { "tab-btn" }}
                    onclick={ctx.link().callback(|_| Msg::SetActiveTab("records".to_string()))}
                >
                    {"核算记录"}
                </button>
                <button
                    class={if self.active_tab == "summary" { "tab-btn active" } else { "tab-btn" }}
                    onclick={ctx.link().callback(|_| Msg::SetActiveTab("summary".to_string()))}
                >
                    {"核算汇总"}
                </button>
            </div>
        }
    }

    // 渲染维度标签页
    fn render_dimensions_tab(&self, ctx: &Context<Self>) -> Html {
        if self.dimensions_loading {
            return html! {
                <div class="loading-container">
                    <div class="spinner"></div>
                    <p>{"加载中..."}</p>
                </div>
            };
        }

        html! {
            <div class="card">
                <div class="card-header">
                    <h2>{"辅助核算维度"}</h2>
                    <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::LoadDimensions)}>
                        {"刷新"}
                    </button>
                </div>
                <div class="card-body">
                    if self.dimensions.is_empty() {
                        <div class="empty-state">
                            <div class="empty-icon">{"📭"}</div>
                            <p>{"暂无核算维度"}</p>
                        </div>
                    } else {
                        <div class="dimensions-grid">
                            {for self.dimensions.iter().map(|dim| {
                                let dim_clone = dim.clone();
                                html! {
                                    <div
                                        class="dimension-card"
                                        onclick={ctx.link().callback(move |_| Msg::SelectDimension(dim_clone.clone()))}
                                    >
                                        <div class="dimension-code">{&dim.dimension_code}</div>
                                        <div class="dimension-name">{&dim.dimension_name}</div>
                                        <div class="dimension-desc">{dim.description.clone().unwrap_or_else(|| "-".to_string())}</div>
                                        <div class="dimension-status">
                                            {if dim.is_active { "启用" } else { "禁用" }}
                                        </div>
                                    </div>
                                }
                            })}
                        </div>
                    }
                </div>
            </div>
        }
    }

    // 渲染记录标签页
    fn render_records_tab(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="card">
                <div class="card-header">
                    <h2>{"辅助核算记录查询"}</h2>
                    <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::LoadRecords)}>
                        {"刷新"}
                    </button>
                </div>
                <div class="card-body">
                    // 查询条件
                    {self.render_query_form(ctx)}

                    // 记录列表
                    if self.loading {
                        <div class="loading-container">
                            <div class="spinner"></div>
                            <p>{"加载中..."}</p>
                        </div>
                    } else if let Some(error) = &self.error {
                        <div class="error-container">
                            <div class="error-icon">{"⚠️"}</div>
                            <p class="error-message">{error}</p>
                            <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadRecords)}>
                                {"重新加载"}
                            </button>
                        </div>
                    } else if self.records.is_empty() {
                        <div class="empty-state">
                            <div class="empty-icon">{"📭"}</div>
                            <p>{"暂无核算记录"}</p>
                        </div>
                    } else {
                        <div class="table-responsive">
                            <table class="data-table">
                                <thead>
                                    <tr>
                                        <th>{"ID"}</th>
                                        <th>{"业务类型"}</th>
                                        <th>{"业务单号"}</th>
                                        <th>{"五维ID"}</th>
                                        <th>{"批次"}</th>
                                        <th>{"色号"}</th>
                                        <th>{"等级"}</th>
                                        <th>{"借方金额"}</th>
                                        <th>{"贷方金额"}</th>
                                        <th>{"操作"}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {for self.records.iter().map(|record| {
                                        let record_clone = record.clone();
                                        html! {
                                            <tr>
                                                <td>{record.id.to_string()}</td>
                                                <td>{&record.business_type}</td>
                                                <td>{&record.business_no}</td>
                                                <td class="five-dim-id">{&record.five_dimension_id}</td>
                                                <td>{&record.batch_no}</td>
                                                <td>{&record.color_no}</td>
                                                <td>{&record.grade}</td>
                                                <td class="numeric">{self.format_decimal(&record.debit_amount)}</td>
                                                <td class="numeric">{self.format_decimal(&record.credit_amount)}</td>
                                                <td>
                                                    <button
                                                        class="btn-sm btn-info"
                                                        onclick={ctx.link().callback(move |_| Msg::ViewRecord(record_clone.clone()))}
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

    // 渲染查询表单
    fn render_query_form(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="query-form">
                <div class="form-row">
                    <div class="form-group">
                        <label for="accounting-period">{"会计期间"}</label>
                        <input
                            id="accounting-period"
                            type="month"
                            class="form-control"
                            value={self.query_params.accounting_period.clone().unwrap_or_default()}
                            oninput={ctx.link().callback(|e: InputEvent| {
                                let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                Msg::UpdateAccountingPeriod(target.value())
                            })}
                        />
                    </div>
                    <div class="form-group">
                        <label for="business-type">{"业务类型"}</label>
                        <input
                            id="business-type"
                            type="text"
                            class="form-control"
                            placeholder="如：采购、销售"
                            value={self.query_params.business_type.clone().unwrap_or_default()}
                            oninput={ctx.link().callback(|e: InputEvent| {
                                let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                Msg::UpdateBusinessType(target.value())
                            })}
                        />
                    </div>
                    <div class="form-group">
                        <label for="warehouse-id">{"仓库ID"}</label>
                        <input
                            id="warehouse-id"
                            type="number"
                            class="form-control"
                            placeholder="仓库ID"
                            value={self.query_params.warehouse_id.map(|id| id.to_string()).unwrap_or_default()}
                            oninput={ctx.link().callback(|e: InputEvent| {
                                let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                Msg::UpdateWarehouseId(target.value())
                            })}
                        />
                    </div>
                </div>
                <div class="form-actions">
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::QueryRecords)}>
                        {"查询"}
                    </button>
                </div>
            </div>
        }
    }

    // 渲染汇总标签页
    fn render_summary_tab(&self, ctx: &Context<Self>) -> Html {
        if self.summary_loading {
            return html! {
                <div class="loading-container">
                    <div class="spinner"></div>
                    <p>{"加载中..."}</p>
                </div>
            };
        }

        html! {
            <div class="card">
                <div class="card-header">
                    <h2>{"辅助核算汇总"}</h2>
                    <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::LoadSummary)}>
                        {"刷新"}
                    </button>
                </div>
                <div class="card-body">
                    // 汇总期间选择
                    <div class="summary-header">
                        <div class="form-group">
                            <label for="summary-period">{"会计期间"}</label>
                            <input
                                id="summary-period"
                                type="month"
                                class="form-control"
                                value={self.summary_params.accounting_period.clone()}
                                oninput={ctx.link().callback(|e: InputEvent| {
                                    let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                    Msg::UpdateAccountingPeriod(target.value())
                                })}
                            />
                        </div>
                        <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadSummary)}>
                            {"查询汇总"}
                        </button>
                    </div>

                    if self.summaries.is_empty() {
                        <div class="empty-state">
                            <div class="empty-icon">{"📭"}</div>
                            <p>{"暂无汇总数据"}</p>
                        </div>
                    } else {
                        <div class="table-responsive">
                            <table class="data-table">
                                <thead>
                                    <tr>
                                        <th>{"ID"}</th>
                                        <th>{"会计期间"}</th>
                                        <th>{"维度代码"}</th>
                                        <th>{"维度值"}</th>
                                        <th>{"会计科目ID"}</th>
                                        <th>{"借方合计"}</th>
                                        <th>{"贷方合计"}</th>
                                        <th>{"总米数"}</th>
                                        <th>{"总公斤数"}</th>
                                        <th>{"记录数"}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {for self.summaries.iter().map(|summary| {
                                        html! {
                                            <tr>
                                                <td>{summary.id.to_string()}</td>
                                                <td>{&summary.accounting_period}</td>
                                                <td>{&summary.dimension_code}</td>
                                                <td>{&summary.dimension_value_name}</td>
                                                <td>{summary.account_subject_id.to_string()}</td>
                                                <td class="numeric">{self.format_decimal(&summary.total_debit)}</td>
                                                <td class="numeric">{self.format_decimal(&summary.total_credit)}</td>
                                                <td class="numeric">{self.format_decimal(&summary.total_quantity_meters)}</td>
                                                <td class="numeric">{self.format_decimal(&summary.total_quantity_kg)}</td>
                                                <td class="numeric">{summary.record_count.to_string()}</td>
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

    // 渲染记录详情弹窗
    fn render_record_detail(&self, ctx: &Context<Self>, record: &AssistRecord) -> Html {
        html! {
            <div class="modal-overlay" onclick={ctx.link().callback(|_| Msg::CloseDetail)}>
                <div class="modal-content" onclick={|_| {}}>
                    <div class="modal-header">
                        <h2>{"辅助核算记录详情"}</h2>
                        <button class="close-btn" onclick={ctx.link().callback(|_| Msg::CloseDetail)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        <div class="detail-grid">
                            <div class="detail-item">
                                <span class="label">{"ID"}</span>
                                <span class="value">{record.id.to_string()}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"业务类型"}</span>
                                <span class="value">{&record.business_type}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"业务单号"}</span>
                                <span class="value">{&record.business_no}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"业务ID"}</span>
                                <span class="value">{record.business_id.to_string()}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"会计科目ID"}</span>
                                <span class="value">{record.account_subject_id.to_string()}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"五维ID"}</span>
                                <span class="value five-dim-id">{&record.five_dimension_id}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"产品ID"}</span>
                                <span class="value">{record.product_id.to_string()}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"批次号"}</span>
                                <span class="value">{&record.batch_no}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"色号"}</span>
                                <span class="value">{&record.color_no}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"染缸号"}</span>
                                <span class="value">{record.dye_lot_no.clone().unwrap_or_else(|| "-".to_string())}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"等级"}</span>
                                <span class="value">{&record.grade}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"仓库ID"}</span>
                                <span class="value">{record.warehouse_id.to_string()}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"借方金额"}</span>
                                <span class="value numeric">{self.format_decimal(&record.debit_amount)}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"贷方金额"}</span>
                                <span class="value numeric">{self.format_decimal(&record.credit_amount)}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"数量(米)"}</span>
                                <span class="value numeric">{self.format_decimal(&record.quantity_meters)}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"数量(公斤)"}</span>
                                <span class="value numeric">{self.format_decimal(&record.quantity_kg)}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"车间ID"}</span>
                                <span class="value">{record.workshop_id.map(|id| id.to_string()).unwrap_or_else(|| "-".to_string())}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"客户ID"}</span>
                                <span class="value">{record.customer_id.map(|id| id.to_string()).unwrap_or_else(|| "-".to_string())}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"供应商ID"}</span>
                                <span class="value">{record.supplier_id.map(|id| id.to_string()).unwrap_or_else(|| "-".to_string())}</span>
                            </div>
                            <div class="detail-item full-width">
                                <span class="label">{"备注"}</span>
                                <span class="value">{record.remarks.clone().unwrap_or_else(|| "-".to_string())}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"创建时间"}</span>
                                <span class="value">{&record.created_at}</span>
                            </div>
                        </div>
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
        } else {
            value.to_string()
        }
    }
}