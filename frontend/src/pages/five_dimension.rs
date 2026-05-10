// 五维查询页面
// 提供面料五维数据的查询、搜索和管理功能

use crate::components::{
    empty_state::EmptyState,
    loading_state::LoadingState,
    page_header::PageHeader,
    pagination::Pagination,
    search_bar::SearchBar,
};
use crate::models::five_dimension::{
    FiveDimensionItem, FiveDimensionListResponse, FiveDimensionSearchParams,
    FiveDimensionSearchResponse, FiveDimensionStatsParams, FiveDimensionStatsResponse,
};
use crate::services::crud_service::CrudService;
use crate::services::five_dimension_service::FiveDimensionService;
use crate::utils::permissions;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

pub struct FiveDimensionPage {
    list_data: Option<FiveDimensionListResponse>,
    selected_stats: Option<FiveDimensionStatsResponse>,
    search_results: Vec<FiveDimensionItem>,
    parsed_dimension: Option<FiveDimensionItem>,
    loading: bool,
    searching: bool,
    error: Option<String>,
    search_keyword: String,
    search_type: String,
    parse_input: String,
    parse_error: Option<String>,
    active_tab: String,
    page: u64,
    page_size: u64,
    filtered_list: Vec<FiveDimensionStatsResponse>,
    search_types: Vec<(String, String)>,
}

pub enum Msg {
    LoadList,
    ListLoaded(Result<FiveDimensionListResponse, String>),
    UpdateSearchKeyword(String),
    UpdateSearchType(String),
    Search,
    SearchResultLoaded(Result<FiveDimensionSearchResponse, String>),
    UpdateParseInput(String),
    ParseId,
    ParseResultLoaded(Result<FiveDimensionItem, String>),
    ViewStats(FiveDimensionItem),
    ClearSelection,
    SetActiveTab(String),
    PageChanged(u64),
    Error(String),
}

impl FiveDimensionPage {
    const SEARCH_TYPES: &'static [(&'static str, &'static str)] = &[
        ("product", "按产品"),
        ("batch", "按批次"),
        ("color", "按色号"),
        ("dye_lot", "按染缸"),
        ("grade", "按等级"),
    ];
}

impl Component for FiveDimensionPage {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::LoadList);
        Self {
            list_data: None,
            selected_stats: None,
            search_results: Vec::new(),
            parsed_dimension: None,
            loading: true,
            searching: false,
            error: None,
            search_keyword: String::new(),
            search_type: "product".to_string(),
            parse_input: String::new(),
            parse_error: None,
            active_tab: "list".to_string(),
            page: 0,
            page_size: 10,
            filtered_list: Vec::new(),
            search_types: Self::SEARCH_TYPES
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadList => {
                self.loading = true;
                let link = _ctx.link().clone();
                spawn_local(async move {
                    let params = FiveDimensionStatsParams {
                        product_id: None,
                        batch_no: None,
                        color_no: None,
                        dye_lot_no: None,
                        grade: None,
                        warehouse_id: None,
                        page: Some(0),
                        page_size: Some(100),
                    };
                    let result = FiveDimensionService::list_stats(params).await;
                    link.send_message(Msg::ListLoaded(result));
                });
                false
            }
            Msg::ListLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(data) => {
                        self.filtered_list = data.items.clone();
                        self.list_data = Some(data);
                        self.error = None;
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
                true
            }
            Msg::UpdateSearchKeyword(keyword) => {
                self.search_keyword = keyword;
                false
            }
            Msg::UpdateSearchType(search_type) => {
                self.search_type = search_type;
                false
            }
            Msg::Search => {
                self.searching = true;
                let link = _ctx.link().clone();
                let keyword = self.search_keyword.clone();
                let search_type = self.search_type.clone();
                spawn_local(async move {
                    let params = FiveDimensionSearchParams {
                        keyword,
                        search_type,
                        page: Some(0),
                        page_size: Some(100),
                    };
                    let result = FiveDimensionService::search(params).await;
                    link.send_message(Msg::SearchResultLoaded(result));
                });
                false
            }
            Msg::SearchResultLoaded(result) => {
                self.searching = false;
                match result {
                    Ok(response) => {
                        self.search_results = response.items;
                        self.error = None;
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
                true
            }
            Msg::UpdateParseInput(input) => {
                self.parse_input = input;
                false
            }
            Msg::ParseId => {
                if self.parse_input.is_empty() {
                    self.parse_error = Some("请输入五维ID".to_string());
                    return true;
                }
                let link = _ctx.link().clone();
                let five_dimension_id = self.parse_input.clone();
                spawn_local(async move {
                    let result = FiveDimensionService::parse_id(&five_dimension_id).await;
                    match result {
                        Ok(response) => {
                            if response.success {
                                if let Some(dimension) = response.dimension {
                                    link.send_message(Msg::ParseResultLoaded(Ok(dimension)));
                                } else {
                                    link.send_message(Msg::ParseResultLoaded(Err(
                                        "解析结果为空".to_string(),
                                    )));
                                }
                            } else {
                                link.send_message(Msg::ParseResultLoaded(Err(
                                    response
                                        .error
                                        .unwrap_or_else(|| "解析失败".to_string()),
                                )));
                            }
                        }
                        Err(e) => {
                            link.send_message(Msg::ParseResultLoaded(Err(e)));
                        }
                    }
                });
                false
            }
            Msg::ParseResultLoaded(result) => {
                match result {
                    Ok(dimension) => {
                        self.parsed_dimension = Some(dimension);
                        self.parse_error = None;
                    }
                    Err(e) => {
                        self.parsed_dimension = None;
                        self.parse_error = Some(e);
                    }
                }
                true
            }
            Msg::ViewStats(item) => {
                let link = _ctx.link().clone();
                let five_dimension_id = item.five_dimension_id.clone();
                spawn_local(async move {
                    let result = FiveDimensionService::get_stats_by_id(&five_dimension_id).await;
                    match result {
                        Ok(stats) => {
                            link.send_message(Msg::ListLoaded(Ok(FiveDimensionListResponse {
                                items: vec![stats],
                                total: 1,
                                page: 0,
                                page_size: 20,
                            })));
                        }
                        Err(e) => {
                            link.send_message(Msg::Error(e));
                        }
                    }
                });
                false
            }
            Msg::ClearSelection => {
                self.selected_stats = None;
                true
            }
            Msg::SetActiveTab(tab) => {
                self.active_tab = tab;
                self.page = 0;
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
            <div class="five-dimension-page">
                <PageHeader title={"五维数据".to_string()} subtitle={Some("面料批次、色号、染缸、等级五维数据管理".to_string())}>
                    <button class="btn btn-primary" onclick={ctx.link().callback(|_| Msg::LoadList)}>
                        {"刷新"}
                    </button>
                </PageHeader>

                {self.render_tabs(ctx)}
                {self.render_content(ctx)}
            </div>
        }
    }
}

impl FiveDimensionPage {
    fn render_tabs(&self, ctx: &Context<Self>) -> Html {
        let tabs = vec![
            ("list", "数据列表"),
            ("search", "搜索"),
            ("parse", "解析ID"),
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
                <LoadingState message={"正在加载五维数据...".to_string()} />
            };
        }

        if let Some(error) = &self.error {
            return html! {
                <div class="error-container">
                    <div class="error-icon">{"⚠️"}</div>
                    <p class="error-message">{error}</p>
                    <button class="btn btn-primary" onclick={ctx.link().callback(|_| Msg::LoadList)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        match self.active_tab.as_str() {
            "list" => self.render_list_section(ctx),
            "search" => self.render_search_section(ctx),
            "parse" => self.render_parse_section(ctx),
            _ => html! {},
        }
    }

    fn render_search_section(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="card search-card">
                <div class="card-header">
                    <h2>{"搜索五维数据"}</h2>
                </div>
                <div class="card-body">
                    <div class="search-form">
                        <div class="form-group">
                            <label for="search-type">{"搜索类型"}</label>
                            <select
                                id="search-type"
                                class="form-control"
                                value={self.search_type.clone()}
                                onchange={ctx.link().callback(|e: Event| {
                                    let target = e.target_unchecked_into::<HtmlSelectElement>();
                                    Msg::UpdateSearchType(target.value())
                                })}
                            >
                                {for self.search_types.iter().map(|(k, v)| {
                                    html! {
                                        <option value={k.clone()}>{v}</option>
                                    }
                                })}
                            </select>
                        </div>
                        <div class="form-group">
                            <label for="search-keyword">{"关键字"}</label>
                            <input
                                id="search-keyword"
                                type="text"
                                class="form-control"
                                placeholder="请输入搜索关键字"
                                value={self.search_keyword.clone()}
                                oninput={ctx.link().callback(|e: InputEvent| {
                                    let target = e.target_unchecked_into::<HtmlInputElement>();
                                    Msg::UpdateSearchKeyword(target.value())
                                })}
                            />
                        </div>
                        <button
                            class="btn btn-primary"
                            onclick={ctx.link().callback(|_| Msg::Search)}
                            disabled={self.searching}
                        >
                            {if self.searching { "搜索中..." } else { "搜索" }}
                        </button>
                    </div>

                    if !self.search_results.is_empty() {
                        <div class="search-results">
                            <h3>{"搜索结果"}</h3>
                            <div class="table-container">
                                <table class="data-table">
                                    <thead>
                                        <tr>
                                            <th>{"五维ID"}</th>
                                            <th>{"产品ID"}</th>
                                            <th>{"产品名称"}</th>
                                            <th>{"批次号"}</th>
                                            <th>{"色号"}</th>
                                            <th>{"染缸号"}</th>
                                            <th>{"等级"}</th>
                                            <th>{"操作"}</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {for self.search_results.iter().map(|item| {
                                            let item_clone = item.clone();
                                            html! {
                                                <tr>
                                                    <td class="five-dim-id">{&item.five_dimension_id}</td>
                                                    <td>{item.product_id}</td>
                                                    <td>{item.product_name.clone().unwrap_or_else(|| "-".to_string())}</td>
                                                    <td>{&item.batch_no}</td>
                                                    <td>{&item.color_no}</td>
                                                    <td>{item.dye_lot_no.clone().unwrap_or_else(|| "-".to_string())}</td>
                                                    <td>{&item.grade}</td>
                                                    <td>
                                                        <button
                                                            class="btn btn-sm btn-info"
                                                            onclick={ctx.link().callback(move |_| Msg::ViewStats(item_clone.clone()))}
                                                        >
                                                            {"查看详情"}
                                                        </button>
                                                    </td>
                                                </tr>
                                            }
                                        })}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    } else if !self.search_keyword.is_empty() && !self.searching {
                        <EmptyState
                            icon={"🔍".to_string()}
                            title={"暂无搜索结果".to_string()}
                            description={"没有匹配搜索条件的五维数据".to_string()}
                        />
                    }
                </div>
            </div>
        }
    }

    fn render_parse_section(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="card parse-card">
                <div class="card-header">
                    <h2>{"解析五维ID"}</h2>
                </div>
                <div class="card-body">
                    <div class="parse-form">
                        <div class="form-group">
                            <label for="parse-input">{"五维ID"}</label>
                            <input
                                id="parse-input"
                                type="text"
                                class="form-control"
                                placeholder="请输入五维ID"
                                value={self.parse_input.clone()}
                                oninput={ctx.link().callback(|e: InputEvent| {
                                    let target = e.target_unchecked_into::<HtmlInputElement>();
                                    Msg::UpdateParseInput(target.value())
                                })}
                            />
                        </div>
                        <button
                            class="btn btn-primary"
                            onclick={ctx.link().callback(|_| Msg::ParseId)}
                        >
                            {"解析"}
                        </button>
                    </div>

                    if let Some(dimension) = &self.parsed_dimension {
                        <div class="parse-result success">
                            <h3>{"解析成功"}</h3>
                            <div class="result-details">
                                <div class="result-item">
                                    <span class="label">{"五维ID："}</span>
                                    <span class="value">{&dimension.five_dimension_id}</span>
                                </div>
                                <div class="result-item">
                                    <span class="label">{"产品ID："}</span>
                                    <span class="value">{dimension.product_id}</span>
                                </div>
                                <div class="result-item">
                                    <span class="label">{"产品名称："}</span>
                                    <span class="value">{dimension.product_name.clone().unwrap_or_else(|| "-".to_string())}</span>
                                </div>
                                <div class="result-item">
                                    <span class="label">{"批次号："}</span>
                                    <span class="value">{&dimension.batch_no}</span>
                                </div>
                                <div class="result-item">
                                    <span class="label">{"色号："}</span>
                                    <span class="value">{&dimension.color_no}</span>
                                </div>
                                <div class="result-item">
                                    <span class="label">{"染缸号："}</span>
                                    <span class="value">{dimension.dye_lot_no.clone().unwrap_or_else(|| "-".to_string())}</span>
                                </div>
                                <div class="result-item">
                                    <span class="label">{"等级："}</span>
                                    <span class="value">{&dimension.grade}</span>
                                </div>
                            </div>
                        </div>
                    }

                    if let Some(error) = &self.parse_error {
                        <div class="parse-result error">
                            <div class="error-message">{"⚠️ "}{error}</div>
                        </div>
                    }
                </div>
            </div>
        }
    }

    fn render_list_section(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="card list-card">
                <div class="card-header">
                    <h2>{"五维统计数据列表"}</h2>
                </div>
                <div class="card-body">
                    if let Some(list_data) = &self.list_data {
                        if list_data.items.is_empty() {
                            <EmptyState
                                icon={"📭".to_string()}
                                title={"暂无数据".to_string()}
                                description={"当前暂无五维统计数据".to_string()}
                            />
                        } else {
                            <div class="table-container">
                                <table class="data-table">
                                    <thead>
                                        <tr>
                                            <th>{"五维ID"}</th>
                                            <th>{"产品"}</th>
                                            <th>{"批次"}</th>
                                            <th>{"色号"}</th>
                                            <th>{"染缸"}</th>
                                            <th>{"等级"}</th>
                                            <th>{"总米数"}</th>
                                            <th>{"总公斤数"}</th>
                                            <th>{"库存记录数"}</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {for self.paginated_items(&list_data.items).iter().map(|item| {
                                            html! {
                                                <tr>
                                                    <td class="five-dim-id">{&item.dimension.five_dimension_id}</td>
                                                    <td>
                                                        {item.dimension.product_name.clone().unwrap_or_else(|| format!("产品#{}", item.dimension.product_id))}
                                                    </td>
                                                    <td>{&item.dimension.batch_no}</td>
                                                    <td>{&item.dimension.color_no}</td>
                                                    <td>{item.dimension.dye_lot_no.clone().unwrap_or_else(|| "-".to_string())}</td>
                                                    <td>{&item.dimension.grade}</td>
                                                    <td class="numeric">{self.format_decimal(&item.total_meters)}</td>
                                                    <td class="numeric">{self.format_decimal(&item.total_kg)}</td>
                                                    <td class="numeric">{item.stock_count}</td>
                                                </tr>
                                            }
                                        })}
                                    </tbody>
                                </table>
                                <Pagination
                                    current_page={self.page}
                                    page_size={self.page_size}
                                    total={list_data.total}
                                    on_page_change={ctx.link().callback(|page| Msg::PageChanged(page))}
                                />
                            </div>
                        }
                    } else {
                        <EmptyState
                            icon={"📭".to_string()}
                            title={"暂无数据".to_string()}
                            description={"当前暂无五维统计数据".to_string()}
                        />
                    }
                </div>
            </div>
        }
    }

    fn paginated_items(&self, items: &[FiveDimensionStatsResponse]) -> Vec<FiveDimensionStatsResponse> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        items[start..end.min(items.len())].to_vec()
    }

    fn format_decimal(&self, value: &serde_json::Value) -> String {
        if let Some(num) = value.as_f64() {
            format!("{:.2}", num)
        } else if let Some(num) = value.as_i64() {
            num.to_string()
        } else {
            value.to_string()
        }
    }
}
