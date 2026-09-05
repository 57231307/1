//! 匹号（布卷）查询处理器
//!
//! 匹号领域四维追溯的最小查询闭环（设计文档 docs/piece-number-domain-design.md）：
//! - 按产品/匹号/匹类型/仓库过滤分页列表
//! - 供前端追溯页与 E2E 四维追溯断言使用

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::{inventory_piece, warehouse};
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    Json,
    extract::{Query, State},
};
use sea_orm::{Condition, PaginatorTrait, QueryOrder};
use serde::Deserialize;

/// 匹号列表查询参数
#[derive(Debug, Deserialize)]
pub struct ListPieceParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    /// 匹号精确匹配
    pub piece_no: Option<String>,
    /// 匹类型：greige=生产匹 / dyed=染色匹
    pub piece_type: Option<String>,
    /// 产品 ID
    pub product_id: Option<i32>,
    /// 仓库 ID
    pub warehouse_id: Option<i32>,
    /// 批次号（生产匹 = 工艺单号；染色匹 = 缸号/染色批次号）
    pub batch_no: Option<String>,
    /// 染色批号
    pub dye_lot_no: Option<String>,
}

/// 匹号列表响应条目（含仓库名便于追溯展示）
#[derive(Debug, serde::Serialize)]
pub struct PieceResponse {
    pub id: i32,
    pub piece_no: String,
    pub piece_type: String,
    pub dye_lot_id: Option<i32>,
    pub dye_lot_no: Option<String>,
    pub machine_no: Option<String>,
    pub machine_operator: Option<String>,
    pub warehouse_in_at: Option<chrono::DateTime<chrono::Utc>>,
    pub length: rust_decimal::Decimal,
    pub weight: Option<rust_decimal::Decimal>,
    pub batch_no: String,
    pub color_no: Option<String>,
    pub product_id: i32,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    pub warehouse_type: Option<String>,
    pub parent_piece_id: Option<i32>,
    pub piece_seq: Option<i32>,
    pub status: String,
    pub quality_status: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// GET /api/v1/erp/inventory/pieces - 匹号分页列表（四维追溯查询）
pub async fn list_pieces(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ListPieceParams>,
) -> Result<Json<ApiResponse<PaginatedResponse<PieceResponse>>>, AppError> {
    let page = params.page.unwrap_or(1).clamp(1, 1000);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    let mut condition = Condition::all();
    if let Some(no) = &params.piece_no {
        condition = condition.add(inventory_piece::Column::PieceNo.eq(no));
    }
    if let Some(pt) = &params.piece_type {
        condition = condition.add(inventory_piece::Column::PieceType.eq(pt));
    }
    if let Some(pid) = params.product_id {
        condition = condition.add(inventory_piece::Column::ProductId.eq(pid));
    }
    if let Some(wid) = params.warehouse_id {
        condition = condition.add(inventory_piece::Column::WarehouseId.eq(wid));
    }
    if let Some(bn) = &params.batch_no {
        condition = condition.add(inventory_piece::Column::BatchNo.eq(bn));
    }
    if let Some(lot) = &params.dye_lot_no {
        condition = condition.add(inventory_piece::Column::DyeLotNo.eq(lot));
    }

    let paginator = inventory_piece::Entity::find()
        .filter(condition)
        .order_by_desc(inventory_piece::Column::CreatedAt)
        .paginate(&state.db, page_size);
    let total = paginator.num_items().await?;
    // SeaORM paginate 使用 0-based 页码
    let models = paginator.fetch_page(page.saturating_sub(1)).await?;

    // 批量取仓库名（避免 N+1：逐条查询改为一次性收集仓库 ID 查询）
    let warehouse_ids: Vec<i32> = models.iter().map(|p| p.warehouse_id).collect();
    let warehouses: std::collections::HashMap<i32, warehouse::Model> =
        warehouse::Entity::find()
            .filter(warehouse::Column::Id.is_in(warehouse_ids))
            .all(&state.db)
            .await?
            .into_iter()
            .map(|w| (w.id, w))
            .collect();

    let items: Vec<PieceResponse> = models
        .into_iter()
        .map(|p| {
            let w = warehouses.get(&p.warehouse_id);
            PieceResponse {
                id: p.id,
                piece_no: p.piece_no,
                piece_type: p.piece_type,
                dye_lot_id: p.dye_lot_id,
                dye_lot_no: p.dye_lot_no,
                machine_no: p.machine_no,
                machine_operator: p.machine_operator,
                warehouse_in_at: p.warehouse_in_at,
                length: p.length,
                weight: p.weight,
                batch_no: p.batch_no,
                color_no: p.color_no,
                product_id: p.product_id,
                warehouse_id: p.warehouse_id,
                warehouse_name: w.map(|w| w.name.clone()),
                warehouse_type: w.and_then(|w| w.warehouse_type.clone()),
                parent_piece_id: p.parent_piece_id,
                piece_seq: p.piece_seq,
                status: p.status,
                quality_status: p.quality_status,
                created_at: p.created_at,
            }
        })
        .collect();

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}
