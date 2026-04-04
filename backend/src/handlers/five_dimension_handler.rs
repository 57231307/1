use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use crate::utils::app_state::AppState;
use serde::{Deserialize, Serialize};

use crate::utils::fabric_five_dimension::FabricFiveDimension;

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
    State(_state): State<AppState>,
    Query(params): Query<FiveDimensionStatsParams>,
) -> Result<Json<FiveDimensionStatsResponse>, (StatusCode, String)> {
    // TODO: 实现数据库查询逻辑
    // 这里先返回示例数据

    let dimension = FabricFiveDimension::new(
        params.product_id.unwrap_or(1),
        params.batch_no.unwrap_or_else(|| "B20240101".to_string()),
        params.color_no.unwrap_or_else(|| "C001".to_string()),
        params.dye_lot_no,
        params.grade.unwrap_or_else(|| "一等品".to_string()),
    );

    let five_dimension_id = dimension.generate_unique_id();
    let response = FiveDimensionStatsResponse {
        dimension: FiveDimensionItem {
            product_id: dimension.product_id,
            product_name: Some("示例面料".to_string()),
            batch_no: dimension.batch_no,
            color_no: dimension.color_no,
            dye_lot_no: dimension.dye_lot_no,
            grade: dimension.grade,
            five_dimension_id,
        },
        total_meters: Decimal::from(1000),
        total_kg: Decimal::from(32),
        stock_count: 5,
        warehouse_distribution: vec![
            WarehouseDistribution {
                warehouse_id: 1,
                warehouse_name: "主仓库".to_string(),
                quantity_meters: Decimal::from(600),
                quantity_kg: Decimal::from_f64(19.2).unwrap(),
            },
            WarehouseDistribution {
                warehouse_id: 2,
                warehouse_name: "次仓库".to_string(),
                quantity_meters: Decimal::from(400),
                quantity_kg: Decimal::from_f64(12.8).unwrap(),
            },
        ],
    };

    Ok(Json(response))
}

/// 按五维 ID 查询统计信息
pub async fn get_stats_by_five_dimension_id(
    State(_state): State<AppState>,
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

    let five_dimension_id = dimension.generate_unique_id();

    // TODO: 实现数据库查询逻辑
    let response = FiveDimensionStatsResponse {
        dimension: FiveDimensionItem {
            product_id: dimension.product_id,
            product_name: Some("示例面料".to_string()),
            batch_no: dimension.batch_no,
            color_no: dimension.color_no,
            dye_lot_no: dimension.dye_lot_no,
            grade: dimension.grade,
            five_dimension_id,
        },
        total_meters: Decimal::from(500),
        total_kg: Decimal::from(16),
        stock_count: 2,
        warehouse_distribution: vec![WarehouseDistribution {
            warehouse_id: 1,
            warehouse_name: "主仓库".to_string(),
            quantity_meters: Decimal::from(500),
            quantity_kg: Decimal::from(16),
        }],
    };

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
    State(_state): State<AppState>,
    Query(params): Query<FiveDimensionSearchParams>,
) -> Result<Json<FiveDimensionSearchResponse>, (StatusCode, String)> {
    let page = params.page.unwrap_or(0);
    let page_size = params.page_size.unwrap_or(20);

    // TODO: 实现数据库搜索逻辑
    // 这里返回示例数据

    let items = vec![
        FiveDimensionItem {
            product_id: 1,
            product_name: Some("面料 A".to_string()),
            batch_no: "B20240101".to_string(),
            color_no: "C001".to_string(),
            dye_lot_no: Some("D20240101001".to_string()),
            grade: "一等品".to_string(),
            five_dimension_id: "P1|B20240101|C001|D20240101001|G 一等品".to_string(),
        },
        FiveDimensionItem {
            product_id: 2,
            product_name: Some("面料 B".to_string()),
            batch_no: "B20240102".to_string(),
            color_no: "C002".to_string(),
            dye_lot_no: None,
            grade: "二等品".to_string(),
            five_dimension_id: "P2|B20240102|C002|DN|G 二等品".to_string(),
        },
    ];

    Ok(Json(FiveDimensionSearchResponse {
        items,
        total: 2,
        page,
        page_size,
    }))
}

/// 列出所有五维统计
pub async fn list_five_dimension_stats(
    State(_state): State<AppState>,
    Query(params): Query<FiveDimensionStatsParams>,
) -> Result<Json<FiveDimensionListResponse>, (StatusCode, String)> {
    let page = params.page.unwrap_or(0);
    let page_size = params.page_size.unwrap_or(20);

    // TODO: 实现数据库查询逻辑
    let items = vec![FiveDimensionStatsResponse {
        dimension: FiveDimensionItem {
            product_id: 1,
            product_name: Some("面料 A".to_string()),
            batch_no: "B20240101".to_string(),
            color_no: "C001".to_string(),
            dye_lot_no: Some("D20240101001".to_string()),
            grade: "一等品".to_string(),
            five_dimension_id: "P1|B20240101|C001|D20240101001|G 一等品".to_string(),
        },
        total_meters: Decimal::from(1000),
        total_kg: Decimal::from(32),
        stock_count: 5,
        warehouse_distribution: vec![],
    }];

    Ok(Json(FiveDimensionListResponse {
        items,
        total: 1,
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
