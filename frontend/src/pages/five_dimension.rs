// 五维查询页面
// 提供面料五维数据的查询、搜索和管理功能

use crate::utils::permissions;
use yew::prelude::*;
use wasm_bindgen_futures::spawn_local;
use crate::models::five_dimension::{
    FiveDimensionStatsParams, FiveDimensionItem, FiveDimensionStatsResponse, FiveDimensionListResponse, FiveDimensionSearchParams,
};
use crate::services::five_dimension_service::FiveDimensionService;
use crate::services::crud_service::CrudService;

pub struct FiveDimensionPage {
    // 列表数据
    list_data: Option<FiveDimensionListResponse>,
    // 当前选中的统计详情
    selected_stats: Option<FiveDimensionStatsResponse>,
    // 搜索结果
    search_results: Vec<FiveDimensionItem>,
    // 解析后的五维ID详情
    parsed_dimension: Option<FiveDimensionItem>,
    // 加载状态
    loading: bool,
    // 搜索加载状态
    searching: bool,
    // 错误信息
    error: Option<String>,
    // 搜索关键字
    search_keyword: String,
    // 搜索类型
    search_type: String,
    // 解析输入
    parse_input: String,
    // 解析错误
    parse_error: Option<String>,
    // 搜索类型选项
    search_types: Vec<(String, String)>,
}

pub enum Msg {
    // 加载列表数据
    LoadList,
    ListLoaded(Result<FiveDimensionListResponse, String>),
    // 搜索
    UpdateSearchKeyword(String),
    UpdateSearchType(String),
    Search,
    SearchResultLoaded(Result<Vec<FiveDimensionItem>, String>),
    // 解析五维ID
    UpdateParseInput(String),
    ParseId,
    ParseResultLoaded(Result<FiveDimensionItem, String>),
    // 查看详情
    ViewStats(FiveDimensionItem),
    ClearSelection,
    // 错误处理
    Error(String),
}

impl FiveDimensionPage {
    // 搜索类型列表
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
            search_types: Self::SEARCH_TYPES.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
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
                        page_size: Some(20),
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
                        page_size: Some(20),
                    };
                    let result = FiveDimensionService::search(params).await;
                    link.send_message(Msg::SearchResultLoaded(result.map(|r| r.items)));
                });
                false
            }
            Msg::SearchResultLoaded(result) => {
                self.searching = false;
                match result {
                    Ok(items) => {
                        self.search_results = items;
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
                                    link.send_message(Msg::ParseResultLoaded(Err("解析结果为空".to_string())));
                                }
                            } else {
                                link.send_message(Msg::ParseResultLoaded(Err(response.error.unwrap_or_else(|| "解析失败".to_string()))));
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
                            // 更新选中项
                            let mut stats = stats;
                            stats.dimension = item;
                            link.send_message(Msg::ListLoaded(Ok(FiveDimensionListResponse {
                                items: vec![stats.clone()],
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
            Msg::Error(e) => {
                self.error = Some(e);
                self.loading = false;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <div class="five-dimension-page">
                    <div class="page-header">
                        <h1>{"五维查询"}</h1>
                        <p class="subtitle">{"面料批次、色号、染缸、等级五维数据管理"}</p>
                    </div>

                    {self.render_content(ctx)}
                </div>
            </>
        }
    }
}

impl FiveDimensionPage {
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
                    <button class="btn-primary" onclick={ctx.link().callback(|_| Msg::LoadList)}>
                        {"重新加载"}
                    </button>
                </div>
            };
        }

        html! {
            <>
                // 搜索区域
                {self.render_search_section(ctx)}

                // 解析五维ID区域
                {self.render_parse_section(ctx)}

                // 列表区域
                {self.render_list_section(ctx)}
            </>
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
                                    let target = e.target_unchecked_into::<web_sys::HtmlSelectElement>();
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
                                    let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                    Msg::UpdateSearchKeyword(target.value())
                                })}
                            />
                        </div>
                        <button
                            class="btn-primary"
                            onclick={ctx.link().callback(|_| Msg::Search)}
                            disabled={self.searching}
                        >
                            {if self.searching { "搜索中..." } else { "搜索" }}
                        </button>
                    </div>

                    // 搜索结果
                    if !self.search_results.is_empty() {
                        <div class="search-results">
                            <h3>{"搜索结果"}</h3>
                            <div class="table-responsive">
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
                                                    <td>{item.product_id.to_string()}</td>
                                                    <td>{item.product_name.clone().unwrap_or_else(|| "-".to_string())}</td>
                                                    <td>{&item.batch_no}</td>
                                                    <td>{&item.color_no}</td>
                                                    <td>{item.dye_lot_no.clone().unwrap_or_else(|| "-".to_string())}</td>
                                                    <td>{&item.grade}</td>
                                                    <td>
                                                        <button
                                                            class="btn-sm btn-info"
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
                                placeholder="请输入五维ID，如：P1|B20240101|C001|D20240101001|G 一等品"
                                value={self.parse_input.clone()}
                                oninput={ctx.link().callback(|e: InputEvent| {
                                    let target = e.target_unchecked_into::<web_sys::HtmlInputElement>();
                                    Msg::UpdateParseInput(target.value())
                                })}
                            />
                        </div>
                        <button
                            class="btn-primary"
                            onclick={ctx.link().callback(|_| Msg::ParseId)}
                        >
                            {"解析"}
                        </button>
                    </div>

                    // 解析结果
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
                                    <span class="value">{dimension.product_id.to_string()}</span>
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
                    <button class="btn-secondary" onclick={ctx.link().callback(|_| Msg::LoadList)}>
                        {"刷新"}
                    </button>
                </div>
                <div class="card-body">
                    if let Some(list_data) = &self.list_data {
                        if list_data.items.is_empty() {
                            <div class="empty-state">
                                <div class="empty-icon">{"📭"}</div>
                                <p>{"暂无数据"}</p>
                            </div>
                        } else {
                            <div class="table-responsive">
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
                                        {for list_data.items.iter().map(|item| {
                                            html! {
                                                <tr>
                                                    <td class="five-dim-id">{&item.dimension.five_dimension_id}</td>
                                                    <td>
                                                        <div class="cell-with-tooltip">
                                                            {item.dimension.product_name.clone().unwrap_or_else(|| format!("产品#{}", item.dimension.product_id))}
                                                        </div>
                                                    </td>
                                                    <td>{&item.dimension.batch_no}</td>
                                                    <td>{&item.dimension.color_no}</td>
                                                    <td>{item.dimension.dye_lot_no.clone().unwrap_or_else(|| "-".to_string())}</td>
                                                    <td>{&item.dimension.grade}</td>
                                                    <td class="numeric">{self.format_decimal(&item.total_meters)}</td>
                                                    <td class="numeric">{self.format_decimal(&item.total_kg)}</td>
                                                    <td class="numeric">{item.stock_count.to_string()}</td>
                                                </tr>
                                            }
                                        })}
                                    </tbody>
                                </table>
                            </div>
                            <div class="pagination-info">
                                <span>{"第 "}{list_data.page + 1}{" / "}{((list_data.total as f64 / list_data.page_size as f64).ceil() as u64).max(1)}{" 页"}</span>
                                <span>{"共 "}{list_data.total}{" 条记录"}</span>
                            </div>
                        }
                    }
                </div>
            </div>
        }
    }

    // 格式化数值
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