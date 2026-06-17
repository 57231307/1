//! 五维管理 Service
//!
//! 提供面料五维编码的查询、统计和搜索功能
//! 五维编码：产品ID + 批次号 + 色号 + 缸号 + 等级

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::inventory_stock::{Column as StockColumn, Entity as InventoryStockEntity};
use crate::models::product::{Column as ProductColumn, Entity as ProductEntity};
use crate::utils::error::AppError;

/// 五维统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiveDimensionStats {
    pub product_id: i32,
    pub product_name: String,
    pub batch_no: String,
    pub color_no: String,
    pub dye_lot_no: Option<String>,
    pub grade: String,
    pub five_dimension_id: String,
    pub total_meters: Decimal,
    pub total_kg: Decimal,
    pub stock_count: i64,
    pub warehouse_distribution: Vec<WarehouseStock>,
}

/// 仓库库存分布
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarehouseStock {
    pub warehouse_id: i32,
    pub warehouse_name: String,
    pub quantity_meters: Decimal,
    pub quantity_kg: Decimal,
}

/// 五维查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct FiveDimensionQuery {
    pub product_id: Option<i32>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub warehouse_id: Option<i32>,
    #[allow(dead_code)] // TODO(tech-debt): 五维分析分页接口接入业务后移除
    pub page: Option<u64>,
    #[allow(dead_code)] // TODO(tech-debt): 五维分析分页接口接入业务后移除
    pub page_size: Option<u64>,
}

/// 五维搜索参数
#[derive(Debug, Clone, Deserialize)]
pub struct FiveDimensionSearchParams {
    pub keyword: String,
    pub search_type: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 五维管理 Service
pub struct FiveDimensionService {
    db: Arc<DatabaseConnection>,
}

impl FiveDimensionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取五维统计信息
    pub async fn get_stats(
        &self,
        query: FiveDimensionQuery,
    ) -> Result<Vec<FiveDimensionStats>, AppError> {
        let mut stock_query = InventoryStockEntity::find()
            .filter(StockColumn::StockStatus.eq("ACTIVE"))
            .filter(StockColumn::QuantityAvailable.gt(Decimal::ZERO));

        if let Some(product_id) = query.product_id {
            stock_query = stock_query.filter(StockColumn::ProductId.eq(product_id));
        }
        if let Some(warehouse_id) = query.warehouse_id {
            stock_query = stock_query.filter(StockColumn::WarehouseId.eq(warehouse_id));
        }
        // 在数据库层面进行五维字段筛选
        if let Some(ref batch_no) = query.batch_no {
            stock_query = stock_query.filter(StockColumn::BatchNo.contains(batch_no));
        }
        if let Some(ref color_no) = query.color_no {
            stock_query = stock_query.filter(StockColumn::ColorNo.contains(color_no));
        }
        if let Some(ref dye_lot_no) = query.dye_lot_no {
            stock_query = stock_query.filter(StockColumn::DyeLotNo.contains(dye_lot_no));
        }
        if let Some(ref grade) = query.grade {
            stock_query = stock_query.filter(StockColumn::Grade.contains(grade));
        }

        let stocks = stock_query.all(&*self.db).await?;

        // 获取产品信息
        let product_ids: Vec<i32> = stocks.iter().map(|s| s.product_id).collect();
        let products = if product_ids.is_empty() {
            vec![]
        } else {
            ProductEntity::find()
                .filter(ProductColumn::Id.is_in(product_ids))
                .all(&*self.db)
                .await?
        };

        let product_map: HashMap<i32, String> =
            products.into_iter().map(|p| (p.id, p.name)).collect();

        // 按五维分组统计
        let mut stats_map: HashMap<String, FiveDimensionStats> = HashMap::new();

        for stock in stocks {
            // 从库存记录中提取五维信息
            let batch_no = stock.batch_no.clone();
            let color_no = stock.color_no.clone();
            let dye_lot_no = stock.dye_lot_no.clone();
            let grade = stock.grade.clone();

            let product_name = product_map
                .get(&stock.product_id)
                .cloned()
                .unwrap_or_else(|| "未知产品".to_string());

            let five_dimension_id = format!(
                "P{}|B{}|C{}|D{}|G{}",
                stock.product_id,
                batch_no,
                color_no,
                dye_lot_no.as_deref().unwrap_or("DN"),
                grade
            );

            let stats = stats_map
                .entry(five_dimension_id.clone())
                .or_insert_with(|| FiveDimensionStats {
                    product_id: stock.product_id,
                    product_name: product_name.clone(),
                    batch_no: batch_no.clone(),
                    color_no: color_no.clone(),
                    dye_lot_no: dye_lot_no.clone(),
                    grade: grade.clone(),
                    five_dimension_id: five_dimension_id.clone(),
                    total_meters: Decimal::ZERO,
                    total_kg: Decimal::ZERO,
                    stock_count: 0,
                    warehouse_distribution: Vec::new(),
                });

            stats.total_meters += stock.quantity_available;
            stats.total_kg += stock.quantity_kg;
            stats.stock_count += 1;

            // 添加仓库分布
            if let Some(wh) = stats
                .warehouse_distribution
                .iter_mut()
                .find(|w| w.warehouse_id == stock.warehouse_id)
            {
                wh.quantity_meters += stock.quantity_available;
                wh.quantity_kg += stock.quantity_kg;
            } else {
                stats.warehouse_distribution.push(WarehouseStock {
                    warehouse_id: stock.warehouse_id,
                    warehouse_name: format!("仓库{}", stock.warehouse_id),
                    quantity_meters: stock.quantity_available,
                    quantity_kg: stock.quantity_kg,
                });
            }
        }

        let mut results: Vec<FiveDimensionStats> = stats_map.into_values().collect();

        // 排序
        results.sort_by_key(|a| a.product_id);

        Ok(results)
    }

    /// 按五维ID查询统计
    pub async fn get_stats_by_id(
        &self,
        five_dimension_id: &str,
    ) -> Result<Option<FiveDimensionStats>, AppError> {
        // 解析五维ID（带前缀格式：P{}|B{}|C{}|D{}|G{}）
        let parts: Vec<&str> = five_dimension_id.split('|').collect();
        if parts.len() < 5 {
            return Err(AppError::validation("无效的五维ID格式"));
        }

        let product_id: i32 = parts[0]
            .trim_start_matches('P')
            .parse()
            .map_err(|_| AppError::validation("无效的产品ID"))?;
        let batch_no = parts[1].trim_start_matches('B').to_string();
        let color_no = parts[2].trim_start_matches('C').to_string();
        let dye_lot_part = parts[3].trim_start_matches('D');
        let dye_lot_no = if dye_lot_part == "DN" || dye_lot_part.is_empty() {
            None
        } else {
            Some(dye_lot_part.to_string())
        };
        let grade = parts[4].trim_start_matches('G').to_string();

        let query = FiveDimensionQuery {
            product_id: Some(product_id),
            batch_no: Some(batch_no),
            color_no: Some(color_no),
            dye_lot_no,
            grade: Some(grade),
            warehouse_id: None,
            page: None,
            page_size: None,
        };

        let results = self.get_stats(query).await?;
        Ok(results.into_iter().next())
    }

    /// 搜索五维
    pub async fn search(
        &self,
        params: FiveDimensionSearchParams,
    ) -> Result<(Vec<FiveDimensionStats>, u64), AppError> {
        // 分页参数默认 0（首页），与全局分页约定一致
        let page = params.page.unwrap_or_default();
        let page_size = params.page_size.unwrap_or(20);

        let mut query = FiveDimensionQuery {
            product_id: None,
            batch_no: None,
            color_no: None,
            dye_lot_no: None,
            grade: None,
            warehouse_id: None,
            page: Some(page),
            page_size: Some(page_size),
        };

        // 根据搜索类型设置查询条件
        match params.search_type.as_deref() {
            Some("product") => {
                // 按产品名称搜索 - 需要先查找产品ID
                let products = ProductEntity::find()
                    .filter(ProductColumn::Name.contains(&params.keyword))
                    .all(&*self.db)
                    .await?;

                if products.is_empty() {
                    return Ok((vec![], 0));
                }

                // 如果只有一个匹配产品，直接按产品ID查询
                if products.len() == 1 {
                    query.product_id = Some(products[0].id);
                }
            }
            Some("batch") => {
                query.batch_no = Some(params.keyword.clone());
            }
            Some("color") => {
                query.color_no = Some(params.keyword.clone());
            }
            Some("dye_lot") => {
                query.dye_lot_no = Some(params.keyword.clone());
            }
            Some("grade") => {
                query.grade = Some(params.keyword.clone());
            }
            _ => {
                // 通用搜索 - 默认按批次号搜索
                query.batch_no = Some(params.keyword.clone());
            }
        }

        let results = self.get_stats(query).await?;
        let total = results.len() as u64;

        Ok((results, total))
    }

    /// 获取五维统计汇总
    pub async fn get_summary(&self) -> Result<serde_json::Value, AppError> {
        let all_stats = self
            .get_stats(FiveDimensionQuery {
                product_id: None,
                batch_no: None,
                color_no: None,
                dye_lot_no: None,
                grade: None,
                warehouse_id: None,
                page: None,
                page_size: None,
            })
            .await?;

        let total_products: i32 = all_stats
            .iter()
            .map(|s| s.product_id)
            .collect::<std::collections::HashSet<_>>()
            .len() as i32;

        let total_batches: i32 = all_stats
            .iter()
            .map(|s| &s.batch_no)
            .collect::<std::collections::HashSet<_>>()
            .len() as i32;

        let total_colors: i32 = all_stats
            .iter()
            .map(|s| &s.color_no)
            .collect::<std::collections::HashSet<_>>()
            .len() as i32;

        let total_meters: Decimal = all_stats.iter().map(|s| s.total_meters).sum();
        let total_kg: Decimal = all_stats.iter().map(|s| s.total_kg).sum();
        let total_stock_count: i64 = all_stats.iter().map(|s| s.stock_count).sum();

        Ok(serde_json::json!({
            "total_products": total_products,
            "total_batches": total_batches,
            "total_colors": total_colors,
            "total_meters": total_meters,
            "total_kg": total_kg,
            "total_stock_count": total_stock_count,
            "five_dimension_count": all_stats.len(),
        }))
    }
}
