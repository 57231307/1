// 库存查询页面
// 提供面料库存的查询、搜索、分页等功能

use crate::utils::toast_helper;
use yew::prelude::*;
use crate::components::{
    search_bar::SearchBar,
    pagination::Pagination,
    page_header::PageHeader,
    empty_state::EmptyState,
    loading_state::LoadingState,
};
use wasm_bindgen_futures::spawn_local;
use crate::models::inventory::{StockFabricResponse, InventorySummaryItem};
use crate::services::inventory_service::InventoryService;

pub struct InventoryStockPage {
    stocks: Vec<StockFabricResponse>,
    filtered_stocks: Vec<StockFabricResponse>,
    summary: Vec<InventorySummaryItem>,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    page: u64,
    page_size: u64,
}

pub enum Msg {
    LoadData,
    DataLoaded(Vec<StockFabricResponse>),
    SummaryLoaded(Vec<InventorySummaryItem>),
    LoadError(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
}

impl Component for InventoryStockPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            stocks: Vec::new(),
            filtered_stocks: Vec::new(),
            summary: Vec::new(),
            loading: true,
            error: None,
            search_keyword: String::new(),
            page: 0,
            page_size: 10,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadData);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadData => {
                self.loading = true;
                self.error = None;
                let link = ctx.link().clone();
                spawn_local(async move {
                    match InventoryService::list_stock_fabric(1, 1000, None, None).await {
                        Ok(resp) => link.send_message(Msg::DataLoaded(resp)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                let link2 = ctx.link().clone();
                spawn_local(async move {
                    if let Ok(sum) = InventoryService::get_inventory_summary().await {
                        link2.send_message(Msg::SummaryLoaded(sum));
                    }
                });
                false
            }
            Msg::DataLoaded(data) => {
                self.loading = false;
                self.stocks = data;
                self.apply_filter();
                true
            }
            Msg::SummaryLoaded(data) => {
                self.summary = data;
                true
            }
            Msg::LoadError(err) => {
                self.error = Some(err);
                self.loading = false;
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
        let link = ctx.link();

        let total_meters = self.summary.iter()
            .filter_map(|s| s.total_quantity_meters.parse::<f64>().ok())
            .sum::<f64>();
        let total_kg = self.summary.iter()
            .filter_map(|s| s.total_quantity_kg.parse::<f64>().ok())
            .sum::<f64>();

        html! {
            <div class="inventory-stock-page">
                <PageHeader title={"面料库存查询".to_string()} subtitle={Some("查询和管理面料库存信息".to_string())}>
                    <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadData)}>
                        {"刷新数据"}
                    </button>
                </PageHeader>

                if !self.summary.is_empty() {
                    <div class="dashboard-grid">
                        <div class="card">
                            <h3>{"库存总米数"}</h3>
                            <div class="metric-value" style="color: #2980b9;">
                                {format!("{:.2} M", total_meters)}
                            </div>
                        </div>
                        <div class="card">
                            <h3>{"库存总重量"}</h3>
                            <div class="metric-value" style="color: #27ae60;">
                                {format!("{:.2} KG", total_kg)}
                            </div>
                        </div>
                    </div>
                }

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索批号、色号...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载库存数据...".to_string()} />
                } else if let Some(err) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{err}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadData)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_stocks.is_empty() {
                    <EmptyState
                        icon={"📦".to_string()}
                        title={"暂无库存数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "暂无面料库存记录".to_string()
                        } else {
                            "没有匹配搜索条件的库存记录".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"仓库ID"}</th>
                                    <th>{"产品ID"}</th>
                                    <th>{"批号"}</th>
                                    <th>{"色号"}</th>
                                    <th>{"缸号"}</th>
                                    <th>{"等级"}</th>
                                    <th class="numeric">{"米数"}</th>
                                    <th class="numeric">{"重量"}</th>
                                    <th>{"库位"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_stocks().iter().map(|s| {
                                    html! {
                                        <tr>
                                            <td>{ s.warehouse_id }</td>
                                            <td>{ s.product_id }</td>
                                            <td>{ &s.batch_no }</td>
                                            <td>{ &s.color_no }</td>
                                            <td>{ s.dye_lot_no.clone().unwrap_or_else(|| "-".to_string()) }</td>
                                            <td>{ &s.grade }</td>
                                            <td class="numeric">{ &s.quantity_meters }</td>
                                            <td class="numeric">{ &s.quantity_kg }</td>
                                            <td>{ s.bin_location.clone().unwrap_or_else(|| "-".to_string()) }</td>
                                        </tr>
                                    }
                                })}
                            </tbody>
                        </table>

                        <Pagination
                            current_page={self.page}
                            page_size={self.page_size}
                            total={self.filtered_stocks.len() as u64}
                            on_page_change={link.callback(|page| Msg::PageChanged(page))}
                        />
                    </div>
                }
            </div>
        }
    }
}

impl InventoryStockPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_stocks = self.stocks.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_stocks = self.stocks.iter()
                .filter(|s| {
                    s.batch_no.to_lowercase().contains(&keyword) ||
                    s.color_no.to_lowercase().contains(&keyword) ||
                    s.grade.to_lowercase().contains(&keyword) ||
                    s.dye_lot_no.as_ref().map(|v| v.to_lowercase().contains(&keyword)).unwrap_or(false) ||
                    s.bin_location.as_ref().map(|v| v.to_lowercase().contains(&keyword)).unwrap_or(false)
                })
                .cloned()
                .collect();
        }
    }

    fn paginated_stocks(&self) -> Vec<StockFabricResponse> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_stocks[start..end.min(self.filtered_stocks.len())].to_vec()
    }
}
