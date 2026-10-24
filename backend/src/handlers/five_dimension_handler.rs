//! 五维管理 Handler
//!
//! 提供面料五维编码的查询、统计和搜索功能
//! 五维编码：产品ID + 批次号 + 色号 + 缸号 + 等级

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use validator::Validate;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::five_dimension_service::{
    FiveDimensionQuery, FiveDimensionSearchParams as ServiceFiveDimensionSearchParams,
    FiveDimensionService, FiveDimensionStats,
};
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

/// 解析分页入参为**实际生效**的分页值：`page` 缺省第 1 页、上限 1000，
/// `page_size` 缺省 20 条、上限 100（防 DoS，与全仓列表接口同一套分页约定）。
///
/// 出参 `PaginatedResponse` 里的 page/page_size 必须是这两个生效值，前端分页条才能
/// 与后端真实切片一致；入参在传入前已被调用方 clamp 时本函数幂等。
fn resolve_paging(page: Option<u64>, page_size: Option<u64>) -> (u64, u64) {
    (
        page.unwrap_or(1).clamp(1, 1000),
        page_size.unwrap_or(20).clamp(1, 100),
    )
}

/// 五维统计列表的真实取数 + 统一分页信封构造（stats 与 list 两个挂载点共用同一口径，
/// 避免两处各写一套默认值而漂出第二种分页形状）。
async fn stats_page(
    state: &AppState,
    params: FiveDimensionStatsParams,
) -> Result<PaginatedResponse<FiveDimensionStats>, AppError> {
    let service = FiveDimensionService::new(state.db.clone());
    let (page, page_size) = resolve_paging(params.page, params.page_size);

    let query = FiveDimensionQuery {
        product_id: params.product_id,
        batch_no: params.batch_no,
        color_no: params.color_no,
        dye_lot_no: params.dye_lot_no,
        grade: params.grade,
        warehouse_id: params.warehouse_id,
        page: Some(page),
        page_size: Some(page_size),
    };

    let (items, total) = service.get_stats(query).await?;
    Ok(PaginatedResponse::new(items, total, page, page_size))
}

/// P1-2l 修复（批次 81 v1 复审）：解析五维ID请求 DTO
/// 替代 parse_five_dimension_id 中的 Json<serde_json::Value>，提供强类型校验
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct ParseFiveDimensionIdDto {
    /// 五维编码：必填，长度至少 1
    #[validate(length(min = 1, max = 200, message = "五维编码长度必须在1到200字符之间"))]
    pub five_dimension_id: String,
}

/// 五维统计请求参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct FiveDimensionStatsParams {
    pub product_id: Option<i32>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub warehouse_id: Option<i32>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 五维搜索参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct FiveDimensionSearchParams {
    pub keyword: String,
    pub search_type: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// GET /api/v1/erp/five-dimension/stats - 获取五维统计信息
pub async fn get_five_dimension_stats(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<FiveDimensionStatsParams>,
) -> Result<Json<ApiResponse<PaginatedResponse<FiveDimensionStats>>>, AppError> {
    Ok(Json(ApiResponse::success(
        stats_page(&state, params).await?,
    )))
}

/// GET /api/v1/erp/five-dimension/stats/:id - 按五维ID查询统计
pub async fn get_stats_by_five_dimension_id(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(five_dimension_id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = FiveDimensionService::new(state.db.clone());

    let stats = service.get_stats_by_id(&five_dimension_id).await?;

    match stats {
        Some(stats) => Ok(Json(ApiResponse::success(serde_json::to_value(stats)?))),
        None => Err(AppError::not_found("未找到五维统计数据")),
    }
}

/// GET /api/v1/erp/five-dimension/search - 五维搜索
pub async fn search_five_dimension(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<FiveDimensionSearchParams>,
) -> Result<Json<ApiResponse<PaginatedResponse<FiveDimensionStats>>>, AppError> {
    let service = FiveDimensionService::new(state.db.clone());
    let (page, page_size) = resolve_paging(params.page, params.page_size);

    let search_params = ServiceFiveDimensionSearchParams {
        keyword: params.keyword,
        search_type: params.search_type,
        page: Some(page),
        page_size: Some(page_size),
    };

    let (items, total) = service.search(search_params).await?;

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// GET /api/v1/erp/five-dimension/list - 列出五维统计
pub async fn list_five_dimension_stats(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<FiveDimensionStatsParams>,
) -> Result<Json<ApiResponse<PaginatedResponse<FiveDimensionStats>>>, AppError> {
    Ok(Json(ApiResponse::success(
        stats_page(&state, params).await?,
    )))
}

/// GET /api/v1/erp/five-dimension/summary - 五维统计汇总
pub async fn get_five_dimension_summary(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = FiveDimensionService::new(state.db.clone());

    let summary = service.get_summary().await?;

    Ok(Json(ApiResponse::success(summary)))
}

/// POST /api/v1/erp/five-dimension/parse - 解析五维ID
pub async fn parse_five_dimension_id(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<ParseFiveDimensionIdDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // P1-2l 修复（批次 81 v1 复审）：强类型 DTO + validator 替代 Json<Value>
    req.validate().map_err(AppError::from)?;

    let service = FiveDimensionService::new(state.db.clone());

    let stats = service.get_stats_by_id(&req.five_dimension_id).await?;

    match stats {
        Some(stats) => Ok(Json(ApiResponse::success(serde_json::json!({
            "success": true,
            "dimension": stats,
        })))),
        None => Ok(Json(ApiResponse::success(serde_json::json!({
            "success": false,
            "error": "未找到五维数据",
        })))),
    }
}
