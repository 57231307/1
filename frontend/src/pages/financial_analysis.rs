//! 财务分析页面

use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::models::financial_analysis::{DupontAnalysis, FinancialRatio};
use crate::services::financial_analysis_service::FinancialAnalysisService;

pub struct FinancialAnalysisPage {
    loading: bool,
    error: Option<String>,
    dupont_data: Option<DupontAnalysis>,
    ratios: Vec<FinancialRatio>,
    period: String,
}

pub enum Msg {
    LoadData,
    DataLoaded(DupontAnalysis, Vec<FinancialRatio>),
    LoadError(String),
    SetPeriod(String),
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
            period: "2026-03".to_string(), // 默认当前期间
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
                    let ratios_res = FinancialAnalysisService::analyze_ratios(&period).await;
                    match (dupont_res, ratios_res) {
                        (Ok(dupont), Ok(ratios)) => {
                            link.send_message(Msg::DataLoaded(dupont, ratios));
                        }
                        (Err(e), _) | (_, Err(e)) => {
                            link.send_message(Msg::LoadError(e));
                        }
                    }
                });
                false
            }
            Msg::DataLoaded(dupont, ratios) => {
                self.dupont_data = Some(dupont);
                self.ratios = ratios;
                self.loading = false;
                true
            }
            Msg::LoadError(e) => {
                self.error = Some(e);
                self.loading = false;
                true
            }
            Msg::SetPeriod(p) => {
                self.period = p;
                ctx.link().send_message(Msg::LoadData);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_period_change = ctx.link().callback(|e: Event| {
            let target = e.target_dyn_into::<web_sys::HtmlSelectElement>();
            if let Some(select) = target {
                Msg::SetPeriod(select.value())
            } else {
                Msg::LoadData
            }
        });

        html! {
            <div class="financial-analysis-page">
                <div class="page-header">
                    <h1>{"📈 财务分析仪表板"}</h1>
                    <div class="header-actions">
                        <select class="period-select" value={self.period.clone()} onchange={on_period_change}>
                            <option value="2026-01">{"2026年1月"}</option>
                            <option value="2026-02">{"2026年2月"}</option>
                            <option value="2026-03">{"2026年3月"}</option>
                        </select>
                        <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadData)}>
                            {"刷新"}
                        </button>
                    </div>
                </div>

                {self.render_content()}
            </div>
        }
    }
}

impl FinancialAnalysisPage {
    fn render_content(&self) -> Html {
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
                </div>
            };
        }

        html! {
            <>
                {self.render_dupont()}
                {self.render_ratios()}
            </>
        }
    }

    fn render_dupont(&self) -> Html {
        if let Some(dupont) = &self.dupont_data {
            html! {
                <div class="card mb-4">
                    <div class="card-header">
                        <h2>{"杜邦分析树 ("}{&dupont.period}{")"}</h2>
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
                        <div class="analysis-summary mt-3 p-3 bg-light rounded">
                            <strong>{"分析总结："}</strong>
                            <p>{&dupont.analysis_summary}</p>
                        </div>
                    </div>
                </div>
            }
        } else {
            html! {}
        }
    }

    fn render_ratios(&self) -> Html {
        if self.ratios.is_empty() {
            return html! {
                <div class="empty-state">
                    <p>{"暂无财务比率数据"}</p>
                </div>
            };
        }

        html! {
            <div class="card">
                <div class="card-header">
                    <h2>{"关键财务比率"}</h2>
                </div>
                <div class="card-body">
                    <div class="table-responsive">
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
                                {for self.ratios.iter().map(|ratio| {
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
                    </div>
                </div>
            </div>
        }
    }
}
