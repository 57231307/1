use crate::models::dashboard::{
    DashboardOverview, InventoryStatistics, LowStockAlert, SalesStatistics,
};
use crate::services::dashboard_service::DashboardService;
use chrono::{Datelike, Timelike};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

pub struct DashboardPage {
    overview: Option<DashboardOverview>,
    low_stock_alerts: Vec<LowStockAlert>,
    sales_trend: Option<SalesStatistics>,
    inventory_status: Option<InventoryStatistics>,
    loading: bool,
    error: Option<String>,
    auto_refresh: bool,
}

pub enum Msg {
    LoadData,
    DataLoaded {
        overview: DashboardOverview,
        low_stock_alerts: Vec<LowStockAlert>,
        sales_trend: SalesStatistics,
        inventory_status: InventoryStatistics,
    },
    Error(String),
    ToggleAutoRefresh,
    RefreshData,
}

impl Component for DashboardPage {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::LoadData);
        Self {
            overview: None,
            low_stock_alerts: Vec::new(),
            sales_trend: None,
            inventory_status: None,
            loading: true,
            error: None,
            auto_refresh: false,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadData | Msg::RefreshData => {
                self.loading = true;
                let link = _ctx.link().clone();
                spawn_local(async move {
                    let now = chrono::Utc::now();
                    let start_of_month = now
                        .with_day(1)
                        .unwrap_or(now)
                        .with_hour(0)
                        .unwrap_or(now)
                        .with_minute(0)
                        .unwrap_or(now);

                    let overview_result = DashboardService::get_overview(
                        &start_of_month.to_rfc3339(),
                        &now.to_rfc3339(),
                    )
                    .await;

                    let alerts_result = DashboardService::get_low_stock_alerts().await;
                    let sales_trend_result =
                        DashboardService::get_sales_statistics("2026-01-01", "2026-03-31").await;
                    let inventory_status_result =
                        DashboardService::get_inventory_statistics().await;

                    match (
                        overview_result,
                        alerts_result,
                        sales_trend_result,
                        inventory_status_result,
                    ) {
                        (Ok(overview), Ok(alerts), Ok(sales_trend), Ok(inventory_status)) => {
                            link.send_message(Msg::DataLoaded {
                                overview,
                                low_stock_alerts: alerts,
                                sales_trend,
                                inventory_status,
                            });
                        }
                        (Err(e), _, _, _)
                        | (_, Err(e), _, _)
                        | (_, _, Err(e), _)
                        | (_, _, _, Err(e)) => {
                            link.send_message(Msg::Error(e));
                        }
                    }
                });
                false
            }
            Msg::DataLoaded {
                overview,
                low_stock_alerts,
                sales_trend,
                inventory_status,
            } => {
                self.overview = Some(overview);
                self.low_stock_alerts = low_stock_alerts;
                self.sales_trend = Some(sales_trend);
                self.inventory_status = Some(inventory_status);
                self.loading = false;
                true
            }
            Msg::Error(e) => {
                self.error = Some(e);
                self.loading = false;
                true
            }
            Msg::ToggleAutoRefresh => {
                self.auto_refresh = !self.auto_refresh;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="dashboard-page">
                <div class="dashboard-header">
                    <div class="header-left">
                        <h1>{"📊 管理仪表板"}</h1>
                        <p class="subtitle">{"欢迎使用秉羲管理系统"}</p>
                    </div>
                    <div class="header-right">
                        <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::RefreshData)}>
                            {"🔄 刷新数据"}
                        </button>
                        <label class="toggle-switch">
                            <input
                                type="checkbox"
                                checked={self.auto_refresh}
                                onclick={ctx.link().callback(|_| Msg::ToggleAutoRefresh)}
                            />
                            <span class="toggle-slider"></span>
                        </label>
                        <span class="toggle-label">{"自动刷新"}</span>
                    </div>
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
                    {self.render_metric_card("💰", "库存总价值", &overview.total_inventory_value, "当前库存估值")}
                    {self.render_metric_card("📝", "订单总数", &overview.total_orders.to_string(), "所有订单")}
                    {self.render_metric_card("⏳", "待处理订单", &overview.pending_orders.to_string(), "等待处理")}
                    {self.render_metric_card("👥", "活跃用户", &overview.active_users.to_string(), "最近7天登录")}
                </div>

                // 图表区域
                <div class="charts-grid">
                    // 销售趋势图表
                    <div class="card chart-card">
                        <div class="card-header">
                            <h2>{"📈 销售趋势"}</h2>
                        </div>
                        <div class="card-body">
                            {self.render_sales_chart()}
                        </div>
                    </div>

                    // 库存状态图表
                    <div class="card chart-card">
                        <div class="card-header">
                            <h2>{"📊 库存状态"}</h2>
                        </div>
                        <div class="card-body">
                            {self.render_inventory_chart()}
                        </div>
                    </div>
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

    fn render_sales_chart(&self) -> Html {
        if let Some(sales_data) = &self.sales_trend {
            html! {
                <div class="chart-container">
                    <div class="sales-trend-chart">
                        <div class="metric-card">
                            <div class="metric-title">{"总销售额"}</div>
                            <div class="metric-value">{&sales_data.total_sales_amount}</div>
                        </div>
                        <div class="metric-card">
                            <div class="metric-title">{"订单数量"}</div>
                            <div class="metric-value">{sales_data.order_count}</div>
                        </div>
                        <div class="metric-card">
                            <div class="metric-title">{"平均客单价"}</div>
                            <div class="metric-value">{&sales_data.avg_order_amount}</div>
                        </div>
                    </div>
                </div>
            }
        } else {
            html! {
                <div class="skeleton-chart"></div>
            }
        }
    }

    fn render_inventory_chart(&self) -> Html {
        if let Some(inventory_data) = &self.inventory_status {
            if inventory_data.warehouse_distribution.is_empty() {
                return html! {
                    <div class="empty-state">
                        <div class="empty-icon">{"📊"}</div>
                        <p>{"暂无库存数据"}</p>
                    </div>
                };
            }

            // 生成库存状态图表
            html! {
                <div class="chart-container">
                    <div class="inventory-status-chart">
                        {for inventory_data.warehouse_distribution.iter().map(|warehouse| {
                            html! {
                                <div class="inventory-item">
                                    <div class="inventory-label">{&warehouse.warehouse_name}</div>
                                    <div class="inventory-progress">
                                        <div
                                            class="inventory-progress-bar"
                                            style={format!("width: 50%")}
                                        ></div>
                                    </div>
                                    <div class="inventory-value">{&warehouse.total_quantity}</div>
                                </div>
                            }
                        })}
                    </div>
                </div>
            }
        } else {
            html! {
                <div class="empty-state">
                    <div class="empty-icon">{"📊"}</div>
                    <p>{"暂无库存数据"}</p>
                </div>
            }
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
