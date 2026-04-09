//! 业务追溯页面
//! 提供面料业务的正向追溯和反向追溯功能

use crate::components::main_layout::MainLayout;
use crate::models::business_trace::{FullTraceChainResponse, TraceChain, TraceStageDetail};
use crate::services::business_trace_service::BusinessTraceService;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use web_sys::window;

/// 业务追溯页面状态
pub struct BusinessTracePage {
    // 追溯链数据
    trace_chain: Option<FullTraceChainResponse>,
    // 正向追溯结果
    forward_traces: Vec<TraceChain>,
    // 反向追溯结果
    backward_traces: Vec<TraceChain>,
    // 加载状态
    loading: bool,
    // 错误信息
    error: Option<String>,
    // 当前标签页
    active_tab: String,
    // 五维ID查询输入
    five_dimension_id_input: String,
    // 正向追溯参数
    forward_supplier_id: String,
    forward_batch_no: String,
    // 反向追溯参数
    backward_customer_id: String,
    backward_batch_no: String,
    // 追溯模式
    trace_mode: String,
}

/// 页面消息
pub enum Msg {
    // 五维ID查询
    UpdateFiveDimensionId(String),
    QueryByFiveDimension,
    FiveDimensionLoaded(Result<FullTraceChainResponse, String>),
    // 正向追溯
    UpdateForwardSupplierId(String),
    UpdateForwardBatchNo(String),
    ForwardTrace,
    ForwardTraceLoaded(Result<Vec<TraceChain>, String>),
    // 反向追溯
    UpdateBackwardCustomerId(String),
    UpdateBackwardBatchNo(String),
    BackwardTrace,
    BackwardTraceLoaded(Result<Vec<TraceChain>, String>),
    // 切换标签页
    SetActiveTab(String),
    // 切换追溯模式
    SetTraceMode(String),
    // 创建快照
    CreateSnapshot(String),
    SnapshotCreated(Result<String, String>),
    // 错误处理
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
            trace_mode: "five_dimension".to_string(),
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
                    let result =
                        BusinessTraceService::get_trace_by_five_dimension(&five_dimension_id).await;
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
                        Ok(response) => {
                            link.send_message(Msg::ForwardTraceLoaded(Ok(response.traces)))
                        }
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
                        Ok(response) => {
                            link.send_message(Msg::BackwardTraceLoaded(Ok(response.traces)))
                        }
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
                true
            }
            Msg::SetTraceMode(mode) => {
                self.trace_mode = mode;
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
                        // 可以显示成功消息
                        log::info!("快照创建成功: {}", msg);
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
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
            <MainLayout current_page="business_trace">
                <div class="business-trace-page">
                    <div class="page-header">
                        <h1>{"业务追溯"}</h1>
                        <p class="subtitle">{"面料业务正向追溯（供应商→客户）和反向追溯（客户→供应商）"}</p>
                    </div>

                    {self.render_content(ctx)}
                </div>
            </MainLayout>
        }
    }
}

impl BusinessTracePage {
    fn render_content(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                // 标签页导航
                {self.render_tabs(ctx)}

                // 五维ID追溯标签页
                if self.active_tab == "five_dimension" {
                    {self.render_five_dimension_tab(ctx)}
                }

                // 正向追溯标签页
                if self.active_tab == "forward" {
                    {self.render_forward_tab(ctx)}
                }

                // 反向追溯标签页
                if self.active_tab == "backward" {
                    {self.render_backward_tab(ctx)}
                }

                // 错误提示
                if let Some(error) = &self.error {
                    <div class="error-toast">
                        <span class="error-icon">{"⚠️"}</span>
                        <span class="error-message">{error}</span>
                    </div>
                }
            </>
        }
    }

    // 渲染标签页
    fn render_tabs(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="tabs">
                <button
                    class={if self.active_tab == "five_dimension" { "tab-btn active" } else { "tab-btn" }}
                    onclick={ctx.link().callback(|_| Msg::SetActiveTab("five_dimension".to_string()))}
                >
                    {"五维ID追溯"}
                </button>
                <button
                    class={if self.active_tab == "forward" { "tab-btn active" } else { "tab-btn" }}
                    onclick={ctx.link().callback(|_| Msg::SetActiveTab("forward".to_string()))}
                >
                    {"正向追溯"}
                </button>
                <button
                    class={if self.active_tab == "backward" { "tab-btn active" } else { "tab-btn" }}
                    onclick={ctx.link().callback(|_| Msg::SetActiveTab("backward".to_string()))}
                >
                    {"反向追溯"}
                </button>
            </div>
        }
    }

    // 渲染五维ID追溯标签页
    fn render_five_dimension_tab(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="card">
                <div class="card-header">
                    <h2>{"按五维ID查询追溯链"}</h2>
                </div>
                <div class="card-body">
                    // 查询表单
                    <div class="query-form">
                        <div class="form-group">
                            <label for="five-dimension-id">{"五维ID"}</label>
                            <input
                                id="five-dimension-id"
                                type="text"
                                class="form-control"
                                placeholder="请输入五维ID，如：P1|B20240101|C001|D001|G 一等品"
                                value={self.five_dimension_id_input.clone()}
                                oninput={ctx.link().callback(|e: InputEvent| {
                                    let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                    Msg::UpdateFiveDimensionId(target.value())
                                })}
                            />
                        </div>
                        <button
                            class="btn-primary"
                            onclick={ctx.link().callback(|_| Msg::QueryByFiveDimension)}
                            disabled={self.loading}
                        >
                            {if self.loading { "查询中..." } else { "查询" }}
                        </button>
                    </div>

                    // 追溯链详情
                    if let Some(ref chain) = self.trace_chain {
                        {self.render_trace_chain(ctx, chain)}
                    }
                </div>
            </div>
        }
    }

    // 渲染正向追溯标签页
    fn render_forward_tab(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="card">
                <div class="card-header">
                    <h2>{"正向追溯 - 从供应商到客户"}</h2>
                </div>
                <div class="card-body">
                    // 查询表单
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
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
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
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateForwardBatchNo(target.value())
                                    })}
                                />
                            </div>
                        </div>
                        <button
                            class="btn-primary"
                            onclick={ctx.link().callback(|_| Msg::ForwardTrace)}
                            disabled={self.loading}
                        >
                            {if self.loading { "追溯中..." } else { "开始追溯" }}
                        </button>
                    </div>

                    // 追溯结果
                    if !self.forward_traces.is_empty() {
                        {self.render_trace_list(ctx, &self.forward_traces, "forward")}
                    }
                </div>
            </div>
        }
    }

    // 渲染反向追溯标签页
    fn render_backward_tab(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="card">
                <div class="card-header">
                    <h2>{"反向追溯 - 从客户到供应商"}</h2>
                </div>
                <div class="card-body">
                    // 查询表单
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
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
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
                                        let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                        Msg::UpdateBackwardBatchNo(target.value())
                                    })}
                                />
                            </div>
                        </div>
                        <button
                            class="btn-primary"
                            onclick={ctx.link().callback(|_| Msg::BackwardTrace)}
                            disabled={self.loading}
                        >
                            {if self.loading { "追溯中..." } else { "开始追溯" }}
                        </button>
                    </div>

                    // 追溯结果
                    if !self.backward_traces.is_empty() {
                        {self.render_trace_list(ctx, &self.backward_traces, "backward")}
                    }
                </div>
            </div>
        }
    }

    // 渲染追溯链详情
    fn render_trace_chain(&self, ctx: &Context<Self>, chain: &FullTraceChainResponse) -> Html {
        let chain_id = chain.trace_chain_id.clone();
        html! {
            <div class="trace-chain-detail">
                <div class="chain-header">
                    <h3>{"追溯链详情"}</h3>
                    <button
                        class="btn-secondary"
                        onclick={ctx.link().callback(move |_| Msg::CreateSnapshot(chain_id.clone()))}
                    >
                        {"创建快照"}
                    </button>
                </div>

                // 基本信息
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
                        <span class="value">{chain.product_id.to_string()}</span>
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
                        <span class="value">{chain.total_stages.to_string()}</span>
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

                // 追溯环节列表
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

    // 渲染追溯环节项
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
                            <span class="value numeric">{format!("{:.2}", stage.quantity_meters)}</span>
                        </div>
                        <div class="detail-row">
                            <span class="label">{"数量(公斤)："}</span>
                            <span class="value numeric">{format!("{:.2}", stage.quantity_kg)}</span>
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

    // 渲染追溯列表
    fn render_trace_list(&self, _ctx: &Context<Self>, traces: &[TraceChain], _mode: &str) -> Html {
        html! {
            <div class="trace-list">
                <h3>{"追溯结果"}</h3>
                <div class="table-responsive">
                    <div class="overflow-x-auto w-full pb-4">
<table class="data-table w-full">
                        <thead>
                            <tr>
                                <th>{"追溯链ID"}</th>
                                <th>{"五维ID"}</th>
                                <th>{"产品ID"}</th>
                                <th>{"全链路溯源"}</th>
                                <th>{"批次号"}</th>
                                <th>{"色号"}</th>
                                <th>{"等级"}</th>
                                <th>{"当前环节"}</th>
                                <th>{"当前单据"}</th>
                                <th class="numeric-cell text-right">{"数量(米)"}</th>
                                <th class="numeric-cell text-right">{"数量(公斤)"}</th>
                                <th>{"状态"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {for traces.iter().map(|trace| {
                                html! {
                                    <tr>
                                        <td class="trace-chain-id">{&trace.trace_chain_id}</td>
                                        <td class="five-dim-id">{&trace.five_dimension_id}</td>
                                        <td>{trace.product_id.to_string()}</td>
                                        <td>
                                            <div class="trace-link">
                                                <span class="trace-node">{"订单号"}</span>
                                                <span class="trace-arrow">{"->"}</span>
                                                <span class="trace-node">{format!("织造批次({})", trace.batch_no)}</span>
                                                <span class="trace-arrow">{"->"}</span>
                                                <span class="trace-node">{format!("染缸号({})", trace.dye_lot_no.clone().unwrap_or_else(|| "-".to_string()))}</span>
                                                <span class="trace-arrow">{"->"}</span>
                                                <span class="trace-node">{format!("发货单({})", trace.current_bill_no)}</span>
                                            </div>
                                        </td>
                                        <td>{&trace.batch_no}</td>
                                        <td>{&trace.color_no}</td>
                                        <td>{&trace.grade}</td>
                                        <td>{&trace.current_stage}</td>
                                        <td>{&trace.current_bill_no}</td>
                                        <td class="numeric-cell text-right">{format!("{:.2}", trace.quantity_meters)}</td>
                                        <td class="numeric-cell text-right">{format!("{:.2}", trace.quantity_kg)}</td>
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
</div>
                </div>
            </div>
        }
    }
}
