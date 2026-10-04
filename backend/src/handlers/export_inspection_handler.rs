//! 出口商检处理器
//! V15 P2 B08-12
use crate::container::AppState;
use crate::services::export_inspection_service::{ExportInspectionService, ListParams};
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

/// 出口商检单列表（分页）。
///
/// 出参遵循全站统一成功信封 `ApiResponse`，分页载荷为 `PaginatedResponse{items,total,page,page_size}`
/// （与 handlers/purchase_inspection_handler.rs 同形，本仓仅此一种分页信封）。
/// `page`/`page_size` 回显 handler 收到的分页入参（缺省 page=1/page_size=20，与 service 内部取值一致）。
pub async fn list_inspections(
    State(state): State<AppState>,
    Query(params): Query<ListQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ExportInspectionService::new(state.db.clone());
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);
    let (items, total) = service
        .list(ListParams {
            sales_order_id: params.sales_order_id,
            inspection_no: params.inspection_no,
            result: params.result,
            page: params.page,
            page_size: params.page_size,
        })
        .await?;
    let data = serde_json::to_value(PaginatedResponse::new(items, total, page, page_size))?;
    Ok(Json(ApiResponse::success(data)))
}

/// 出口商检单详情。
///
/// 出参遵循全站统一成功信封 `ApiResponse`，data 为 `export_inspection::Model` 序列化对象。
pub async fn get_inspection(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ExportInspectionService::new(state.db.clone());
    let item = service.get_by_id(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(item)?)))
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct ListQuery {
    sales_order_id: Option<i32>,
    inspection_no: Option<String>,
    result: Option<String>,
    page: Option<u64>,
    page_size: Option<u64>,
}
