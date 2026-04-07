use crate::utils::app_state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
use serde::{Deserialize, Serialize};

use crate::models::{inventory_stock, product, warehouse};
use crate::utils::fabric_five_dimension::FabricFiveDimension;

/// 内部辅助函数：从数据库查询五维统计
async fn get_stats_from_db(
    db: &sea_orm::DatabaseConnection,
    dimension: &FabricFiveDimension,
) -> Result<FiveDimensionStatsResponse, String> {
    let mut query = inventory_stock::Entity::find()
        .filter(inventory_stock::Column::ProductId.eq(dimension.product_id))
        .filter(inventory_stock::Column::BatchNo.eq(dimension.batch_no.clone()))
        .filter(inventory_stock::Column::ColorNo.eq(dimension.color_no.clone()))
        .filter(inventory_stock::Column::Grade.eq(dimension.grade.clone()));

    if let Some(dye_lot) = &dimension.dye_lot_no {
        query = query.filter(inventory_stock::Column::DyeLotNo.eq(dye_lot.clone()));
    } else {
        query = query.filter(inventory_stock::Column::DyeLotNo.is_null());
    }

    let stocks = query.all(db).await.map_err(|e| e.to_string())?;

    let product_model = product::Entity::find_by_id(dimension.product_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?;

    let product_name = product_model.map(|p| p.name);

    let mut total_meters = Decimal::ZERO;
    let mut total_kg = Decimal::ZERO;
    let mut stock_count = 0;

    let mut warehouse_map: std::collections::HashMap<i32, (Decimal, Decimal)> =
        std::collections::HashMap::new();

    for stock in stocks {
        total_meters += stock.quantity_meters;
        total_kg += stock.quantity_kg;
        stock_count += 1;

        let entry = warehouse_map
            .entry(stock.warehouse_id)
            .or_insert((Decimal::ZERO, Decimal::ZERO));
        entry.0 += stock.quantity_meters;
        entry.1 += stock.quantity_kg;
    }

    let mut warehouse_distribution = Vec::new();
    for (wid, (m, kg)) in warehouse_map {
        let wh = warehouse::Entity::find_by_id(wid)
            .one(db)
            .await
            .map_err(|e| e.to_string())?;
        let warehouse_name = wh.map(|w| w.name).unwrap_or_else(|| "未知仓库".to_string());
        warehouse_distribution.push(WarehouseDistribution {
            warehouse_id: wid,
            warehouse_name,
            quantity_meters: m,
            quantity_kg: kg,
        });
    }

    let five_dimension_id = dimension.generate_unique_id();
    Ok(FiveDimensionStatsResponse {
        dimension: FiveDimensionItem {
            product_id: dimension.product_id,
            product_name,
            batch_no: dimension.batch_no.clone(),
            color_no: dimension.color_no.clone(),
            dye_lot_no: dimension.dye_lot_no.clone(),
            grade: dimension.grade.clone(),
            five_dimension_id,
        },
        total_meters,
        total_kg,
        stock_count,
        warehouse_distribution,
    })
}

/// 五维统计请求参数
#[derive(Debug, Deserialize)]
pub struct FiveDimensionStatsParams {
    pub product_id: Option<i32>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    #[allow(dead_code)]
    pub warehouse_id: Option<i32>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 五维统计响应
#[derive(Debug, Serialize)]
pub struct FiveDimensionStatsResponse {
    /// 五维对象
    pub dimension: FiveDimensionItem,
    /// 总米数
    pub total_meters: Decimal,
    /// 总公斤数
    pub total_kg: Decimal,
    /// 库存记录数
    pub stock_count: i64,
    /// 仓库分布
    pub warehouse_distribution: Vec<WarehouseDistribution>,
}

/// 五维对象（简化版）
#[derive(Debug, Serialize)]
pub struct FiveDimensionItem {
    pub product_id: i32,
    pub product_name: Option<String>,
    pub batch_no: String,
    pub color_no: String,
    pub dye_lot_no: Option<String>,
    pub grade: String,
    pub five_dimension_id: String,
}

/// 仓库分布
#[derive(Debug, Serialize)]
pub struct WarehouseDistribution {
    pub warehouse_id: i32,
    pub warehouse_name: String,
    pub quantity_meters: Decimal,
    pub quantity_kg: Decimal,
}

/// 五维列表响应
#[derive(Debug, Serialize)]
pub struct FiveDimensionListResponse {
    pub items: Vec<FiveDimensionStatsResponse>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 五维 ID 查询请求
#[derive(Debug, Deserialize)]
pub struct FiveDimensionIdRequest {
    pub five_dimension_id: String,
}

/// 五维 ID 解析响应
#[derive(Debug, Serialize)]
pub struct FiveDimensionIdParseResponse {
    pub success: bool,
    pub dimension: Option<FiveDimensionItem>,
    pub error: Option<String>,
}

/// 五维搜索参数
#[derive(Debug, Deserialize)]
pub struct FiveDimensionSearchParams {
    #[allow(dead_code)]
    pub keyword: String,
    #[allow(dead_code)]
    pub search_type: String, // product, batch, color, dye_lot, grade
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 五维搜索结果
#[derive(Debug, Serialize)]
pub struct FiveDimensionSearchResponse {
    pub items: Vec<FiveDimensionItem>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 获取五维统计信息
pub async fn get_five_dimension_stats(
    State(state): State<AppState>,
    Query(params): Query<FiveDimensionStatsParams>,
) -> Result<Json<FiveDimensionStatsResponse>, (StatusCode, String)> {
    let product_id = params.product_id.unwrap_or(1);
    let batch_no = params.batch_no.unwrap_or_else(|| "B20240101".to_string());
    let color_no = params.color_no.unwrap_or_else(|| "C001".to_string());
    let grade = params.grade.unwrap_or_else(|| "一等品".to_string());

    let dimension =
        FabricFiveDimension::new(product_id, batch_no, color_no, params.dye_lot_no, grade);

    let response = get_stats_from_db(&state.db, &dimension)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(response))
}

/// 按五维 ID 查询统计信息
pub async fn get_stats_by_five_dimension_id(
    State(state): State<AppState>,
    Path(five_dimension_id): Path<String>,
) -> Result<Json<FiveDimensionStatsResponse>, (StatusCode, String)> {
    // 解析五维 ID
    let dimension = match FabricFiveDimension::from_unique_id(&five_dimension_id) {
        Ok(dim) => dim,
        Err(e) => {
            return Err((StatusCode::BAD_REQUEST, format!("无效的五维 ID: {}", e)));
        }
    };

    // 验证五维数据
    if let Err(e) = dimension.validate() {
        return Err((StatusCode::BAD_REQUEST, format!("五维数据验证失败：{}", e)));
    }

    let response = get_stats_from_db(&state.db, &dimension)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(response))
}

/// 解析五维 ID
pub async fn parse_five_dimension_id(
    Json(req): Json<FiveDimensionIdRequest>,
) -> Result<Json<FiveDimensionIdParseResponse>, (StatusCode, String)> {
    match FabricFiveDimension::from_unique_id(&req.five_dimension_id) {
        Ok(dimension) => {
            let five_dimension_id = dimension.generate_unique_id();
            let response = FiveDimensionIdParseResponse {
                success: true,
                dimension: Some(FiveDimensionItem {
                    product_id: dimension.product_id,
                    product_name: None,
                    batch_no: dimension.batch_no,
                    color_no: dimension.color_no,
                    dye_lot_no: dimension.dye_lot_no,
                    grade: dimension.grade,
                    five_dimension_id,
                }),
                error: None,
            };
            Ok(Json(response))
        }
        Err(e) => {
            let response = FiveDimensionIdParseResponse {
                success: false,
                dimension: None,
                error: Some(e),
            };
            Ok(Json(response))
        }
    }
}

/// 五维搜索
pub async fn search_five_dimension(
    State(state): State<AppState>,
    Query(params): Query<FiveDimensionSearchParams>,
) -> Result<Json<FiveDimensionSearchResponse>, (StatusCode, String)> {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);

    let mut query = inventory_stock::Entity::find()
        .select_only()
        .column(inventory_stock::Column::ProductId)
        .column(inventory_stock::Column::BatchNo)
        .column(inventory_stock::Column::ColorNo)
        .column(inventory_stock::Column::DyeLotNo)
        .column(inventory_stock::Column::Grade)
        .distinct();

    if !params.keyword.is_empty() {
        let keyword_pattern = format!("%{}%", params.keyword);
        match params.search_type.as_str() {
            "batch" => {
                query = query.filter(inventory_stock::Column::BatchNo.like(&keyword_pattern))
            }
            "color" => {
                query = query.filter(inventory_stock::Column::ColorNo.like(&keyword_pattern))
            }
            "dye_lot" => {
                query = query.filter(inventory_stock::Column::DyeLotNo.like(&keyword_pattern))
            }
            "grade" => query = query.filter(inventory_stock::Column::Grade.like(&keyword_pattern)),
            _ => {}
        }
    }

    let items_json: Vec<serde_json::Value> = query
        .into_json()
        .all(state.db.as_ref())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total = items_json.len() as u64;

    let start = ((page.saturating_sub(1)) * page_size) as usize;
    let end = (start + page_size as usize).min(items_json.len());

    let mut items = Vec::new();
    if start < items_json.len() {
        for val in &items_json[start..end] {
            let product_id = val.get("product_id").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
            let batch_no = val
                .get("batch_no")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let color_no = val
                .get("color_no")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let dye_lot_no = val
                .get("dye_lot_no")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let grade = val
                .get("grade")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let product_model = product::Entity::find_by_id(product_id)
                .one(state.db.as_ref())
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            let product_name = product_model.map(|p| p.name);

            let dim = FabricFiveDimension::new(
                product_id,
                batch_no.clone(),
                color_no.clone(),
                dye_lot_no.clone(),
                grade.clone(),
            );
            let five_dimension_id = dim.generate_unique_id();

            items.push(FiveDimensionItem {
                product_id,
                product_name,
                batch_no,
                color_no,
                dye_lot_no,
                grade,
                five_dimension_id,
            });
        }
    }

    Ok(Json(FiveDimensionSearchResponse {
        items,
        total,
        page,
        page_size,
    }))
}

/// 列出所有五维统计
pub async fn list_five_dimension_stats(
    State(state): State<AppState>,
    Query(params): Query<FiveDimensionStatsParams>,
) -> Result<Json<FiveDimensionListResponse>, (StatusCode, String)> {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);

    let mut query = inventory_stock::Entity::find()
        .select_only()
        .column(inventory_stock::Column::ProductId)
        .column(inventory_stock::Column::BatchNo)
        .column(inventory_stock::Column::ColorNo)
        .column(inventory_stock::Column::DyeLotNo)
        .column(inventory_stock::Column::Grade)
        .distinct();

    if let Some(pid) = params.product_id {
        query = query.filter(inventory_stock::Column::ProductId.eq(pid));
    }
    if let Some(b) = &params.batch_no {
        query = query.filter(inventory_stock::Column::BatchNo.eq(b.clone()));
    }
    if let Some(c) = &params.color_no {
        query = query.filter(inventory_stock::Column::ColorNo.eq(c.clone()));
    }
    if let Some(d) = &params.dye_lot_no {
        query = query.filter(inventory_stock::Column::DyeLotNo.eq(d.clone()));
    }
    if let Some(g) = &params.grade {
        query = query.filter(inventory_stock::Column::Grade.eq(g.clone()));
    }

    let items_json: Vec<serde_json::Value> = query
        .into_json()
        .all(state.db.as_ref())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total = items_json.len() as u64;

    let start = ((page.saturating_sub(1)) * page_size) as usize;
    let end = (start + page_size as usize).min(items_json.len());

    let mut items = Vec::new();
    if start < items_json.len() {
        for val in &items_json[start..end] {
            let product_id = val.get("product_id").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
            let batch_no = val
                .get("batch_no")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let color_no = val
                .get("color_no")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let dye_lot_no = val
                .get("dye_lot_no")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let grade = val
                .get("grade")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let dimension =
                FabricFiveDimension::new(product_id, batch_no, color_no, dye_lot_no, grade);
            if let Ok(stats) = get_stats_from_db(&state.db, &dimension).await {
                items.push(stats);
            }
        }
    }

    Ok(Json(FiveDimensionListResponse {
        items,
        total,
        page,
        page_size,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_five_dimension_item_serialization() {
        let item = FiveDimensionItem {
            product_id: 1,
            product_name: Some("测试面料".to_string()),
            batch_no: "B20240101".to_string(),
            color_no: "C001".to_string(),
            dye_lot_no: Some("D20240101001".to_string()),
            grade: "一等品".to_string(),
            five_dimension_id: "P1|B20240101|C001|D20240101001|G 一等品".to_string(),
        };

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("测试面料"));
        assert!(json.contains("B20240101"));
    }
}
