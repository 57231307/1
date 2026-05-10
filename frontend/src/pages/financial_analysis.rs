// 财务分析页面

use crate::components::{
    empty_state::EmptyState,
    loading_state::LoadingState,
    page_header::PageHeader,
    pagination::Pagination,
    search_bar::SearchBar,
};
use crate::models::financial_analysis::{
    AnalysisResult, DupontAnalysis, FinancialIndicator, FinancialRatio,
};
use crate::services::crud_service::CrudService;
use crate::services::financial_analysis_service::FinancialAnalysisService;
use crate::utils::permissions;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

pub struct FinancialAnalysisPage {
    loading: bool,
    error: Option<String>,
    dupont_data: Option<DupontAnalysis>,
    ratios: Vec<FinancialRatio>,
    indicators: Vec<FinancialIndicator>,
    analysis_results: Vec<AnalysisResult>,
    period: String,
    active_tab: String,
    search_keyword: String,
    page: u64,
    page_size: u64,
    filtered_ratios: Vec<FinancialRatio>,
    filtered_indicators: Vec<FinancialIndicator>,
    filtered_results: Vec<AnalysisResult>,
}

pub enum Msg {
    LoadData,
    DupontLoaded(Result<DupontAnalysis, String>),
    RatiosLoaded(Result<Vec<FinancialRatio>, String>),
    IndicatorsLoaded(Result<Vec<FinancialIndicator>, String>),
    ResultsLoaded(Result<Vec<AnalysisResult>, String>),
    SetPeriod(String),
    SetActiveTab(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
}

impl Component for FinancialAnalysisPage {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::LoadData);
        Self {
            loading: true,
            error: None,
            dupont_data: None,
            ratios: Vec::new(),
            indicators: Vec::new(),
            analysis_results: Vec::new(),
            period: "2026-03".to_string(),
            active_tab: "dupont".to_string(),
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            filtered_ratios: Vec::new(),
            filtered_indicators: Vec::new(),
            filtered_results: Vec::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadData => {
                self.loading = true;
                self.error = None;
                let link = ctx.link().clone();
                let period = self.period.clone();
                spawn_local(async move {
                    let dupont_res = FinancialAnalysisService::dupont_analysis(&period).await;
                    link.send_message(Msg::DupontLoaded(dupont_res));
                });

                let link2 = ctx.link().clone();
                let period2 = self.period.clone();
                spawn_local(async move {
                    let ratios_res = FinancialAnalysisService::analyze_ratios(&period2).await;
                    link2.send_message(Msg::RatiosLoaded(ratios_res));
                });

                let link3 = ctx.link().clone();
                spawn_local(async move {
                    let ind_res =
                        FinancialAnalysisService::list_indicators(None, None, 0, 100).await;
                    link3.send_message(Msg::IndicatorsLoaded(ind_res));
                });

                let link4 = ctx.link().clone();
                let period4 = self.period.clone();
                spawn_local(async move {
                    let res = FinancialAnalysisService::list_analysis_results(
                        None,
                        Some(&period4),
                        0,
                        100,
                    )
                    .await;
                    link4.send_message(Msg::ResultsLoaded(res));
                });
                false
            }
            Msg::DupontLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(data) => self.dupont_data = Some(data),
                    Err(e) => self.error = Some(e),
                }
                true
            }
            Msg::RatiosLoaded(result) => {
                match result {
                    Ok(data) => {
                        self.ratios = data;
                        self.apply_filter();
                    }
                    Err(e) => self.error = Some(e),
                }
                true
            }
            Msg::IndicatorsLoaded(result) => {
                match result {
                    Ok(data) => {
                        self.indicators = data;
                        self.apply_filter();
                    }
                    Err(e) => self.error = Some(e),
                }
                true
            }
            Msg::ResultsLoaded(result) => {
                match result {
                    Ok(data) => {
                        self.analysis_results = data;
                        self.apply_filter();
                    }
                    Err(e) => self.error = Some(e),
                }
                true
            }
            Msg::SetPeriod(period) => {
                self.period = period;
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::SetActiveTab(tab) => {
                self.active_tab = tab;
                self.page = 0;
                self.apply_filter();
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
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_period_change = ctx.link().callback(|e: Event| {
            let target = e.target_dyn_into::<HtmlSelectElement>();
            if let Some(select) = target {
                Msg::SetPeriod(select.value())
            } else {
                Msg::LoadData
            }
        });

        html! {
            <div class="financial-analysis-page">
                <PageHeader title={"财务分析".to_string()} subtitle={Some("杜邦分析与财务比率分析".to_string())}>
                    <select class="period-select" value={self.period.clone()} onchange={on_period_change}>
                        <option value="2026-01">{"2026年1月"}</option>
                        <option value="2026-02">{"2026年2月"}</option>
                        <option value="2026-03">{"2026年3月"}</option>
                        <option value="2026-04">{"2026年4月"}</option>
                    </select>
                    <button class="btn btn-primary" onclick={ctx.link().callback(|_| Msg::LoadData)}>
                        {"刷新"}
                    </button>
                </PageHeader>

                {self.render_tabs(ctx)}
                {self.render_content(ctx)}
            </div>
        }
    }
}

impl FinancialAnalysisPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_ratios = self.ratios.clone();
            self.filtered_indicators = self.indicators.clone();
            self.filtered_results = self.analysis_results.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_ratios = self
                .ratios
                .iter()
                .filter(|r| {
                    r.indicator_name.to_lowercase().contains(&keyword)
                        || r.indicator_code.to_lowercase().contains(&keyword)
                        || r.analysis_result.to_lowercase().contains(&keyword)
                        || r.ratio_level.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
            self.filtered_indicators = self
                .indicators
                .iter()
                .filter(|i| {
                    i.indicator_name.to_lowercase().contains(&keyword)
                        || i.indicator_code.to_lowercase().contains(&keyword)
                        || i.indicator_type.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
            self.filtered_results = self
                .analysis_results
                .iter()
                .filter(|r| {
                    r.result_type.to_lowercase().contains(&keyword)
                        || r.conclusion.to_lowercase().contains(&keyword)
                        || r.recommendation.to_lowercase().contains(&keyword)
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
            ("dupont", "杜邦分析"),
            ("ratio", "财务比率"),
            ("indicator", "财务指标"),
            ("result", "分析结果"),
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
                <LoadingState message={"正在加载财务分析数据...".to_string()} />
            };
        }

        if let Some(error) = &self.error {
            return html! {
                <div class="error-container">
                    <div class="error-icon">{"⚠️"}</div>
                    <p class="error-message">{error}</p>
                    <button class="btn btn-primary" onclick={ctx.link().callback(|_| Msg::LoadData)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        match self.active_tab.as_str() {
            "dupont" => self.render_dupont(ctx),
            "ratio" => self.render_ratios(ctx),
            "indicator" => self.render_indicators(ctx),
            "result" => self.render_results(ctx),
            _ => html! {},
        }
    }

    fn render_dupont(&self, _ctx: &Context<Self>) -> Html {
        if let Some(dupont) = &self.dupont_data {
            html! {
                <>
                    <div class="card">
                        <div class="card-header">
                            <h2>{format!("杜邦分析树 ({})", &dupont.period)}</h2>
                        </div>
                        <div class="card-body">
                            <div class="metrics-grid">
                                <div class="metric-card">
                                    <div class="metric-icon">{"💹"}</div>
                                    <div class="metric-content">
                                        <div class="metric-title">{"净资产收益率 (ROE)"}</div>
                                        <div class="metric-value">{&dupont.roe}</div>
                                        <div class="metric-description">{"综合反映盈利能力"}</div>
                                    </div>
                                </div>
                                <div class="metric-card">
                                    <div class="metric-icon">{"💰"}</div>
                                    <div class="metric-content">
                                        <div class="metric-title">{"销售净利率"}</div>
                                        <div class="metric-value">{&dupont.net_profit_margin}</div>
                                        <div class="metric-description">{"盈利水平"}</div>
                                    </div>
                                </div>
                                <div class="metric-card">
                                    <div class="metric-icon">{"🔄"}</div>
                                    <div class="metric-content">
                                        <div class="metric-title">{"总资产周转率"}</div>
                                        <div class="metric-value">{&dupont.asset_turnover}</div>
                                        <div class="metric-description">{"营运能力"}</div>
                                    </div>
                                </div>
                                <div class="metric-card">
                                    <div class="metric-icon">{"⚖️"}</div>
                                    <div class="metric-content">
                                        <div class="metric-title">{"权益乘数"}</div>
                                        <div class="metric-value">{&dupont.equity_multiplier}</div>
                                        <div class="metric-description">{"财务杠杆"}</div>
                                    </div>
                                </div>
                            </div>
                            <div class="analysis-summary">
                                <strong>{"分析总结："}</strong>
                                <p>{&dupont.analysis_summary}</p>
                            </div>
                        </div>
                    </div>
                </>
            }
        } else {
            html! {
                <EmptyState
                    icon={"📊".to_string()}
                    title={"暂无杜邦分析数据".to_string()}
                    description={"请选择其他期间或刷新数据".to_string()}
                />
            }
        }
    }

    fn render_ratios(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索指标名称、编码或分析结果...".to_string()}
                        on_search={ctx.link().callback(|keyword| Msg::Search(keyword))}
                        on_reset={ctx.link().callback(|_| Msg::ResetSearch)}
                    />
                </div>
                if self.filtered_ratios.is_empty() {
                    <EmptyState
                        icon={"📈".to_string()}
                        title={"暂无财务比率数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "当前期间暂无财务比率数据".to_string()
                        } else {
                            "没有匹配搜索条件的比率数据".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"指标名称"}</th>
                                    <th>{"当前值"}</th>
                                    <th>{"行业平均"}</th>
                                    <th>{"水平"}</th>
                                    <th>{"分析结果"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_items(&self.filtered_ratios).iter().map(|ratio| {
                                    html! {
                                        <tr>
                                            <td>{&ratio.indicator_name}</td>
                                            <td class="numeric">{&ratio.indicator_value}</td>
                                            <td class="numeric">{&ratio.industry_average}</td>
                                            <td>
                                                <span class={format!("status-badge status-{}", match ratio.ratio_level.as_str() {
                                                    "优秀" => "success",
                                                    "良好" => "info",
                                                    "较差" => "warning",
                                                    "危险" => "danger",
                                                    _ => "default"
                                                })}>
                                                    {&ratio.ratio_level}
                                                </span>
                                            </td>
                                            <td>{&ratio.analysis_result}</td>
                                        </tr>
                                    }
                                })}
                            </tbody>
                        </table>
                        <Pagination
                            current_page={self.page}
                            page_size={self.page_size}
                            total={self.filtered_ratios.len() as u64}
                            on_page_change={ctx.link().callback(|page| Msg::PageChanged(page))}
                        />
                    </div>
                }
            </>
        }
    }

    fn render_indicators(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索指标名称、编码或类型...".to_string()}
                        on_search={ctx.link().callback(|keyword| Msg::Search(keyword))}
                        on_reset={ctx.link().callback(|_| Msg::ResetSearch)}
                    />
                </div>
                if self.filtered_indicators.is_empty() {
                    <EmptyState
                        icon={"📋".to_string()}
                        title={"暂无财务指标数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "当前暂无财务指标数据".to_string()
                        } else {
                            "没有匹配搜索条件的指标".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"指标编码"}</th>
                                    <th>{"指标名称"}</th>
                                    <th>{"指标类型"}</th>
                                    <th>{"计算公式"}</th>
                                    <th>{"单位"}</th>
                                    <th>{"基准值"}</th>
                                    <th>{"权重"}</th>
                                    <th>{"状态"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_items(&self.filtered_indicators).iter().map(|ind| {
                                    html! {
                                        <tr>
                                            <td>{&ind.indicator_code}</td>
                                            <td>{&ind.indicator_name}</td>
                                            <td>{&ind.indicator_type}</td>
                                            <td>{&ind.formula}</td>
                                            <td>{&ind.unit}</td>
                                            <td class="numeric">{ind.benchmark_value.clone().unwrap_or_else(|| "-".to_string())}</td>
                                            <td class="numeric">{ind.weight.clone().unwrap_or_else(|| "-".to_string())}</td>
                                            <td>
                                                <span class={format!("status-badge status-{}", match ind.status.as_str() {
                                                    "启用" => "success",
                                                    "禁用" => "danger",
                                                    _ => "default"
                                                })}>
                                                    {&ind.status}
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
                            total={self.filtered_indicators.len() as u64}
                            on_page_change={ctx.link().callback(|page| Msg::PageChanged(page))}
                        />
                    </div>
                }
            </>
        }
    }

    fn render_results(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索分析类型、结论或建议...".to_string()}
                        on_search={ctx.link().callback(|keyword| Msg::Search(keyword))}
                        on_reset={ctx.link().callback(|_| Msg::ResetSearch)}
                    />
                </div>
                if self.filtered_results.is_empty() {
                    <EmptyState
                        icon={"📝".to_string()}
                        title={"暂无分析结果数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "当前期间暂无分析结果".to_string()
                        } else {
                            "没有匹配搜索条件的结果".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"ID"}</th>
                                    <th>{"分析类型"}</th>
                                    <th>{"期间"}</th>
                                    <th>{"数据"}</th>
                                    <th>{"结论"}</th>
                                    <th>{"建议"}</th>
                                    <th>{"创建人"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_items(&self.filtered_results).iter().map(|r| {
                                    html! {
                                        <tr>
                                            <td>{r.id}</td>
                                            <td>{&r.result_type}</td>
                                            <td>{&r.period}</td>
                                            <td>{&r.data}</td>
                                            <td>{&r.conclusion}</td>
                                            <td>{&r.recommendation}</td>
                                            <td>{r.created_by}</td>
                                        </tr>
                                    }
                                })}
                            </tbody>
                        </table>
                        <Pagination
                            current_page={self.page}
                            page_size={self.page_size}
                            total={self.filtered_results.len() as u64}
                            on_page_change={ctx.link().callback(|page| Msg::PageChanged(page))}
                        />
                    </div>
                }
            </>
        }
    }
}
