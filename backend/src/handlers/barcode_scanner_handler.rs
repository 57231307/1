
use axum::{extract::Query, extract::State, Json};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use serde::Deserialize;
use tracing::info;

use crate::middleware::auth_context::AuthContext;
use crate::models::inventory_piece;
// 批次 236 v13 P1-1：库存裁片状态常量接入（规则 0）
use crate::models::status::inventory_piece as piece_status;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

#[derive(Deserialize)]
pub struct ScanToShipRequest {
    pub barcode: String,
    pub order_id: i32,
}

#[derive(Deserialize)]
pub struct ScanToShipQuery {
    pub barcode: Option<String>,
    pub order_id: Option<i32>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 扫码盘库请求参数
#[derive(Debug, Deserialize)]
pub struct ScanInventoryParams {
    pub barcode: Option<String>,
}

/// 扫码历史查询参数
#[derive(Debug, Deserialize)]
pub struct ScanHistoryQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    // v11 批次 153 P2-A：接入 scan_type filter（inventory_piece.scan_type 列已通过 m0043 迁移添加）
    // 值如 SHIP=扫码发货，INVENTORY=扫码盘库
    pub scan_type: Option<String>,
    pub result: Option<String>,
}

pub async fn scan_to_ship_get(
    State(state): State<AppState>,
    Query(query): Query<ScanToShipQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 如果没有 barcode 参数，返回扫码记录列表
    let barcode = match query.barcode {
        Some(b) => b,
        None => {
            // 返回空列表或扫码历史记录
            // v13 批次 40 修复：page_size clamp(1,100) 统一规范，防止前端误用超大值
            return Ok(Json(ApiResponse::success(serde_json::json!({
                "items": [],
                "total": 0,
                "page": query.page.unwrap_or(1).clamp(1, 1000), // 批次 95 P3-3~8：分页 clamp 防 DoS
                "page_size": query.page_size.unwrap_or(20).clamp(1, 100)
            }))));
        }
    };
    scan_to_ship_impl(state, barcode, query.order_id.unwrap_or_default()).await
}

pub async fn scan_to_ship_post(
    State(state): State<AppState>,
    Json(req): Json<ScanToShipRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    scan_to_ship_impl(state, req.barcode, req.order_id).await
}

async fn scan_to_ship_impl(
    state: AppState,
    barcode: String,
    _order_id: i32,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let txn = state.db.begin().await?;

    let piece = inventory_piece::Entity::find()
        .filter(inventory_piece::Column::Barcode.eq(&barcode))
        .one(&txn)
        .await?
        .ok_or_else(|| AppError::not_found("未找到该条码对应的布卷"))?;

    if piece.status == piece_status::SHIPPED {
        return Err(AppError::bad_request("该布卷已发货"));
    }

    let mut active_piece: inventory_piece::ActiveModel = piece.clone().into();
    active_piece.status = Set(piece_status::SHIPPED.to_string());
    active_piece.updated_at = Set(Utc::now());
    active_piece.update(&txn).await?;

    txn.commit().await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "布卷扫码出库成功",
        "barcode": barcode,
        "piece_no": piece.piece_no
    }))))
}

/// 扫码盘库：按条码查询库存布卷详情
pub async fn scan_inventory(
    State(state): State<AppState>,
    Query(params): Query<ScanInventoryParams>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let barcode = params
        .barcode
        .ok_or_else(|| AppError::bad_request("缺少 barcode 参数"))?;

    let piece = inventory_piece::Entity::find()
        .filter(inventory_piece::Column::Barcode.eq(&barcode))
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("未找到该条码对应的布卷"))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "barcode": piece.barcode,
        "piece_no": piece.piece_no,
        "product_id": piece.product_id,
        "batch_no": piece.batch_no,
        "warehouse_id": piece.warehouse_id,
        "location_id": piece.location_id,
        "length": piece.length,
        "weight": piece.weight,
        "status": piece.status,
        "remarks": piece.remarks,
    }))))
}

/// 扫码历史记录：分页查询已扫码的布卷
pub async fn scan_history(
    State(state): State<AppState>,
    Query(params): Query<ScanHistoryQuery>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 页码采用 1-based 约定，与全局分页契约保持一致
    let page = params.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    // v11 批次 36 修复：page_size clamp 防止 DoS（用户传超大值导致内存爆炸）
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    let mut query = inventory_piece::Entity::find();

    // v11 批次 153 P2-A：接入 scan_type filter（精确匹配 scan_type 列）
    if let Some(scan_type) = &params.scan_type {
        query = query.filter(inventory_piece::Column::ScanType.eq(scan_type));
    }

    if let Some(result) = &params.result {
        match result.as_str() {
            "SUCCESS" => {
                query = query.filter(inventory_piece::Column::Status.eq(piece_status::SHIPPED));
            }
            "FAILED" => {
                query = query.filter(inventory_piece::Column::Status.eq(piece_status::DEFECT));
            }
            _ => {}
        }
    }

    let paginator = query
        .clone()
        .order_by_desc(inventory_piece::Column::UpdatedAt)
        .paginate(&*state.db, page_size);

    let total = paginator.num_items().await?;
    // fetch_page 接收 0-based 页码，需将 1-based page 转换
    // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
    let items = paginator.fetch_page(page.clamp(1, 1000).saturating_sub(1)).await?;

    info!(
        "扫码历史查询 page={} page_size={} total={}",
        page, page_size, total
    );

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// 扫码统计：按状态统计库存匹数
pub async fn scan_statistics(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use sea_orm::sea_query::Expr;
    use sea_orm::QuerySelect;

    let row = inventory_piece::Entity::find()
        .select_only()
        .column_as(
            Expr::cust(format!(
                "COUNT(*) FILTER (WHERE status = '{}')",
                piece_status::AVAILABLE
            )),
            "available_count",
        )
        .column_as(
            Expr::cust(format!(
                "COUNT(*) FILTER (WHERE status = '{}')",
                piece_status::SHIPPED
            )),
            "shipped_count",
        )
        .column_as(
            Expr::cust(format!(
                "COUNT(*) FILTER (WHERE status = '{}')",
                piece_status::DEFECT
            )),
            "defect_count",
        )
        .column_as(
            Expr::cust(format!(
                "COUNT(*) FILTER (WHERE status = '{}')",
                piece_status::RESERVED
            )),
            "reserved_count",
        )
        .column_as(Expr::cust("COUNT(*)"), "total_count")
        .into_model::<ScanStatisticsRow>()
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::internal("扫码统计查询失败"))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "available_count": row.available_count,
        "shipped_count": row.shipped_count,
        "defect_count": row.defect_count,
        "reserved_count": row.reserved_count,
        "total_count": row.total_count,
    }))))
}

#[derive(serde::Serialize, sea_orm::FromQueryResult)]
struct ScanStatisticsRow {
    pub available_count: i64,
    pub shipped_count: i64,
    pub defect_count: i64,
    pub reserved_count: i64,
    pub total_count: i64,
}
