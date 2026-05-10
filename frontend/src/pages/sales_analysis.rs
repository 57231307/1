use crate::components::{
    empty_state::EmptyState,
    loading_state::LoadingState,
    page_header::PageHeader,
    pagination::Pagination,
    search_bar::SearchBar,
};
use crate::models::sales_analysis::{
    CustomerRanking, ProductRanking, SalesTarget, SalesTrendAnalysis,
};
use crate::services::crud_service::CrudService;
use crate::services::sales_analysis_service::SalesAnalysisService;
use crate::utils::permissions;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

pub struct SalesAnalysisPage {
    trend: Option<SalesTrendAnalysis>,
    product_rankings: Vec<ProductRanking>,
    customer_rankings: Vec<CustomerRanking>,
    targets: Vec<SalesTarget>,
    loading: bool,
    error: Option<String>,
    active_tab: String,
    period: String,
    search_keyword: String,
    page: u64,
    page_size: u64,
    filtered_products: Vec<ProductRanking>,
    filtered_customers: Vec<CustomerRanking>,
    filtered_targets: Vec<SalesTarget>,
}

pub enum Msg {
    LoadData,
    TrendLoaded(Result<SalesTrendAnalysis, String>),
    ProductRankingLoaded(Result<Vec<ProductRanking>, String>),
    CustomerRankingLoaded(Result<Vec<CustomerRanking>, String>),
    TargetsLoaded(Result<Vec<SalesTarget>, String>),
    SetPeriod(String),
    SetActiveTab(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
}

impl Component for SalesAnalysisPage {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::LoadData);
        Self {
            trend: None,
            product_rankings: Vec::new(),
            customer_rankings: Vec::new(),
            targets: Vec::new(),
            loading: true,
            error: None,
            active_tab: "overview".to_string(),
            period: "2026-04".to_string(),
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
            filtered_products: Vec::new(),
            filtered_customers: Vec::new(),
            filtered_targets: Vec::new(),
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
                    let trend_res = SalesAnalysisService::get_trend_analysis(
                        "MONTH",
                        &format!("{}-01", period),
                        &format!("{}-30", period),
                        None,
                        None,
                    )
                    .await;
                    link.send_message(Msg::TrendLoaded(trend_res));
                });

                let link2 = ctx.link().clone();
                let period2 = self.period.clone();
                spawn_local(async move {
                    let res = SalesAnalysisService::get_product_ranking(
                        Some("MONTH"),
                        Some(&format!("{}-01", period2)),
                        Some(&format!("{}-30", period2)),
                        None,
                        50,
                    )
                    .await;
                    link2.send_message(Msg::ProductRankingLoaded(res));
                });

                let link3 = ctx.link().clone();
                let period3 = self.period.clone();
                spawn_local(async move {
                    let res = SalesAnalysisService::get_customer_ranking(
                        Some("MONTH"),
                        Some(&format!("{}-01", period3)),
                        Some(&format!("{}-30", period3)),
                        None,
                        50,
                    )
                    .await;
                    link3.send_message(Msg::CustomerRankingLoaded(res));
                });

                let link4 = ctx.link().clone();
                let period4 = self.period.clone();
                spawn_local(async move {
                    let res = SalesAnalysisService::list_targets(None, None, 0, 100).await;
                    link4.send_message(Msg::TargetsLoaded(res));
                });
                false
            }
            Msg::TrendLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(data) => self.trend = Some(data),
                    Err(e) => self.error = Some(e),
                }
                true
            }
            Msg::ProductRankingLoaded(result) => {
                match result {
                    Ok(data) => {
                        self.product_rankings = data;
                        self.apply_filter();
                    }
                    Err(e) => self.error = Some(e),
                }
                true
            }
            Msg::CustomerRankingLoaded(result) => {
                match result {
                    Ok(data) => {
                        self.customer_rankings = data;
                        self.apply_filter();
                    }
                    Err(e) => self.error = Some(e),
                }
                true
            }
            Msg::TargetsLoaded(result) => {
                match result {
                    Ok(data) => {
                        self.targets = data;
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
            <div class="sales-analysis-page">
                <PageHeader title={"销售分析".to_string()} subtitle={Some("销售趋势、排名与目标分析".to_string())}>
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

impl SalesAnalysisPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_products = self.product_rankings.clone();
            self.filtered_customers = self.customer_rankings.clone();
            self.filtered_targets = self.targets.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_products = self
                .product_rankings
                .iter()
                .filter(|p| {
                    p.product_name
                        .as_ref()
                        .map(|n| n.to_lowercase().contains(&keyword))
                        .unwrap_or(false)
                        || p
                            .product_code
                            .as_ref()
                            .map(|c| c.to_lowercase().contains(&keyword))
                            .unwrap_or(false)
                })
                .cloned()
                .collect();
            self.filtered_customers = self
                .customer_rankings
                .iter()
                .filter(|c| {
                    c.customer_name
                        .as_ref()
                        .map(|n| n.to_lowercase().contains(&keyword))
                        .unwrap_or(false)
                        || c.customer_type.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
            self.filtered_targets = self
                .targets
                .iter()
                .filter(|t| {
                    t.target_type.to_lowercase().contains(&keyword)
                        || t.period.to_lowercase().contains(&keyword)
                        || t.status.to_lowercase().contains(&keyword)
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
            ("overview", "总览"),
            ("product", "产品排名"),
            ("customer", "客户排名"),
            ("target", "销售目标"),
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
                <LoadingState message={"正在加载销售分析数据...".to_string()} />
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
            "overview" => self.render_overview(ctx),
            "product" => self.render_product_ranking(ctx),
            "customer" => self.render_customer_ranking(ctx),
            "target" => self.render_targets(ctx),
            _ => html! {},
        }
    }

    fn render_overview(&self, _ctx: &Context<Self>) -> Html {
        if let Some(data) = &self.trend {
            html! {
                <>
                    <div class="dashboard-grid">
                        <div class="card metric-card">
                            <h3>{"总销售额"}</h3>
                            <div class="metric-value">{format!("¥{}", data.total_sales_amount)}</div>
                            <div class={if data.trend_direction == "UP" { "trend-up" } else { "trend-down" }}>
                                {format!("增长率: {} {}", data.growth_rate, if data.trend_direction == "UP" { "↑" } else { "↓" })}
                            </div>
                        </div>
                        <div class="card metric-card">
                            <h3>{"总销量"}</h3>
                            <div class="metric-value">{format!("{}", data.total_sales_quantity)}</div>
                            <div class="metric-sub">{format!("日均销量: {}", data.average_daily_sales)}</div>
                        </div>
                        <div class="card metric-card">
                            <h3>{"分析周期"}</h3>
                            <div class="metric-period">{format!("{} 至 {}", data.start_date, data.end_date)}</div>
                            <div class="metric-sub">{format!("周期类型: {}", data.period)}</div>
                        </div>
                    </div>
                    <div class="card">
                        <div class="card-header">
                            <h3>{"近期销售极值记录"}</h3>
                        </div>
                        <div class="card-body">
                            <ul class="extreme-list">
                                <li>
                                    {"最高峰日期: "}
                                    <strong class="peak">{data.peak_date.clone().unwrap_or_else(|| "暂无数据".to_string())}</strong>
                                </li>
                                <li>
                                    {"最低谷日期: "}
                                    <strong class="low">{data.lowest_date.clone().unwrap_or_else(|| "暂无数据".to_string())}</strong>
                                </li>
                            </ul>
                        </div>
                    </div>
                </>
            }
        } else {
            html! {
                <EmptyState
                    icon={"📊".to_string()}
                    title={"暂无销售趋势数据".to_string()}
                    description={"请选择其他期间或刷新数据".to_string()}
                />
            }
        }
    }

    fn render_product_ranking(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索产品名称或编码...".to_string()}
                        on_search={ctx.link().callback(|keyword| Msg::Search(keyword))}
                        on_reset={ctx.link().callback(|_| Msg::ResetSearch)}
                    />
                </div>
                if self.filtered_products.is_empty() {
                    <EmptyState
                        icon={"📦".to_string()}
                        title={"暂无产品排名数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "当前期间暂无产品销售数据".to_string()
                        } else {
                            "没有匹配搜索条件的产品".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"排名"}</th>
                                    <th>{"产品编码"}</th>
                                    <th>{"产品名称"}</th>
                                    <th>{"销售金额"}</th>
                                    <th>{"销售数量"}</th>
                                    <th>{"毛利"}</th>
                                    <th>{"毛利率"}</th>
                                    <th>{"客户数"}</th>
                                    <th>{"订单数"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_items(&self.filtered_products).iter().map(|p| {
                                    html! {
                                        <tr>
                                            <td>{p.rank}</td>
                                            <td>{p.product_code.clone().unwrap_or_else(|| "-".to_string())}</td>
                                            <td>{p.product_name.clone().unwrap_or_else(|| "-".to_string())}</td>
                                            <td class="numeric">{&p.total_sales_amount}</td>
                                            <td class="numeric">{p.total_sales_quantity}</td>
                                            <td class="numeric">{&p.gross_profit}</td>
                                            <td class="numeric">{&p.gross_margin}</td>
                                            <td class="numeric">{p.customer_count}</td>
                                            <td class="numeric">{p.order_count}</td>
                                        </tr>
                                    }
                                })}
                            </tbody>
                        </table>
                        <Pagination
                            current_page={self.page}
                            page_size={self.page_size}
                            total={self.filtered_products.len() as u64}
                            on_page_change={ctx.link().callback(|page| Msg::PageChanged(page))}
                        />
                    </div>
                }
            </>
        }
    }

    fn render_customer_ranking(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索客户名称或类型...".to_string()}
                        on_search={ctx.link().callback(|keyword| Msg::Search(keyword))}
                        on_reset={ctx.link().callback(|_| Msg::ResetSearch)}
                    />
                </div>
                if self.filtered_customers.is_empty() {
                    <EmptyState
                        icon={"👥".to_string()}
                        title={"暂无客户排名数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "当前期间暂无客户销售数据".to_string()
                        } else {
                            "没有匹配搜索条件的客户".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"排名"}</th>
                                    <th>{"客户名称"}</th>
                                    <th>{"客户类型"}</th>
                                    <th>{"销售金额"}</th>
                                    <th>{"销售数量"}</th>
                                    <th>{"毛利"}</th>
                                    <th>{"订单数"}</th>
                                    <th>{"平均订单金额"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_items(&self.filtered_customers).iter().map(|c| {
                                    html! {
                                        <tr>
                                            <td>{c.rank}</td>
                                            <td>{c.customer_name.clone().unwrap_or_else(|| "-".to_string())}</td>
                                            <td>{&c.customer_type}</td>
                                            <td class="numeric">{&c.total_sales_amount}</td>
                                            <td class="numeric">{c.total_sales_quantity}</td>
                                            <td class="numeric">{&c.gross_profit}</td>
                                            <td class="numeric">{c.order_count}</td>
                                            <td class="numeric">{&c.average_order_value}</td>
                                        </tr>
                                    }
                                })}
                            </tbody>
                        </table>
                        <Pagination
                            current_page={self.page}
                            page_size={self.page_size}
                            total={self.filtered_customers.len() as u64}
                            on_page_change={ctx.link().callback(|page| Msg::PageChanged(page))}
                        />
                    </div>
                }
            </>
        }
    }

    fn render_targets(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索目标类型、期间或状态...".to_string()}
                        on_search={ctx.link().callback(|keyword| Msg::Search(keyword))}
                        on_reset={ctx.link().callback(|_| Msg::ResetSearch)}
                    />
                </div>
                if self.filtered_targets.is_empty() {
                    <EmptyState
                        icon={"🎯".to_string()}
                        title={"暂无销售目标数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "当前暂无销售目标数据".to_string()
                        } else {
                            "没有匹配搜索条件的目标".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"ID"}</th>
                                    <th>{"目标类型"}</th>
                                    <th>{"期间"}</th>
                                    <th>{"目标金额"}</th>
                                    <th>{"实际金额"}</th>
                                    <th>{"完成率"}</th>
                                    <th>{"状态"}</th>
                                    <th>{"开始日期"}</th>
                                    <th>{"结束日期"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_items(&self.filtered_targets).iter().map(|t| {
                                    html! {
                                        <tr>
                                            <td>{t.id}</td>
                                            <td>{&t.target_type}</td>
                                            <td>{&t.period}</td>
                                            <td class="numeric">{&t.target_amount}</td>
                                            <td class="numeric">{&t.actual_amount}</td>
                                            <td class="numeric">{&t.completion_rate}</td>
                                            <td>
                                                <span class={format!("status-badge status-{}", match t.status.as_str() {
                                                    "已完成" => "success",
                                                    "进行中" => "info",
                                                    "未开始" => "default",
                                                    _ => "default"
                                                })}>
                                                    {&t.status}
                                                </span>
                                            </td>
                                            <td>{&t.start_date}</td>
                                            <td>{&t.end_date}</td>
                                        </tr>
                                    }
                                })}
                            </tbody>
                        </table>
                        <Pagination
                            current_page={self.page}
                            page_size={self.page_size}
                            total={self.filtered_targets.len() as u64}
                            on_page_change={ctx.link().callback(|page| Msg::PageChanged(page))}
                        />
                    </div>
                }
            </>
        }
    }
}
