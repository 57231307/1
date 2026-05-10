// 业务追溯页面
// 提供面料业务的正向追溯和反向追溯功能

use crate::components::{
    empty_state::EmptyState,
    loading_state::LoadingState,
    page_header::PageHeader,
    pagination::Pagination,
    search_bar::SearchBar,
};
use crate::components::permission_guard::PermissionGuard;
use crate::models::business_trace::{
    FullTraceChainResponse, TraceChain, TraceStageDetail,
};
use crate::services::business_trace_service::BusinessTraceService;
use crate::services::crud_service::CrudService;
use crate::utils::permissions;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;

/// 业务追溯页面状态
pub struct BusinessTracePage {
    trace_chain: Option<FullTraceChainResponse>,
    forward_traces: Vec<TraceChain>,
    backward_traces: Vec<TraceChain>,
    loading: bool,
    error: Option<String>,
    active_tab: String,
    five_dimension_id_input: String,
    forward_supplier_id: String,
    forward_batch_no: String,
    backward_customer_id: String,
    backward_batch_no: String,
    search_keyword: String,
    page: u64,
    page_size: u64,
    filtered_forward: Vec<TraceChain>,
    filtered_backward: Vec<TraceChain>,
}

/// 页面消息
pub enum Msg {
    UpdateFiveDimensionId(String),
    QueryByFiveDimension,
    FiveDimensionLoaded(Result<FullTraceChainResponse, String>),
    UpdateForwardSupplierId(String),
    UpdateForwardBatchNo(String),
    ForwardTrace,
    ForwardTraceLoaded(Result<Vec<TraceChain>, String>),
    UpdateBackwardCustomerId(String),
    UpdateBackwardBatchNo(String),
    BackwardTrace,
    BackwardTraceLoaded(Result<Vec<TraceChain>, String>),
    SetActiveTab(String),
    CreateSnapshot(String),
    SnapshotCreated(Result<String, String>),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    Error(String),
}

impl Component for BusinessTracePage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            trace_chain: None,
            forward_traces: Vec::new(),
            backward_traces: Vec::new(),
            loading: false,
            error: None,
            active_tab: "five_dimension".to_string(),
            five_dimension_id_input: String::new(),
            forward_supplier_id: String::new(),
            forward_batch_no: String::new(),
            backward_customer_id: String::new(),
            backward_batch_no: String::new(),
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            filtered_forward: Vec::new(),
            filtered_backward: Vec::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::UpdateFiveDimensionId(id) => {
                self.five_dimension_id_input = id;
                false
            }
            Msg::QueryByFiveDimension => {
                if self.five_dimension_id_input.is_empty() {
                    self.error = Some("请输入五维ID".to_string());
                    return true;
                }
                self.loading = true;
                self.error = None;
                let link = ctx.link().clone();
                let five_dimension_id = self.five_dimension_id_input.clone();
                spawn_local(async move {
                    let result = BusinessTraceService::get_trace_by_five_dimension(&five_dimension_id).await;
                    link.send_message(Msg::FiveDimensionLoaded(result));
                });
                false
            }
            Msg::FiveDimensionLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(data) => {
                        self.trace_chain = Some(data);
                        self.error = None;
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
                true
            }
            Msg::UpdateForwardSupplierId(id) => {
                self.forward_supplier_id = id;
                false
            }
            Msg::UpdateForwardBatchNo(batch_no) => {
                self.forward_batch_no = batch_no;
                false
            }
            Msg::ForwardTrace => {
                let supplier_id = match self.forward_supplier_id.parse::<i32>() {
                    Ok(id) => id,
                    Err(_) => {
                        self.error = Some("请输入有效的供应商ID".to_string());
                        return true;
                    }
                };
                if self.forward_batch_no.is_empty() {
                    self.error = Some("请输入批次号".to_string());
                    return true;
                }
                self.loading = true;
                self.error = None;
                let link = ctx.link().clone();
                let batch_no = self.forward_batch_no.clone();
                spawn_local(async move {
                    let result = BusinessTraceService::forward_trace(supplier_id, &batch_no).await;
                    match result {
                        Ok(response) => link.send_message(Msg::ForwardTraceLoaded(Ok(response.traces))),
                        Err(e) => link.send_message(Msg::ForwardTraceLoaded(Err(e))),
                    }
                });
                false
            }
            Msg::ForwardTraceLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(traces) => {
                        self.forward_traces = traces;
                        self.apply_filter();
                        self.error = None;
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
                true
            }
            Msg::UpdateBackwardCustomerId(id) => {
                self.backward_customer_id = id;
                false
            }
            Msg::UpdateBackwardBatchNo(batch_no) => {
                self.backward_batch_no = batch_no;
                false
            }
            Msg::BackwardTrace => {
                let customer_id = match self.backward_customer_id.parse::<i32>() {
                    Ok(id) => id,
                    Err(_) => {
                        self.error = Some("请输入有效的客户ID".to_string());
                        return true;
                    }
                };
                if self.backward_batch_no.is_empty() {
                    self.error = Some("请输入批次号".to_string());
                    return true;
                }
                self.loading = true;
                self.error = None;
                let link = ctx.link().clone();
                let batch_no = self.backward_batch_no.clone();
                spawn_local(async move {
                    let result = BusinessTraceService::backward_trace(customer_id, &batch_no).await;
                    match result {
                        Ok(response) => link.send_message(Msg::BackwardTraceLoaded(Ok(response.traces))),
                        Err(e) => link.send_message(Msg::BackwardTraceLoaded(Err(e))),
                    }
                });
                false
            }
            Msg::BackwardTraceLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(traces) => {
                        self.backward_traces = traces;
                        self.apply_filter();
                        self.error = None;
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
                true
            }
            Msg::SetActiveTab(tab) => {
                self.active_tab = tab;
                self.page = 0;
                self.search_keyword = String::new();
                self.apply_filter();
                true
            }
            Msg::CreateSnapshot(trace_chain_id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    let result = BusinessTraceService::create_snapshot(&trace_chain_id).await;
                    link.send_message(Msg::SnapshotCreated(result));
                });
                false
            }
            Msg::SnapshotCreated(result) => {
                match result {
                    Ok(msg) => {
                        log::info!("快照创建成功: {}", msg);
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
            Msg::Error(e) => {
                self.error = Some(e);
                self.loading = false;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="business-trace-page">
                <PageHeader title={"业务追溯".to_string()} subtitle={Some("面料业务正向追溯和反向追溯".to_string())}>
                    <button class="btn btn-primary" onclick={ctx.link().callback(|_| Msg::SetActiveTab("five_dimension".to_string()))}>
                        {"五维ID追溯"}
                    </button>
                </PageHeader>

                {self.render_tabs(ctx)}
                {self.render_content(ctx)}

                if let Some(error) = &self.error {
                    <div class="error-toast">
                        <span class="error-icon">{"⚠️"}</span>
                        <span class="error-message">{error}</span>
                    </div>
                }
            </div>
        }
    }
}

impl BusinessTracePage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_forward = self.forward_traces.clone();
            self.filtered_backward = self.backward_traces.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_forward = self
                .forward_traces
                .iter()
                .filter(|t| {
                    t.trace_chain_id.to_lowercase().contains(&keyword)
                        || t.five_dimension_id.to_lowercase().contains(&keyword)
                        || t.batch_no.to_lowercase().contains(&keyword)
                        || t.color_no.to_lowercase().contains(&keyword)
                        || t.grade.to_lowercase().contains(&keyword)
                        || t.current_bill_no.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
            self.filtered_backward = self
                .backward_traces
                .iter()
                .filter(|t| {
                    t.trace_chain_id.to_lowercase().contains(&keyword)
                        || t.five_dimension_id.to_lowercase().contains(&keyword)
                        || t.batch_no.to_lowercase().contains(&keyword)
                        || t.color_no.to_lowercase().contains(&keyword)
                        || t.grade.to_lowercase().contains(&keyword)
                        || t.current_bill_no.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_items<T: Clone>(&self, items: &[T]) -> Vec<T> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        items[start..end.min(items.len())].to_vec()
    }

    fn render_tabs(&self, ctx: &Context<Self>) -> Html {
        let tabs = vec![
            ("five_dimension", "五维ID追溯"),
            ("forward", "正向追溯"),
            ("backward", "反向追溯"),
        ];
        html! {
            <div class="tabs">
                {for tabs.iter().map(|(key, label)| {
                    let is_active = self.active_tab == *key;
                    let key_clone = key.to_string();
                    html! {
                        <button
                            class={if is_active { "tab-btn active" } else { "tab-btn" }}
                            onclick={ctx.link().callback(move |_| Msg::SetActiveTab(key_clone.clone()))}
                        >
                            {label}
                        </button>
                    }
                })}
            </div>
        }
    }

    fn render_content(&self, ctx: &Context<Self>) -> Html {
        if self.loading {
            return html! {
                <LoadingState message={"正在加载追溯数据...".to_string()} />
            };
        }

        match self.active_tab.as_str() {
            "five_dimension" => self.render_five_dimension_tab(ctx),
            "forward" => self.render_forward_tab(ctx),
            "backward" => self.render_backward_tab(ctx),
            _ => html! {},
        }
    }

    fn render_five_dimension_tab(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="card">
                <div class="card-header">
                    <h2>{"按五维ID查询追溯链"}</h2>
                </div>
                <div class="card-body">
                    <div class="query-form">
                        <div class="form-group">
                            <label for="five-dimension-id">{"五维ID"}</label>
                            <input
                                id="five-dimension-id"
                                type="text"
                                class="form-control"
                                placeholder="请输入五维ID"
                                value={self.five_dimension_id_input.clone()}
                                oninput={ctx.link().callback(|e: InputEvent| {
                                    let target = e.target_unchecked_into::<HtmlInputElement>();
                                    Msg::UpdateFiveDimensionId(target.value())
                                })}
                            />
                        </div>
                        <button
                            class="btn btn-primary"
                            onclick={ctx.link().callback(|_| Msg::QueryByFiveDimension)}
                            disabled={self.loading}
                        >
                            {if self.loading { "查询中..." } else { "查询" }}
                        </button>
                    </div>

                    if let Some(ref chain) = self.trace_chain {
                        {self.render_trace_chain(ctx, chain)}
                    } else if !self.loading && self.five_dimension_id_input.is_empty() {
                        <EmptyState
                            icon={"🔍".to_string()}
                            title={"请输入五维ID进行查询".to_string()}
                            description={"输入五维ID后可查看完整追溯链信息".to_string()}
                        />
                    }
                </div>
            </div>
        }
    }

    fn render_forward_tab(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <div class="card">
                    <div class="card-header">
                        <h2>{"正向追溯 - 从供应商到客户"}</h2>
                    </div>
                    <div class="card-body">
                        <div class="query-form">
                            <div class="form-row">
                                <div class="form-group">
                                    <label for="supplier-id">{"供应商ID"}</label>
                                    <input
                                        id="supplier-id"
                                        type="number"
                                        class="form-control"
                                        placeholder="请输入供应商ID"
                                        value={self.forward_supplier_id.clone()}
                                        oninput={ctx.link().callback(|e: InputEvent| {
                                            let target = e.target_unchecked_into::<HtmlInputElement>();
                                            Msg::UpdateForwardSupplierId(target.value())
                                        })}
                                    />
                                </div>
                                <div class="form-group">
                                    <label for="forward-batch-no">{"批次号"}</label>
                                    <input
                                        id="forward-batch-no"
                                        type="text"
                                        class="form-control"
                                        placeholder="请输入批次号"
                                        value={self.forward_batch_no.clone()}
                                        oninput={ctx.link().callback(|e: InputEvent| {
                                            let target = e.target_unchecked_into::<HtmlInputElement>();
                                            Msg::UpdateForwardBatchNo(target.value())
                                        })}
                                    />
                                </div>
                            </div>
                            <button
                                class="btn btn-primary"
                                onclick={ctx.link().callback(|_| Msg::ForwardTrace)}
                                disabled={self.loading}
                            >
                                {if self.loading { "追溯中..." } else { "开始追溯" }}
                            </button>
                        </div>
                    </div>
                </div>

                if !self.forward_traces.is_empty() {
                    <div class="page-toolbar">
                        <SearchBar
                            placeholder={"搜索追溯链ID、批次号或色号...".to_string()}
                            on_search={ctx.link().callback(|keyword| Msg::Search(keyword))}
                            on_reset={ctx.link().callback(|_| Msg::ResetSearch)}
                        />
                    </div>
                    {self.render_trace_list(ctx, &self.filtered_forward, "forward")}
                } else if !self.loading && !self.forward_supplier_id.is_empty() {
                    <EmptyState
                        icon={"📭".to_string()}
                        title={"暂无正向追溯结果".to_string()}
                        description={"请检查供应商ID和批次号是否正确".to_string()}
                    />
                }
            </>
        }
    }

    fn render_backward_tab(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <div class="card">
                    <div class="card-header">
                        <h2>{"反向追溯 - 从客户到供应商"}</h2>
                    </div>
                    <div class="card-body">
                        <div class="query-form">
                            <div class="form-row">
                                <div class="form-group">
                                    <label for="customer-id">{"客户ID"}</label>
                                    <input
                                        id="customer-id"
                                        type="number"
                                        class="form-control"
                                        placeholder="请输入客户ID"
                                        value={self.backward_customer_id.clone()}
                                        oninput={ctx.link().callback(|e: InputEvent| {
                                            let target = e.target_unchecked_into::<HtmlInputElement>();
                                            Msg::UpdateBackwardCustomerId(target.value())
                                        })}
                                    />
                                </div>
                                <div class="form-group">
                                    <label for="backward-batch-no">{"批次号"}</label>
                                    <input
                                        id="backward-batch-no"
                                        type="text"
                                        class="form-control"
                                        placeholder="请输入批次号"
                                        value={self.backward_batch_no.clone()}
                                        oninput={ctx.link().callback(|e: InputEvent| {
                                            let target = e.target_unchecked_into::<HtmlInputElement>();
                                            Msg::UpdateBackwardBatchNo(target.value())
                                        })}
                                    />
                                </div>
                            </div>
                            <button
                                class="btn btn-primary"
                                onclick={ctx.link().callback(|_| Msg::BackwardTrace)}
                                disabled={self.loading}
                            >
                                {if self.loading { "追溯中..." } else { "开始追溯" }}
                            </button>
                        </div>
                    </div>
                </div>

                if !self.backward_traces.is_empty() {
                    <div class="page-toolbar">
                        <SearchBar
                            placeholder={"搜索追溯链ID、批次号或色号...".to_string()}
                            on_search={ctx.link().callback(|keyword| Msg::Search(keyword))}
                            on_reset={ctx.link().callback(|_| Msg::ResetSearch)}
                        />
                    </div>
                    {self.render_trace_list(ctx, &self.filtered_backward, "backward")}
                } else if !self.loading && !self.backward_customer_id.is_empty() {
                    <EmptyState
                        icon={"📭".to_string()}
                        title={"暂无反向追溯结果".to_string()}
                        description={"请检查客户ID和批次号是否正确".to_string()}
                    />
                }
            </>
        }
    }

    fn render_trace_chain(&self, ctx: &Context<Self>, chain: &FullTraceChainResponse) -> Html {
        let chain_id = chain.trace_chain_id.clone();
        html! {
            <div class="trace-chain-detail">
                <div class="chain-header">
                    <h3>{"追溯链详情"}</h3>
                    <PermissionGuard resource="business_trace" action="create">
                        <button
                            class="btn btn-secondary"
                            onclick={ctx.link().callback(move |_| Msg::CreateSnapshot(chain_id.clone()))}
                        >
                            {"创建快照"}
                        </button>
                    </PermissionGuard>
                </div>

                <div class="chain-info">
                    <div class="info-item">
                        <span class="label">{"追溯链ID："}</span>
                        <span class="value">{&chain.trace_chain_id}</span>
                    </div>
                    <div class="info-item">
                        <span class="label">{"五维ID："}</span>
                        <span class="value five-dim-id">{&chain.five_dimension_id}</span>
                    </div>
                    <div class="info-item">
                        <span class="label">{"产品ID："}</span>
                        <span class="value">{chain.product_id}</span>
                    </div>
                    <div class="info-item">
                        <span class="label">{"批次号："}</span>
                        <span class="value">{&chain.batch_no}</span>
                    </div>
                    <div class="info-item">
                        <span class="label">{"色号："}</span>
                        <span class="value">{&chain.color_no}</span>
                    </div>
                    <div class="info-item">
                        <span class="label">{"等级："}</span>
                        <span class="value">{&chain.grade}</span>
                    </div>
                    <div class="info-item">
                        <span class="label">{"总环节数："}</span>
                        <span class="value">{chain.total_stages}</span>
                    </div>
                    <div class="info-item">
                        <span class="label">{"开始时间："}</span>
                        <span class="value">{&chain.start_time}</span>
                    </div>
                    if let Some(ref end_time) = chain.end_time {
                        <div class="info-item">
                            <span class="label">{"结束时间："}</span>
                            <span class="value">{end_time}</span>
                        </div>
                    }
                </div>

                <div class="stages-section">
                    <h4>{"追溯环节"}</h4>
                    <div class="stages-timeline">
                        {for chain.stages.iter().enumerate().map(|(index, stage)| {
                            self.render_stage_item(index, stage)
                        })}
                    </div>
                </div>
            </div>
        }
    }

    fn render_stage_item(&self, index: usize, stage: &TraceStageDetail) -> Html {
        html! {
            <div class="stage-item">
                <div class="stage-number">{(index + 1).to_string()}</div>
                <div class="stage-content">
                    <div class="stage-header">
                        <span class="stage-name">{&stage.stage_name}</span>
                        <span class="stage-type">{&stage.stage_type}</span>
                    </div>
                    <div class="stage-details">
                        <div class="detail-row">
                            <span class="label">{"单据类型："}</span>
                            <span class="value">{&stage.bill_type}</span>
                        </div>
                        <div class="detail-row">
                            <span class="label">{"单据编号："}</span>
                            <span class="value">{&stage.bill_no}</span>
                        </div>
                        <div class="detail-row">
                            <span class="label">{"数量(米)："}</span>
                            <span class="value numeric">{&stage.quantity_meters}</span>
                        </div>
                        <div class="detail-row">
                            <span class="label">{"数量(公斤)："}</span>
                            <span class="value numeric">{&stage.quantity_kg}</span>
                        </div>
                        if let Some(ref warehouse_name) = stage.warehouse_name {
                            <div class="detail-row">
                                <span class="label">{"仓库："}</span>
                                <span class="value">{warehouse_name}</span>
                            </div>
                        }
                        if let Some(ref supplier_name) = stage.supplier_name {
                            <div class="detail-row">
                                <span class="label">{"供应商："}</span>
                                <span class="value">{supplier_name}</span>
                            </div>
                        }
                        if let Some(ref customer_name) = stage.customer_name {
                            <div class="detail-row">
                                <span class="label">{"客户："}</span>
                                <span class="value">{customer_name}</span>
                            </div>
                        }
                        <div class="detail-row">
                            <span class="label">{"时间："}</span>
                            <span class="value">{&stage.created_at}</span>
                        </div>
                    </div>
                </div>
            </div>
        }
    }

    fn render_trace_list(&self, ctx: &Context<Self>, traces: &[TraceChain], _mode: &str) -> Html {
        if traces.is_empty() && !self.search_keyword.is_empty() {
            return html! {
                <EmptyState
                    icon={"🔍".to_string()}
                    title={"暂无匹配数据".to_string()}
                    description={"没有匹配搜索条件的追溯记录".to_string()}
                />
            };
        }

        html! {
            <div class="table-container">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>{"追溯链ID"}</th>
                            <th>{"五维ID"}</th>
                            <th>{"产品ID"}</th>
                            <th>{"批次号"}</th>
                            <th>{"色号"}</th>
                            <th>{"等级"}</th>
                            <th>{"当前环节"}</th>
                            <th>{"当前单据"}</th>
                            <th>{"数量(米)"}</th>
                            <th>{"数量(公斤)"}</th>
                            <th>{"状态"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.paginated_items(traces).iter().map(|trace| {
                            html! {
                                <tr>
                                    <td class="trace-chain-id">{&trace.trace_chain_id}</td>
                                    <td class="five-dim-id">{&trace.five_dimension_id}</td>
                                    <td>{trace.product_id}</td>
                                    <td>{&trace.batch_no}</td>
                                    <td>{&trace.color_no}</td>
                                    <td>{&trace.grade}</td>
                                    <td>{&trace.current_stage}</td>
                                    <td>{&trace.current_bill_no}</td>
                                    <td class="numeric">{&trace.quantity_meters}</td>
                                    <td class="numeric">{&trace.quantity_kg}</td>
                                    <td>
                                        <span class={format!("status-badge status-{}", trace.trace_status.to_lowercase())}>
                                            {&trace.trace_status}
                                        </span>
                                    </td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
                <Pagination
                    current_page={self.page}
                    page_size={self.page_size}
                    total={traces.len() as u64}
                    on_page_change={ctx.link().callback(|page| Msg::PageChanged(page))}
                />
            </div>
        }
    }
}
