use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use chrono::{Datelike, Timelike};
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;
use crate::services::dashboard_service::{DashboardService, DashboardOverview, LowStockAlert, SalesStatistics, InventoryStatistics};
use crate::services::crud_service::CrudService;
use crate::services::auth::AuthService;
use crate::models::auth::TotpSetupResponse;

pub struct DashboardPage {
    overview: Option<DashboardOverview>,
    low_stock_alerts: Vec<LowStockAlert>,
    sales_trend: Option<SalesStatistics>,
    inventory_status: Option<InventoryStatistics>,
    loading: bool,
    error: Option<String>,
    auto_refresh: bool,
    
    // 2FA state
    show_2fa_modal: bool,
    totp_setup_data: Option<TotpSetupResponse>,
    totp_verify_code: String,
    totp_error: Option<String>,
    totp_success: bool,
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
    
    // 2FA messages
    Open2FAModal,
    Close2FAModal,
    TotpSetupDataLoaded(TotpSetupResponse),
    TotpVerifyCodeChanged(String),
    VerifyAndEnable2FA,
    TotpEnabledSuccess,
    TotpError(String),
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
            show_2fa_modal: false,
            totp_setup_data: None,
            totp_verify_code: String::new(),
            totp_error: None,
            totp_success: false,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadData | Msg::RefreshData => {
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
                    let sales_trend_result = DashboardService::get_sales_statistics("2026-01-01", "2026-03-31").await;
                    let inventory_status_result = DashboardService::get_inventory_statistics().await;
                    
                    match (overview_result, alerts_result, sales_trend_result, inventory_status_result) {
                        (Ok(overview), Ok(alerts), Ok(sales_trend), Ok(inventory_status)) => {
                            link.send_message(Msg::DataLoaded { 
                                overview, 
                                low_stock_alerts: alerts,
                                sales_trend,
                                inventory_status,
                            });
                        }
                        (Err(e), _, _, _) | (_, Err(e), _, _) | (_, _, Err(e), _) | (_, _, _, Err(e)) => {
                            link.send_message(Msg::Error(e));
                        }
                    }
                });
                false
            }
            Msg::DataLoaded { overview, low_stock_alerts, sales_trend, inventory_status } => {
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
            Msg::Open2FAModal => {
                self.show_2fa_modal = true;
                self.totp_error = None;
                self.totp_success = false;
                self.totp_verify_code = String::new();
                
                let link = _ctx.link().clone();
                spawn_local(async move {
                    let auth_service = AuthService::new();
                    match auth_service.setup_totp().await {
                        Ok(data) => link.send_message(Msg::TotpSetupDataLoaded(data)),
                        Err(e) => link.send_message(Msg::TotpError(e)),
                    }
                });
                true
            }
            Msg::Close2FAModal => {
                self.show_2fa_modal = false;
                true
            }
            Msg::TotpSetupDataLoaded(data) => {
                self.totp_setup_data = Some(data);
                true
            }
            Msg::TotpVerifyCodeChanged(code) => {
                self.totp_verify_code = code;
                self.totp_error = None;
                true
            }
            Msg::VerifyAndEnable2FA => {
                if self.totp_verify_code.len() != 6 {
                    self.totp_error = Some("请输入6位验证码".to_string());
                    return true;
                }
                
                let link = _ctx.link().clone();
                let code = self.totp_verify_code.clone();
                spawn_local(async move {
                    let auth_service = AuthService::new();
                    match auth_service.enable_totp(&code).await {
                        Ok(_) => link.send_message(Msg::TotpEnabledSuccess),
                        Err(e) => link.send_message(Msg::TotpError(e)),
                    }
                });
                true
            }
            Msg::TotpEnabledSuccess => {
                self.totp_success = true;
                self.totp_error = None;
                true
            }
            Msg::TotpError(e) => {
                self.totp_error = Some(e);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_totp_code_change = ctx.link().batch_callback(|e: Event| {
            let target = e.target()?.dyn_into::<HtmlInputElement>().ok()?;
            Some(Msg::TotpVerifyCodeChanged(target.value()))
        });

        html! {
            <div class="dashboard-page">
                <div class="dashboard-header">
                    <div class="header-left">
                        <h1>{"📊 管理仪表板"}</h1>
                        <p class="subtitle">{"欢迎使用秉羲面料管理"}</p>
                    </div>
                    <div class="header-right">
                        <button class="btn-primary" style="margin-right: 10px;" onclick={ctx.link().callback(|_| Msg::Open2FAModal)}>
                            {"🔐 开启两步验证"}
                        </button>
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

                if self.show_2fa_modal {
                    <div class="modal-overlay">
                        <div class="modal-content" style="max-width: 500px;">
                            <div class="modal-header">
                                <h2>{"开启两步验证 (2FA)"}</h2>
                                <button class="close-btn" onclick={ctx.link().callback(|_| Msg::Close2FAModal)}>{"×"}</button>
                            </div>
                            <div class="modal-body">
                                if self.totp_success {
                                    <div class="success-message" style="color: green; text-align: center; padding: 20px;">
                                        <h3>{"🎉 设置成功！"}</h3>
                                        <p>{"您的账户已开启两步验证。下次登录时需要输入验证码。"}</p>
                                    </div>
                                    <div class="form-actions" style="justify-content: center;">
                                        <button type="button" class="btn-primary" onclick={ctx.link().callback(|_| Msg::Close2FAModal)}>{"完成"}</button>
                                    </div>
                                } else {
                                    if let Some(data) = &self.totp_setup_data {
                                        <div class="totp-setup-container" style="text-align: center;">
                                            <p>{"1. 请使用 Google Authenticator 或兼容应用扫描下方二维码："}</p>
                                            <img src={data.qr_code.clone()} alt="TOTP QR Code" style="margin: 20px auto; border: 1px solid #ddd; padding: 10px; border-radius: 8px; width: 200px;" />
                                            
                                            <p style="margin-bottom: 20px;">{"如果无法扫描，请手动输入密钥："}<br/><code style="background: #f5f5f5; padding: 4px 8px; border-radius: 4px; user-select: all;">{data.secret.clone()}</code></p>
                                            
                                            <p>{"2. 输入应用中显示的 6 位验证码以完成设置："}</p>
                                            <input 
                                                type="text" 
                                                value={self.totp_verify_code.clone()} 
                                                onchange={on_totp_code_change}
                                                placeholder="输入 6 位验证码" 
                                                maxlength="6"
                                                style="padding: 10px; font-size: 16px; width: 200px; text-align: center; margin-bottom: 10px;"
                                            />
                                            
                                            if let Some(err) = &self.totp_error {
                                                <div style="color: red; margin-bottom: 10px;">{err}</div>
                                            }
                                        </div>
                                        <div class="form-actions">
                                            <button type="button" class="btn-secondary" onclick={ctx.link().callback(|_| Msg::Close2FAModal)}>{"取消"}</button>
                                            <button type="button" class="btn-primary" onclick={ctx.link().callback(|_| Msg::VerifyAndEnable2FA)}>{"验证并开启"}</button>
                                        </div>
                                    } else if let Some(err) = &self.totp_error {
                                        <div style="color: red; padding: 20px; text-align: center;">
                                            <p>{"获取两步验证信息失败："}</p>
                                            <p>{err}</p>
                                        </div>
                                        <div class="form-actions">
                                            <button type="button" class="btn-secondary" onclick={ctx.link().callback(|_| Msg::Close2FAModal)}>{"关闭"}</button>
                                        </div>
                                    } else {
                                        <div style="padding: 40px; text-align: center;">{"加载中..."}</div>
                                    }
                                }
                            </div>
                        </div>
                    </div>
                }
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
            if sales_data.daily_sales.is_empty() {
                return html! {
                    <div class="empty-state">
                        <div class="empty-icon">{"📈"}</div>
                        <p>{"暂无销售数据"}</p>
                    </div>
                };
            }

            // 生成销售趋势图表（这里使用简单的HTML表示，实际项目中可以使用Chart.js等库）
            html! {
                <div class="chart-container">
                    <div class="chart-content">
                        <div class="sales-trend-chart">
                            {for sales_data.daily_sales.iter().map(|trend| {
                                html! {
                                    <div class="chart-bar">
                                        <div class="bar-label">{&trend.date}</div>
                                        <div class="bar-container">
                                            <div 
                                                class="bar-fill" 
                                                style={format!("width: {}%", trend.amount.parse::<f64>().unwrap_or(0.0) / 10000.0 * 100.0)}
                                            ></div>
                                        </div>
                                        <div class="bar-value">{&trend.amount}</div>
                                    </div>
                                }
                            })}
                        </div>
                    </div>
                </div>
            }
        } else {
            html! {
                <div class="empty-state">
                    <div class="empty-icon">{"📈"}</div>
                    <p>{"暂无销售数据"}</p>
                </div>
            }
        }
    }

    fn render_inventory_chart(&self) -> Html {
        if let Some(inventory_data) = &self.inventory_status {
            if inventory_data.by_warehouse.is_empty() {
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
                        {for inventory_data.by_warehouse.iter().map(|warehouse| {
                            html! {
                                <div class="inventory-item">
                                    <div class="inventory-label">{&warehouse.warehouse_name}</div>
                                    <div class="inventory-progress">
                                        <div 
                                            class="inventory-progress-bar" 
                                            style={format!("width: 50%")}
                                        ></div>
                                    </div>
                                    <div class="inventory-value">{&warehouse.quantity}</div>
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
