use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use chrono::{Datelike, Timelike};
use crate::models::dashboard::{
    DashboardOverview, LowStockAlert,
};
use crate::services::dashboard_service::DashboardService;

pub struct DashboardPage {
    overview: Option<DashboardOverview>,
    low_stock_alerts: Vec<LowStockAlert>,
    loading: bool,
    error: Option<String>,
}

pub enum Msg {
    LoadData,
    DataLoaded {
        overview: DashboardOverview,
        low_stock_alerts: Vec<LowStockAlert>,
    },
    Error(String),
}

impl Component for DashboardPage {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::LoadData);
        Self {
            overview: None,
            low_stock_alerts: Vec::new(),
            loading: true,
            error: None,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadData => {
                self.loading = true;
                let link = _ctx.link().clone();
                spawn_local(async move {
                    let now = chrono::Utc::now();
                    let start_of_month = now.with_day(1).unwrap_or(now).with_hour(0).unwrap_or(now).with_minute(0).unwrap_or(now);
                    
                    let overview_result = DashboardService::get_overview(
                        &start_of_month.to_rfc3339(),
                        &now.to_rfc3339()
                    ).await;
                    
                    let alerts_result = DashboardService::get_low_stock_alerts().await;
                    
                    match (overview_result, alerts_result) {
                        (Ok(overview), Ok(alerts)) => {
                            link.send_message(Msg::DataLoaded { overview, low_stock_alerts: alerts });
                        }
                        (Err(e), _) | (_, Err(e)) => {
                            link.send_message(Msg::Error(e));
                        }
                    }
                });
                false
            }
            Msg::DataLoaded { overview, low_stock_alerts } => {
                self.overview = Some(overview);
                self.low_stock_alerts = low_stock_alerts;
                self.loading = false;
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
            <div class="dashboard-page">
                <div class="dashboard-header">
                    <h1>{"📊 管理仪表板"}</h1>
                    <p class="subtitle">{"欢迎使用秉羲管理系统"}</p>
                </div>

                {self.render_content(ctx)}
            </div>
        }
    }
}

impl DashboardPage {
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
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadData)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        let overview = match &self.overview {
            Some(ov) => ov,
            None => return html! { <div>{"数据加载失败"}</div> },
        };

        html! {
            <>
                // 关键指标卡片
                <div class="metrics-grid">
                    {self.render_metric_card("📦", "产品总数", &overview.total_products.to_string(), "所有产品")}
                    {self.render_metric_card("🏭", "仓库数量", &overview.total_warehouses.to_string(), "活跃仓库")}
                    {self.render_metric_card("📝", "订单总数", &overview.total_orders.to_string(), "所有订单")}
                    {self.render_metric_card("💰", "本月销售", &overview.monthly_sales, "本月销售额")}
                    {self.render_metric_card("⚠️", "低库存预警", &overview.low_stock_count.to_string(), "需要补货")}
                    {self.render_metric_card("⏳", "待处理订单", &overview.pending_orders.to_string(), "等待处理")}
                </div>

                // 低库存预警表格
                <div class="card">
                    <div class="card-header">
                        <h2>{"⚠️ 低库存预警"}</h2>
                        <span class="badge">{self.low_stock_alerts.len()}</span>
                    </div>
                    <div class="card-body">
                        {self.render_low_stock_table()}
                    </div>
                </div>
            </>
        }
    }

    fn render_metric_card(&self, icon: &str, title: &str, value: &str, description: &str) -> Html {
        html! {
            <div class="metric-card">
                <div class="metric-icon">{icon}</div>
                <div class="metric-content">
                    <div class="metric-title">{title}</div>
                    <div class="metric-value">{value}</div>
                    <div class="metric-description">{description}</div>
                </div>
            </div>
        }
    }

    fn render_low_stock_table(&self) -> Html {
        if self.low_stock_alerts.is_empty() {
            return html! {
                <div class="empty-state">
                    <div class="empty-icon">{"✅"}</div>
                    <p>{"暂无低库存预警"}</p>
                </div>
            };
        }

        html! {
            <div class="table-responsive">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>{"产品 ID"}</th>
                            <th>{"产品名称"}</th>
                            <th>{"仓库"}</th>
                            <th>{"当前库存"}</th>
                            <th>{"最低库存"}</th>
                            <th>{"短缺数量"}</th>
                            <th>{"状态"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for self.low_stock_alerts.iter().map(|alert| {
                            html! {
                                <tr>
                                    <td>{alert.product_id.to_string()}</td>
                                    <td>{&alert.product_name}</td>
                                    <td>{&alert.warehouse_name}</td>
                                    <td class="numeric">{&alert.current_quantity}</td>
                                    <td class="numeric">{&alert.min_stock}</td>
                                    <td class="numeric negative">{"-"}{&alert.shortage}</td>
                                    <td>
                                        <span class="status-badge status-warning">
                                            {"缺货"}
                                        </span>
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
