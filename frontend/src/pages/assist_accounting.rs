// 辅助核算页面
// 提供辅助核算记录的查询、统计和分析功能

use crate::utils::toast_helper;
use yew::prelude::*;
use crate::components::{
    search_bar::SearchBar,
    pagination::Pagination,
    page_header::PageHeader,
    empty_state::EmptyState,
    loading_state::LoadingState,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::models::assist_accounting::{
    AssistRecord, AssistRecordListResponse, AssistRecordQueryParams,
    AssistSummary, AssistSummaryQueryParams, AssistDimension,
};
use crate::services::assist_accounting_service::AssistAccountingService;

/// 辅助核算页面状态
pub struct AssistAccountingPage {
    records: Vec<AssistRecord>,
    filtered_records: Vec<AssistRecord>,
    dimensions: Vec<AssistDimension>,
    summaries: Vec<AssistSummary>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
    filter_dimension: String,
    filter_business_type: String,
    viewing_record: Option<AssistRecord>,
    active_tab: Tab,
}

#[derive(Clone, PartialEq)]
pub enum Tab {
    Records,
    Summary,
}

/// 页面消息
pub enum Msg {
    LoadData,
    DataLoaded(Result<AssistRecordListResponse, String>),
    DimensionsLoaded(Result<Vec<AssistDimension>, String>),
    SummaryLoaded(Result<Vec<AssistSummary>, String>),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    SetFilterDimension(String),
    SetFilterBusinessType(String),
    CloseDetail,
    ViewRecord(AssistRecord),
    SwitchTab(Tab),
    Refresh,
}

impl Component for AssistAccountingPage {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::LoadData);
        Self {
            records: Vec::new(),
            filtered_records: Vec::new(),
            dimensions: Vec::new(),
            summaries: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            filter_dimension: String::from("全部"),
            filter_business_type: String::from("全部"),
            viewing_record: None,
            active_tab: Tab::Records,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadData => {
                self.loading = true;
                self.error = None;

                // 加载维度列表
                let link = ctx.link().clone();
                spawn_local(async move {
                    match AssistAccountingService::list_dimensions().await {
                        Ok(data) => link.send_message(Msg::DimensionsLoaded(Ok(data))),
                        Err(e) => link.send_message(Msg::DimensionsLoaded(Err(e))),
                    }
                });

                // 加载记录列表
                let link = ctx.link().clone();
                let params = AssistRecordQueryParams {
                    accounting_period: None,
                    dimension_code: if self.filter_dimension == "全部" { None } else { Some(self.filter_dimension.clone()) },
                    business_type: if self.filter_business_type == "全部" { None } else { Some(self.filter_business_type.clone()) },
                    warehouse_id: None,
                    page: Some(1),
                    page_size: Some(1000),
                };
                spawn_local(async move {
                    match AssistAccountingService::query_records(params).await {
                        Ok(data) => link.send_message(Msg::DataLoaded(Ok(data))),
                        Err(e) => link.send_message(Msg::DataLoaded(Err(e))),
                    }
                });

                // 加载汇总数据
                let link = ctx.link().clone();
                let summary_params = AssistSummaryQueryParams {
                    accounting_period: chrono::Local::now().format("%Y-%m").to_string(),
                    dimension_code: None,
                };
                spawn_local(async move {
                    match AssistAccountingService::get_summary(summary_params).await {
                        Ok(data) => link.send_message(Msg::SummaryLoaded(Ok(data))),
                        Err(e) => link.send_message(Msg::SummaryLoaded(Err(e))),
                    }
                });

                false
            }
            Msg::DataLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(data) => {
                        self.records = data.records;
                        self.apply_filter();
                        self.error = None;
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
                true
            }
            Msg::DimensionsLoaded(result) => {
                if let Ok(data) = result {
                    self.dimensions = data;
                }
                true
            }
            Msg::SummaryLoaded(result) => {
                if let Ok(data) = result {
                    self.summaries = data;
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
            Msg::SetFilterDimension(dimension) => {
                self.filter_dimension = dimension;
                self.page = 0;
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::SetFilterBusinessType(tp) => {
                self.filter_business_type = tp;
                self.page = 0;
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::CloseDetail => {
                self.viewing_record = None;
                true
            }
            Msg::ViewRecord(record) => {
                self.viewing_record = Some(record);
                true
            }
            Msg::SwitchTab(tab) => {
                self.active_tab = tab;
                true
            }
            Msg::Refresh => {
                ctx.link().send_message(Msg::LoadData);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="assist-accounting-page">
                <PageHeader title={"辅助核算".to_string()} subtitle={Some("查询和统计辅助核算数据".to_string())}>
                    <></>
                </PageHeader>

                <div class="tab-bar">
                    <button
                        class={if self.active_tab == Tab::Records { "tab-btn active" } else { "tab-btn" }}
                        onclick={ctx.link().callback(|_| Msg::SwitchTab(Tab::Records))}
                    >
                        {"核算记录"}
                    </button>
                    <button
                        class={if self.active_tab == Tab::Summary { "tab-btn active" } else { "tab-btn" }}
                        onclick={ctx.link().callback(|_| Msg::SwitchTab(Tab::Summary))}
                    >
                        {"汇总统计"}
                    </button>
                </div>

                {self.render_content(ctx)}

                // 详情弹窗
                if let Some(ref record) = self.viewing_record {
                    {self.render_detail(ctx, record)}
                }
            </div>
        }
    }
}

impl AssistAccountingPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_records = self.records.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_records = self.records.iter()
                .filter(|r| {
                    r.business_type.to_lowercase().contains(&keyword) ||
                    r.business_no.to_lowercase().contains(&keyword) ||
                    r.batch_no.to_lowercase().contains(&keyword) ||
                    r.color_no.to_lowercase().contains(&keyword) ||
                    r.grade.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_records(&self) -> Vec<AssistRecord> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_records[start..end.min(self.filtered_records.len())].to_vec()
    }

    fn render_content(&self, ctx: &Context<Self>) -> Html {
        match self.active_tab {
            Tab::Records => self.render_records_tab(ctx),
            Tab::Summary => self.render_summary_tab(ctx),
        }
    }

    fn render_records_tab(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        let on_dimension_change = link.batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlSelectElement>().ok()?;
            Some(Msg::SetFilterDimension(target.value()))
        });

        let on_business_type_change = link.batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<web_sys::HtmlSelectElement>().ok()?;
            Some(Msg::SetFilterBusinessType(target.value()))
        });

        html! {
            <>
                <div class="page-toolbar">
                    <div class="filter-bar">
                        <div class="filter-item">
                            <label>{"核算维度："}</label>
                            <select value={self.filter_dimension.clone()} onchange={on_dimension_change}>
                                <option value="全部">{"全部"}</option>
                                {for self.dimensions.iter().map(|d| {
                                    let code = d.dimension_code.clone();
                                    html! {
                                        <option value={code.clone()}>{&d.dimension_name}</option>
                                    }
                                })}
                            </select>
                        </div>
                        <div class="filter-item">
                            <label>{"业务类型："}</label>
                            <select value={self.filter_business_type.clone()} onchange={on_business_type_change}>
                                <option value="全部">{"全部"}</option>
                                <option value="采购入库">{"采购入库"}</option>
                                <option value="销售出库">{"销售出库"}</option>
                                <option value="生产领料">{"生产领料"}</option>
                                <option value="生产入库">{"生产入库"}</option>
                                <option value="调拨">{"调拨"}</option>
                                <option value="盘点">{"盘点"}</option>
                            </select>
                        </div>
                    </div>
                    <SearchBar
                        placeholder={"搜索业务单号、批次号、色号...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载辅助核算数据...".to_string()} />
                } else if let Some(error) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{error}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadData)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_records.is_empty() {
                    <EmptyState
                        icon={"📑".to_string()}
                        title={"暂无辅助核算记录".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "暂无数据".to_string()
                        } else {
                            "没有匹配搜索条件的记录".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"业务类型"}</th>
                                    <th>{"业务单号"}</th>
                                    <th>{"科目ID"}</th>
                                    <th>{"五维ID"}</th>
                                    <th>{"批次号"}</th>
                                    <th>{"色号"}</th>
                                    <th>{"等级"}</th>
                                    <th class="numeric">{"借方金额"}</th>
                                    <th class="numeric">{"贷方金额"}</th>
                                    <th class="numeric">{"数量(米)"}</th>
                                    <th class="numeric">{"数量(公斤)"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_records().iter().map(|record| {
                                    let record_clone = record.clone();
                                    html! {
                                        <tr>
                                            <td>{&record.business_type}</td>
                                            <td>{&record.business_no}</td>
                                            <td>{record.account_subject_id}</td>
                                            <td>{&record.five_dimension_id}</td>
                                            <td>{&record.batch_no}</td>
                                            <td>{&record.color_no}</td>
                                            <td>{&record.grade}</td>
                                            <td class="numeric">{format_decimal(&record.debit_amount)}</td>
                                            <td class="numeric">{format_decimal(&record.credit_amount)}</td>
                                            <td class="numeric">{format_decimal(&record.quantity_meters)}</td>
                                            <td class="numeric">{format_decimal(&record.quantity_kg)}</td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-info"
                                                        onclick={link.callback(move |_| Msg::ViewRecord(record_clone.clone()))}
                                                    >
                                                        {"详情"}
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
                            total={self.filtered_records.len() as u64}
                            on_page_change={link.callback(|page| Msg::PageChanged(page))}
                        />
                    </div>
                }
            </>
        }
    }

    fn render_summary_tab(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="summary-container">
                if self.summaries.is_empty() {
                    <EmptyState
                        icon={"📊".to_string()}
                        title={"暂无汇总数据".to_string()}
                        description={"当前会计期间暂无汇总数据".to_string()}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"会计期间"}</th>
                                    <th>{"维度代码"}</th>
                                    <th>{"维度值"}</th>
                                    <th>{"科目ID"}</th>
                                    <th class="numeric">{"借方合计"}</th>
                                    <th class="numeric">{"贷方合计"}</th>
                                    <th class="numeric">{"数量合计(米)"}</th>
                                    <th class="numeric">{"数量合计(公斤)"}</th>
                                    <th>{"记录数"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.summaries.iter().map(|summary| {
                                    html! {
                                        <tr>
                                            <td>{&summary.accounting_period}</td>
                                            <td>{&summary.dimension_code}</td>
                                            <td>{&summary.dimension_value_name}</td>
                                            <td>{summary.account_subject_id}</td>
                                            <td class="numeric">{format_decimal(&summary.total_debit)}</td>
                                            <td class="numeric">{format_decimal(&summary.total_credit)}</td>
                                            <td class="numeric">{format_decimal(&summary.total_quantity_meters)}</td>
                                            <td class="numeric">{format_decimal(&summary.total_quantity_kg)}</td>
                                            <td>{summary.record_count}</td>
                                        </tr>
                                    }
                                })}
                            </tbody>
                        </table>
                    </div>
                }
            </div>
        }
    }

    fn render_detail(&self, ctx: &Context<Self>, record: &AssistRecord) -> Html {
        html! {
            <div class="modal-overlay" onclick={ctx.link().callback(|_| Msg::CloseDetail)}>
                <div class="modal-content modal-lg" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <div class="modal-header">
                        <h3>{"辅助核算详情"}</h3>
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
                                <span class="label">{"科目ID"}</span>
                                <span class="value">{record.account_subject_id.to_string()}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"五维ID"}</span>
                                <span class="value">{&record.five_dimension_id}</span>
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
                                <span class="label">{"缸号"}</span>
                                <span class="value">{record.dye_lot_no.as_deref().unwrap_or("-")}</span>
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
                                <span class="value numeric">{format_decimal(&record.debit_amount)}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"贷方金额"}</span>
                                <span class="value numeric">{format_decimal(&record.credit_amount)}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"数量(米)"}</span>
                                <span class="value numeric">{format_decimal(&record.quantity_meters)}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"数量(公斤)"}</span>
                                <span class="value numeric">{format_decimal(&record.quantity_kg)}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"车间ID"}</span>
                                <span class="value">{record.workshop_id.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string())}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"客户ID"}</span>
                                <span class="value">{record.customer_id.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string())}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"供应商ID"}</span>
                                <span class="value">{record.supplier_id.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string())}</span>
                            </div>
                            <div class="detail-item full-width">
                                <span class="label">{"备注"}</span>
                                <span class="value">{record.remarks.as_deref().unwrap_or("-")}</span>
                            </div>
                            <div class="detail-item">
                                <span class="label">{"创建时间"}</span>
                                <span class="value">{&record.created_at}</span>
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
}

fn format_decimal(value: &serde_json::Value) -> String {
    if let Some(num) = value.as_f64() {
        format!("{:.2}", num)
    } else if let Some(num) = value.as_i64() {
        num.to_string()
    } else if let Some(s) = value.as_str() {
        s.to_string()
    } else {
        value.to_string()
    }
}
