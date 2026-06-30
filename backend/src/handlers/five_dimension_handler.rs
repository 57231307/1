//! 五维管理 Handler
//!
//! 提供面料五维编码的查询、统计和搜索功能
//! 五维编码：产品ID + 批次号 + 色号 + 缸号 + 等级

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::services::five_dimension_service::{
    FiveDimensionQuery, FiveDimensionSearchParams as ServiceFiveDimensionSearchParams,
    FiveDimensionService,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 五维统计请求参数
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
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = FiveDimensionService::new(state.db.clone());

    let query = FiveDimensionQuery {
        product_id: params.product_id,
        batch_no: params.batch_no,
        color_no: params.color_no,
        dye_lot_no: params.dye_lot_no,
        grade: params.grade,
        warehouse_id: params.warehouse_id,
        page: params.page,
        // v10 P1-1 修复：page_size clamp(1,100) 防 DoS
        page_size: params.page_size.map(|ps| ps.clamp(1, 100)),
    };

    let stats = service.get_stats(query).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": stats,
        "total": stats.len(),
    }))))
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
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = FiveDimensionService::new(state.db.clone());

    let search_params = ServiceFiveDimensionSearchParams {
        keyword: params.keyword,
        search_type: params.search_type,
        page: params.page,
        // v10 P1-1 修复：page_size clamp(1,100) 防 DoS
        page_size: params.page_size.map(|ps| ps.clamp(1, 100)),
    };

    let (items, total) = service.search(search_params).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
    }))))
}

/// GET /api/v1/erp/five-dimension/list - 列出五维统计
pub async fn list_five_dimension_stats(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<FiveDimensionStatsParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = FiveDimensionService::new(state.db.clone());

    let query = FiveDimensionQuery {
        product_id: params.product_id,
        batch_no: params.batch_no,
        color_no: params.color_no,
        dye_lot_no: params.dye_lot_no,
        grade: params.grade,
        warehouse_id: params.warehouse_id,
        page: params.page,
        // v10 P1-1 修复：page_size clamp(1,100) 防 DoS
        page_size: params.page_size.map(|ps| ps.clamp(1, 100)),
    };

    let stats = service.get_stats(query).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": stats,
        "total": stats.len(),
    }))))
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
    Json(req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = FiveDimensionService::new(state.db.clone());

    let five_dimension_id = req
        .get("five_dimension_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::validation("缺少five_dimension_id参数"))?;

    let stats = service.get_stats_by_id(five_dimension_id).await?;

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
