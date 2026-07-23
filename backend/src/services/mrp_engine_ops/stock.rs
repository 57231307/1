//! 库存查询与物料需求计算
//!
//! 批次 490 D10-3b 拆分：从 mrp_engine_service.rs 抽取的库存查询和需求计算方法。
//! 包含：get_stock_info/get_stock_info_batch/get_stock_info_cached（库存查询）
//! + calculate_requirement_with_stock/calculate_requirement（需求计算）

use rust_decimal::Decimal;
use sea_orm::EntityTrait;
use std::sync::Arc;

use crate::models::inventory_stock::Entity as InventoryStockEntity;
use crate::utils::error::AppError;

use super::types::{MaterialRequirement, RequirementCalcParams, StockInfo};
use crate::services::mrp_engine_service::MrpEngineService;

impl MrpEngineService {
    /// 获取库存信息
    pub(crate) async fn get_stock_info(&self, product_id: i32) -> Result<StockInfo, AppError> {
        let stocks = InventoryStockEntity::find()
            .filter(crate::models::inventory_stock::Column::ProductId.eq(product_id))
            .all(&*self.db)
            .await?;

        let mut on_hand = Decimal::ZERO;
        let mut in_transit = Decimal::ZERO;
        let mut safety_stock = Decimal::ZERO;

        for stock in stocks {
            // 使用面料行业主计量单位（米），兼容通用字段
            let qty = if stock.quantity_meters > Decimal::ZERO {
                stock.quantity_meters
            } else {
                stock.quantity_on_hand
            };
            on_hand += qty;
            in_transit += stock.quantity_incoming;
            safety_stock += stock.reorder_point;
        }

        let available = on_hand - safety_stock;
        let available = if available > Decimal::ZERO {
            available
        } else {
            Decimal::ZERO
        };

        Ok(StockInfo {
            on_hand,
            in_transit,
            safety_stock,
            available,
        })
    }

    /// v16 批次 43 修复：批量获取多个产品的库存信息，避免循环内逐个查询（N+1）
    pub(crate) async fn get_stock_info_batch(
        &self,
        product_ids: &[i32],
    ) -> Result<std::collections::HashMap<i32, StockInfo>, AppError> {
        if product_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let stocks = InventoryStockEntity::find()
            .filter(crate::models::inventory_stock::Column::ProductId.is_in(product_ids.to_vec()))
            .all(&*self.db)
            .await?;

        // 按 product_id 聚合（一个产品可能有多条库存记录，与 get_stock_info 语义一致）
        let mut agg: std::collections::HashMap<i32, (Decimal, Decimal, Decimal)> =
            std::collections::HashMap::new();
        for stock in stocks {
            let qty = if stock.quantity_meters > Decimal::ZERO {
                stock.quantity_meters
            } else {
                stock.quantity_on_hand
            };
            let entry = agg
                .entry(stock.product_id)
                .or_insert((Decimal::ZERO, Decimal::ZERO, Decimal::ZERO));
            entry.0 += qty; // on_hand
            entry.1 += stock.quantity_incoming; // in_transit
            entry.2 += stock.reorder_point; // safety_stock
        }

        let mut result = std::collections::HashMap::new();
        for (product_id, (on_hand, in_transit, safety_stock)) in agg {
            let available = on_hand - safety_stock;
            let available = if available > Decimal::ZERO {
                available
            } else {
                Decimal::ZERO
            };
            result.insert(
                product_id,
                StockInfo {
                    on_hand,
                    in_transit,
                    safety_stock,
                    available,
                },
            );
        }

        Ok(result)
    }

    /// v16 批次 43 修复：带缓存的库存查询，先查 cache 未命中再查数据库并写入 cache
    pub(crate) async fn get_stock_info_cached(
        &self,
        product_id: i32,
        cache: &mut std::collections::HashMap<i32, StockInfo>,
    ) -> Result<StockInfo, AppError> {
        if let Some(info) = cache.get(&product_id) {
            return Ok(info.clone());
        }
        let info = self.get_stock_info(product_id).await?;
        cache.insert(product_id, info.clone());
        Ok(info)
    }

    /// v16 批次 43 修复：基于已知 StockInfo 计算物料需求（避免重复查询库存）
    ///
    /// 批次 352 v12 复审 P1-1 修复：签名从 10 参数改为参数对象 `RequirementCalcParams` + `&StockInfo`，
    /// 消除 `clippy::too_many_arguments` 警告。与 `calculate_requirement` 共用同一参数对象。
    pub(crate) fn calculate_requirement_with_stock(
        &self,
        params: RequirementCalcParams,
        stock_info: &StockInfo,
    ) -> MaterialRequirement {
        let mut available = stock_info.available;
        if params.consider_in_transit {
            available += stock_info.in_transit;
        }

        let shortage = if params.required_quantity > available {
            params.required_quantity - available
        } else {
            Decimal::ZERO
        };

        MaterialRequirement {
            product_id: params.product_id,
            required_quantity: params.required_quantity,
            required_date: params.required_date,
            on_hand_quantity: stock_info.on_hand,
            in_transit_quantity: stock_info.in_transit,
            safety_stock: if params.consider_safety_stock {
                stock_info.safety_stock
            } else {
                Decimal::ZERO
            },
            available_quantity: available,
            shortage_quantity: shortage,
            source_type: params.source_type,
            source_id: params.source_id,
            bom_level: params.bom_level,
        }
    }

    /// 计算单个物料需求
    ///
    /// 批次 336 v10 复审 P3 修复：签名从 8 参数改为单一参数对象 `RequirementCalcParams`，
    /// 消除 `clippy::too_many_arguments` 警告。
    pub async fn calculate_requirement(
        &self,
        params: RequirementCalcParams,
    ) -> Result<MaterialRequirement, AppError> {
        // 解构参数对象，便于函数体内按字段名访问
        let RequirementCalcParams {
            product_id,
            required_quantity,
            required_date,
            source_type,
            source_id,
            consider_safety_stock,
            consider_in_transit,
            bom_level,
        } = params;

        let stock_info = self.get_stock_info(product_id).await?;

        let mut available = stock_info.available;
        if consider_in_transit {
            available += stock_info.in_transit;
        }

        let shortage = if required_quantity > available {
            required_quantity - available
        } else {
            Decimal::ZERO
        };

        Ok(MaterialRequirement {
            product_id,
            required_quantity,
            required_date,
            on_hand_quantity: stock_info.on_hand,
            in_transit_quantity: stock_info.in_transit,
            safety_stock: if consider_safety_stock {
                stock_info.safety_stock
            } else {
                Decimal::ZERO
            },
            available_quantity: available,
            shortage_quantity: shortage,
            source_type,
            source_id,
            bom_level,
        })
    }
}
