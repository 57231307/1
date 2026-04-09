//! 应付报表页面
//!
//! 应付报表（AP Report）展示页面，包含统计报表、日报、月报和账龄分析

use crate::components::main_layout::MainLayout;
use crate::models::ap_report::{
    ApAgingResponse, ApDailyResponse, ApMonthlyResponse, ApStatisticsResponse,
};
use crate::services::ap_report_service::ApReportService;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

/// 报表类型枚举
#[derive(Clone, PartialEq)]
pub enum ReportType {
    Statistics, // 统计报表
    Daily,      // 日报
    Monthly,    // 月报
    Aging,      // 账龄分析
}

/// 应付报表页面状态
pub struct ApReportPage {
    report_type: ReportType,
    loading: bool,
    error: Option<String>,
    // 统计报表数据
    statistics_data: Option<ApStatisticsResponse>,
    // 日报数据
    daily_data: Option<ApDailyResponse>,
    // 月报数据
    monthly_data: Option<ApMonthlyResponse>,
    // 账龄分析数据
    aging_data: Option<ApAgingResponse>,
    // 筛选参数
    supplier_id: Option<i32>,
    start_date: String,
    end_date: String,
    report_date: String,
    year: i32,
    month: u32,
}

pub enum Msg {
    SetReportType(ReportType),
    LoadStatisticsReport,
    LoadDailyReport,
    LoadMonthlyReport,
    LoadAgingReport,
    StatisticsLoaded(Result<ApStatisticsResponse, String>),
    DailyLoaded(Result<ApDailyResponse, String>),
    MonthlyLoaded(Result<ApMonthlyResponse, String>),
    AgingLoaded(Result<ApAgingResponse, String>),
    LoadError(String),
    SetSupplierId(Option<i32>),
    SetStartDate(String),
    SetEndDate(String),
    SetReportDate(String),
    SetYear(i32),
    SetMonth(u32),
    Refresh,
}

impl Component for ApReportPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let now = chrono::Local::now();
        Self {
            report_type: ReportType::Statistics,
            loading: true,
            error: None,
            statistics_data: None,
            daily_data: None,
            monthly_data: None,
            aging_data: None,
            supplier_id: None,
            start_date: now.format("%Y-01-01").to_string(),
            end_date: now.format("%Y-%m-%d").to_string(),
            report_date: now.format("%Y-%m-%d").to_string(),
            year: now.format("%Y").to_string().parse().unwrap_or(2024),
            month: now.format("%m").to_string().parse().unwrap_or(1),
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadStatisticsReport);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SetReportType(report_type) => {
                self.report_type = report_type;
                self.loading = true;
                self.error = None;
                match self.report_type {
                    ReportType::Statistics => ctx.link().send_message(Msg::LoadStatisticsReport),
                    ReportType::Daily => ctx.link().send_message(Msg::LoadDailyReport),
                    ReportType::Monthly => ctx.link().send_message(Msg::LoadMonthlyReport),
                    ReportType::Aging => ctx.link().send_message(Msg::LoadAgingReport),
                }
                false
            }
            Msg::LoadStatisticsReport => {
                self.loading = true;
                let supplier_id = self.supplier_id;
                let start_date = self.start_date.clone();
                let end_date = self.end_date.clone();
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApReportService::get_statistics_report(supplier_id, start_date, end_date)
                        .await
                    {
                        Ok(data) => link.send_message(Msg::StatisticsLoaded(Ok(data))),
                        Err(e) => link.send_message(Msg::StatisticsLoaded(Err(e))),
                    }
                });
                false
            }
            Msg::LoadDailyReport => {
                self.loading = true;
                let supplier_id = self.supplier_id;
                let report_date = self.report_date.clone();
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApReportService::get_daily_report(report_date, supplier_id).await {
                        Ok(data) => link.send_message(Msg::DailyLoaded(Ok(data))),
                        Err(e) => link.send_message(Msg::DailyLoaded(Err(e))),
                    }
                });
                false
            }
            Msg::LoadMonthlyReport => {
                self.loading = true;
                let supplier_id = self.supplier_id;
                let year = self.year;
                let month = self.month;
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApReportService::get_monthly_report(year, month, supplier_id).await {
                        Ok(data) => link.send_message(Msg::MonthlyLoaded(Ok(data))),
                        Err(e) => link.send_message(Msg::MonthlyLoaded(Err(e))),
                    }
                });
                false
            }
            Msg::LoadAgingReport => {
                self.loading = true;
                let supplier_id = self.supplier_id;
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ApReportService::get_aging_report(supplier_id).await {
                        Ok(data) => link.send_message(Msg::AgingLoaded(Ok(data))),
                        Err(e) => link.send_message(Msg::AgingLoaded(Err(e))),
                    }
                });
                false
            }
            Msg::StatisticsLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(data) => self.statistics_data = Some(data),
                    Err(e) => self.error = Some(e),
                }
                true
            }
            Msg::DailyLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(data) => self.daily_data = Some(data),
                    Err(e) => self.error = Some(e),
                }
                true
            }
            Msg::MonthlyLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(data) => self.monthly_data = Some(data),
                    Err(e) => self.error = Some(e),
                }
                true
            }
            Msg::AgingLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(data) => self.aging_data = Some(data),
                    Err(e) => self.error = Some(e),
                }
                true
            }
            Msg::LoadError(e) => {
                self.error = Some(e);
                self.loading = false;
                true
            }
            Msg::SetSupplierId(supplier_id) => {
                self.supplier_id = supplier_id;
                false
            }
            Msg::SetStartDate(date) => {
                self.start_date = date;
                false
            }
            Msg::SetEndDate(date) => {
                self.end_date = date;
                false
            }
            Msg::SetReportDate(date) => {
                self.report_date = date;
                false
            }
            Msg::SetYear(year) => {
                self.year = year;
                false
            }
            Msg::SetMonth(month) => {
                self.month = month;
                false
            }
            Msg::Refresh => {
                match self.report_type {
                    ReportType::Statistics => ctx.link().send_message(Msg::LoadStatisticsReport),
                    ReportType::Daily => ctx.link().send_message(Msg::LoadDailyReport),
                    ReportType::Monthly => ctx.link().send_message(Msg::LoadMonthlyReport),
                    ReportType::Aging => ctx.link().send_message(Msg::LoadAgingReport),
                }
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <MainLayout current_page={"ap_reports"}>
<div class="ap-report-page">
                <div class="page-header">
                    <h1>{"📊 应付报表"}</h1>
                </div>

                <div class="report-tabs">
                    <button
                        class={format!("tab-btn {}", if self.report_type == ReportType::Statistics { "active" } else { "" })}
                        onclick={ctx.link().callback(|_| Msg::SetReportType(ReportType::Statistics))}
                    >
                        {"统计报表"}
                    </button>
                    <button
                        class={format!("tab-btn {}", if self.report_type == ReportType::Daily { "active" } else { "" })}
                        onclick={ctx.link().callback(|_| Msg::SetReportType(ReportType::Daily))}
                    >
                        {"日报"}
                    </button>
                    <button
                        class={format!("tab-btn {}", if self.report_type == ReportType::Monthly { "active" } else { "" })}
                        onclick={ctx.link().callback(|_| Msg::SetReportType(ReportType::Monthly))}
                    >
                        {"月报"}
                    </button>
                    <button
                        class={format!("tab-btn {}", if self.report_type == ReportType::Aging { "active" } else { "" })}
                        onclick={ctx.link().callback(|_| Msg::SetReportType(ReportType::Aging))}
                    >
                        {"账龄分析"}
                    </button>
                </div>

                <div class="filter-bar">
                    {self.render_filters(ctx)}
                    <button class="btn-refresh" onclick={ctx.link().callback(|_| Msg::Refresh)}>
                        {"刷新"}
                    </button>
                </div>

                {self.render_content(ctx)}
            </div>
        
</MainLayout>}
    }
}

impl ApReportPage {
    fn render_filters(&self, ctx: &Context<Self>) -> Html {
        match self.report_type {
            ReportType::Statistics => {
                html! {
                    <>
                        <div class="filter-item">
                            <label>{"开始日期："}</label>
                            <input
                                type="date"
                                value={self.start_date.clone()}
                                onchange={ctx.link().callback(|e: Event| {
                                    let target = e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
                                    Msg::SetStartDate(target.value())
                                })}
                            />
                        </div>
                        <div class="filter-item">
                            <label>{"结束日期："}</label>
                            <input
                                type="date"
                                value={self.end_date.clone()}
                                onchange={ctx.link().callback(|e: Event| {
                                    let target = e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
                                    Msg::SetEndDate(target.value())
                                })}
                            />
                        </div>
                    </>
                }
            }
            ReportType::Daily => {
                html! {
                    <div class="filter-item">
                        <label>{"报表日期："}</label>
                        <input
                            type="date"
                            value={self.report_date.clone()}
                            onchange={ctx.link().callback(|e: Event| {
                                let target = e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
                                Msg::SetReportDate(target.value())
                            })}
                        />
                    </div>
                }
            }
            ReportType::Monthly => {
                html! {
                    <>
                        <div class="filter-item">
                            <label>{"年份："}</label>
                            <input
                                type="number"
                                value={self.year.to_string()}
                                onchange={ctx.link().callback(|e: Event| {
                                    let target = e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
                                    Msg::SetYear(target.value().parse().unwrap_or(2024))
                                })}
                            />
                        </div>
                        <div class="filter-item">
                            <label>{"月份："}</label>
                            <input
                                type="number"
                                min="1"
                                max="12"
                                value={self.month.to_string()}
                                onchange={ctx.link().callback(|e: Event| {
                                    let target = e.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
                                    Msg::SetMonth(target.value().parse().unwrap_or(1))
                                })}
                            />
                        </div>
                    </>
                }
            }
            ReportType::Aging => {
                html! { <div class="filter-item">{"筛选条件：无"}</div> }
            }
        }
    }

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
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::Refresh)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        match self.report_type {
            ReportType::Statistics => self.render_statistics_content(),
            ReportType::Daily => self.render_daily_content(),
            ReportType::Monthly => self.render_monthly_content(),
            ReportType::Aging => self.render_aging_content(),
        }
    }

    fn render_statistics_content(&self) -> Html {
        if let Some(data) = &self.statistics_data {
            html! {
                <>
                    <div class="summary-cards">
                        <div class="summary-card">
                            <div class="summary-label">{"总应付金额"}</div>
                            <div class="summary-value">{&data.summary.total_amount}</div>
                        </div>
                        <div class="summary-card">
                            <div class="summary-label">{"已付款金额"}</div>
                            <div class="summary-value">{&data.summary.total_paid}</div>
                        </div>
                        <div class="summary-card">
                            <div class="summary-label">{"未付款金额"}</div>
                            <div class="summary-value">{&data.summary.total_outstanding}</div>
                        </div>
                        <div class="summary-card">
                            <div class="summary-label">{"供应商数量"}</div>
                            <div class="summary-value">{data.summary.supplier_count}</div>
                        </div>
                    </div>

                    <div class="table-responsive">
                        <table class="data-table w-full">
                            <thead>
                                <tr>
                                    <th>{"供应商编号"}</th>
                                    <th>{"供应商名称"}</th>
                                    <th>{"发票数量"}</th>
                                    <th>{"总金额"}</th>
                                    <th>{"已付金额"}</th>
                                    <th>{"未付金额"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for data.items.iter().map(|item| {
                                    html! {
                                        <tr>
                                            <td>{item.supplier_id}</td>
                                            <td>{&item.supplier_name}</td>
                                            <td class="numeric-cell text-right">{item.invoice_count}</td>
                                            <td class="numeric-cell text-right">{&item.total_amount}</td>
                                            <td class="numeric-cell text-right">{&item.paid_amount}</td>
                                            <td class="numeric-cell text-right">{&item.outstanding_amount}</td>
                                        </tr>
                                    }
                                })}
                            </tbody>
                        </table>
                    </div>
                </>
            }
        } else {
            html! {
                <div class="empty-state">
                    <div class="empty-icon">{"📊"}</div>
                    <p>{"暂无统计数据"}</p>
                </div>
            }
        }
    }

    fn render_daily_content(&self) -> Html {
        if let Some(data) = &self.daily_data {
            html! {
                <>
                    <div class="summary-cards">
                        <div class="summary-card">
                            <div class="summary-label">{"新增应付"}</div>
                            <div class="summary-value">{&data.summary.total_new_amount}</div>
                        </div>
                        <div class="summary-card">
                            <div class="summary-label">{"当日付款"}</div>
                            <div class="summary-value">{&data.summary.total_paid_amount}</div>
                        </div>
                        <div class="summary-card">
                            <div class="summary-label">{"应付余额"}</div>
                            <div class="summary-value">{&data.summary.total_outstanding}</div>
                        </div>
                        <div class="summary-card">
                            <div class="summary-label">{"供应商数量"}</div>
                            <div class="summary-value">{data.summary.supplier_count}</div>
                        </div>
                    </div>

                    <div class="table-responsive">
                        <table class="data-table w-full">
                            <thead>
                                <tr>
                                    <th>{"供应商编号"}</th>
                                    <th>{"供应商名称"}</th>
                                    <th>{"发票数量"}</th>
                                    <th>{"新增金额"}</th>
                                    <th>{"付款金额"}</th>
                                    <th>{"应付余额"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for data.items.iter().map(|item| {
                                    html! {
                                        <tr>
                                            <td>{item.supplier_id}</td>
                                            <td>{&item.supplier_name}</td>
                                            <td class="numeric-cell text-right">{item.invoice_count}</td>
                                            <td class="numeric-cell text-right">{&item.new_amount}</td>
                                            <td class="numeric-cell text-right">{&item.paid_amount}</td>
                                            <td class="numeric-cell text-right">{&item.outstanding_amount}</td>
                                        </tr>
                                    }
                                })}
                            </tbody>
                        </table>
                    </div>
                </>
            }
        } else {
            html! {
                <div class="empty-state">
                    <div class="empty-icon">{"📊"}</div>
                    <p>{"暂无日报数据"}</p>
                </div>
            }
        }
    }

    fn render_monthly_content(&self) -> Html {
        if let Some(data) = &self.monthly_data {
            html! {
                <>
                    <div class="summary-cards">
                        <div class="summary-card">
                            <div class="summary-label">{"本月应付"}</div>
                            <div class="summary-value">{&data.summary.total_month_amount}</div>
                        </div>
                        <div class="summary-card">
                            <div class="summary-label">{"本月付款"}</div>
                            <div class="summary-value">{&data.summary.total_paid}</div>
                        </div>
                        <div class="summary-card">
                            <div class="summary-label">{"应付余额"}</div>
                            <div class="summary-value">{&data.summary.total_outstanding}</div>
                        </div>
                        <div class="summary-card">
                            <div class="summary-label">{"供应商数量"}</div>
                            <div class="summary-value">{data.summary.supplier_count}</div>
                        </div>
                    </div>

                    <div class="table-responsive">
                        <table class="data-table w-full">
                            <thead>
                                <tr>
                                    <th>{"供应商编号"}</th>
                                    <th>{"供应商名称"}</th>
                                    <th>{"发票数量"}</th>
                                    <th>{"月应付金额"}</th>
                                    <th>{"月付款金额"}</th>
                                    <th>{"应付余额"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for data.items.iter().map(|item| {
                                    html! {
                                        <tr>
                                            <td>{item.supplier_id}</td>
                                            <td>{&item.supplier_name}</td>
                                            <td class="numeric-cell text-right">{item.invoice_count}</td>
                                            <td class="numeric-cell text-right">{&item.month_amount}</td>
                                            <td class="numeric-cell text-right">{&item.paid_amount}</td>
                                            <td class="numeric-cell text-right">{&item.outstanding_amount}</td>
                                        </tr>
                                    }
                                })}
                            </tbody>
                        </table>
                    </div>
                </>
            }
        } else {
            html! {
                <div class="empty-state">
                    <div class="empty-icon">{"📊"}</div>
                    <p>{"暂无月报数据"}</p>
                </div>
            }
        }
    }

    fn render_aging_content(&self) -> Html {
        if let Some(data) = &self.aging_data {
            html! {
                <>
                    <div class="summary-cards">
                        <div class="summary-card">
                            <div class="summary-label">{"总应付余额"}</div>
                            <div class="summary-value">{&data.summary.total_outstanding}</div>
                        </div>
                        <div class="summary-card">
                            <div class="summary-label">{"当前账龄"}</div>
                            <div class="summary-value">{&data.summary.total_current}</div>
                        </div>
                        <div class="summary-card">
                            <div class="summary-label">{"31-60天"}</div>
                            <div class="summary-value">{&data.summary.total_31_60}</div>
                        </div>
                        <div class="summary-card">
                            <div class="summary-label">{"61-90天"}</div>
                            <div class="summary-value">{&data.summary.total_61_90}</div>
                        </div>
                        <div class="summary-card">
                            <div class="summary-label">{"90天以上"}</div>
                            <div class="summary-value">{&data.summary.total_over_90}</div>
                        </div>
                    </div>

                    <div class="table-responsive">
                        <table class="data-table w-full">
                            <thead>
                                <tr>
                                    <th>{"供应商编号"}</th>
                                    <th>{"供应商名称"}</th>
                                    <th>{"发票数量"}</th>
                                    <th>{"总应付"}</th>
                                    <th>{"当前"}</th>
                                    <th>{"1-30天"}</th>
                                    <th>{"31-60天"}</th>
                                    <th>{"61-90天"}</th>
                                    <th>{"90天以上"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for data.items.iter().map(|item| {
                                    html! {
                                        <tr>
                                            <td>{item.supplier_id}</td>
                                            <td>{&item.supplier_name}</td>
                                            <td class="numeric-cell text-right">{item.invoice_count}</td>
                                            <td class="numeric-cell text-right">{&item.total_outstanding}</td>
                                            <td class="numeric-cell text-right">{&item.current_amount}</td>
                                            <td class="numeric-cell text-right">{&item.days_1_30}</td>
                                            <td class="numeric-cell text-right">{&item.days_31_60}</td>
                                            <td class="numeric-cell text-right">{&item.days_61_90}</td>
                                            <td class="numeric-cell text-right">{&item.days_over_90}</td>
                                        </tr>
                                    }
                                })}
                            </tbody>
                        </table>
                    </div>
                </>
            }
        } else {
            html! {
                <div class="empty-state">
                    <div class="empty-icon">{"📊"}</div>
                    <p>{"暂无账龄数据"}</p>
                </div>
            }
        }
    }
}
