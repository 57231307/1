//! 五维查询服务
//! 提供面料五维数据的查询、搜索和解析功能

use crate::models::five_dimension::{
    FiveDimensionIdParseResponse, FiveDimensionIdRequest, FiveDimensionListResponse,
    FiveDimensionSearchParams, FiveDimensionSearchResponse, FiveDimensionStatsParams,
    FiveDimensionStatsResponse,
};
use crate::services::api::ApiService;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

/// 五维查询服务
pub struct FiveDimensionService;

impl FiveDimensionService {
    /// 获取五维统计信息
    #[allow(dead_code)]
    pub async fn get_stats(
        params: FiveDimensionStatsParams,
    ) -> Result<FiveDimensionStatsResponse, String> {
        let query_string = build_query_string(&params);
        let url = if query_string.is_empty() {
            "/five-dimension/stats".to_string()
        } else {
            format!("/five-dimension/stats?{}", query_string)
        };
        ApiService::get::<FiveDimensionStatsResponse>(&url).await
    }

    /// 按五维ID获取统计信息
    pub async fn get_stats_by_id(
        five_dimension_id: &str,
    ) -> Result<FiveDimensionStatsResponse, String> {
        let url = format!("/five-dimension/{}", five_dimension_id);
        ApiService::get::<FiveDimensionStatsResponse>(&url).await
    }

    /// 获取五维列表
    pub async fn list_stats(
        params: FiveDimensionStatsParams,
    ) -> Result<FiveDimensionListResponse, String> {
        let query_string = build_query_string(&params);
        let url = if query_string.is_empty() {
            "/five-dimension/list".to_string()
        } else {
            format!("/five-dimension/list?{}", query_string)
        };
        ApiService::get::<FiveDimensionListResponse>(&url).await
    }

    /// 搜索五维数据
    pub async fn search(
        params: FiveDimensionSearchParams,
    ) -> Result<FiveDimensionSearchResponse, String> {
        let query_string = build_search_query_string(&params);
        let url = if query_string.is_empty() {
            "/five-dimension/search".to_string()
        } else {
            format!("/five-dimension/search?{}", query_string)
        };
        ApiService::get::<FiveDimensionSearchResponse>(&url).await
    }

    /// 解析五维ID
    pub async fn parse_id(five_dimension_id: &str) -> Result<FiveDimensionIdParseResponse, String> {
        let req = FiveDimensionIdRequest {
            five_dimension_id: five_dimension_id.to_string(),
        };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/five-dimension/parse", &payload).await
    }
}

/// 构建查询字符串（用于GET请求）
fn build_query_string(params: &FiveDimensionStatsParams) -> String {
    let mut parts = vec![];

    if let Some(product_id) = params.product_id {
        parts.push(format!("product_id={}", product_id));
    }
    if let Some(ref batch_no) = params.batch_no {
        if !batch_no.is_empty() {
            parts.push(format!(
                "batch_no={}",
                utf8_percent_encode(batch_no, NON_ALPHANUMERIC)
            ));
        }
    }
    if let Some(ref color_no) = params.color_no {
        if !color_no.is_empty() {
            parts.push(format!(
                "color_no={}",
                utf8_percent_encode(color_no, NON_ALPHANUMERIC)
            ));
        }
    }
    if let Some(ref dye_lot_no) = params.dye_lot_no {
        if !dye_lot_no.is_empty() {
            parts.push(format!(
                "dye_lot_no={}",
                utf8_percent_encode(dye_lot_no, NON_ALPHANUMERIC)
            ));
        }
    }
    if let Some(ref grade) = params.grade {
        if !grade.is_empty() {
            parts.push(format!(
                "grade={}",
                utf8_percent_encode(grade, NON_ALPHANUMERIC)
            ));
        }
    }
    if let Some(warehouse_id) = params.warehouse_id {
        parts.push(format!("warehouse_id={}", warehouse_id));
    }
    if let Some(page) = params.page {
        parts.push(format!("page={}", page));
    }
    if let Some(page_size) = params.page_size {
        parts.push(format!("page_size={}", page_size));
    }

    parts.join("&")
}

/// 构建搜索查询字符串
fn build_search_query_string(params: &FiveDimensionSearchParams) -> String {
    let mut parts = vec![];

    if !params.keyword.is_empty() {
        parts.push(format!(
            "keyword={}",
            utf8_percent_encode(&params.keyword, NON_ALPHANUMERIC)
        ));
    }
    if !params.search_type.is_empty() {
        parts.push(format!(
            "search_type={}",
            utf8_percent_encode(&params.search_type, NON_ALPHANUMERIC)
        ));
    }
    if let Some(page) = params.page {
        parts.push(format!("page={}", page));
    }
    if let Some(page_size) = params.page_size {
        parts.push(format!("page_size={}", page_size));
    }

    parts.join("&")
}
