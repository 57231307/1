//! 库存服务 - 库存检查辅助（inv/stock）
//!
//! 提供调拨单创建时的源仓库库存预检逻辑。
//! 由 `move::create_transfer` 调用以确保调出仓库有充足库存。
//!
//! 拆分自原 `inventory_transfer_service.rs` 的 `check_from_warehouse_inventory` 私有方法。

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::models::inventory_stock::{self, Column as StockColumn, Entity as InventoryStockEntity};
use crate::services::inventory_deduction::{OutboundDimensions, require_outbound_dimensions};
use crate::utils::error::AppError;

use super::{InventoryTransferItemRequest, InventoryTransferService};

impl InventoryTransferService {
    /// 检查调出仓库库存是否充足（在调拨单创建事务中调用，确保所有调拨明细在源仓库有足够库存。；采用批量查询优化 N+1：先一次性查出所有相关 product 的库存记录，；再用内存匹配按追溯维度定位命中行。）
    ///
    /// 维度口径与出库真实扣减（`inv::batch::apply_ship_item_deduction`）对齐，判定唯一来源为
    /// 全仓实现 [`fabric_class::validate_fabric_trace`]（经 [`require_outbound_dimensions`] 包装）：
    /// 染色布（色号非空）缺缸号在此即返回明确业务错误（4xx，禁裸 500）；白坯（色号为空）维持宽松、
    /// 仅按款号 + 批次匹配、不强制缸号，保持既有放行行为避免回归。
    pub(crate) async fn check_from_warehouse_inventory(
        &self,
        from_warehouse_id: &i32,
        items: &[InventoryTransferItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        // 批量获取库存记录（优化N+1查询）
        // 跳过 product_id 为 None 的项，避免脏 product_id=0 污染查询
        let product_ids: Vec<i32> = items.iter().filter_map(|item| item.product_id).collect();
        let stocks = InventoryStockEntity::find()
            .filter(StockColumn::WarehouseId.eq(*from_warehouse_id))
            .filter(StockColumn::ProductId.is_in(product_ids))
            .all(txn)
            .await?;

        for item in items {
            // 上游已校验 product_id 必填；None 项直接跳过
            let Some(product_id) = item.product_id else {
                continue;
            };
            let quantity = item.quantity.unwrap_or(Decimal::ZERO);

            // 复用 fabric_class 唯一判定（经出库侧包装为带单据措辞的业务错误）：
            // 白坯布归一为空色号 + 无缸号；染色布缺缸号在此返回与 fabric_class 同口径的明确 4xx 错误。
            let dims = require_outbound_dimensions(
                "调拨出库",
                product_id,
                item.color_no.as_deref(),
                item.dye_lot_no.as_deref(),
                item.batch_no.as_deref(),
            )?;

            // 按归一维度在调出仓库存行中精确匹配并校验数量
            match_single_item_against_stocks(product_id, &dims, quantity, &stocks)?;
        }
        Ok(())
    }
}

/// 按追溯维度匹配调出仓库存并校验数量充足性（纯内存、无 DB，便于单测直接覆盖业务逻辑）。
///
/// 维度口径与出库真实扣减（`inv::batch::apply_ship_item_deduction`）对齐：
/// - 白坯布（`dims.color_no` 为空）：按款号 + 批次匹配，**不强制缸号**（维持既有宽松放行）；
/// - 染色布（`dims.color_no` 非空）：款号 + 色号 + 批次 + 缸号四维齐全才命中并放行。
///
/// 布种判定的唯一来源是 `dims`（由 `fabric_class::validate_fabric_trace` 归一），
/// 此处不再另写一套白坯 / 染色嗅探（尤其不得按色号名称含“白”猜白坯）。
fn match_single_item_against_stocks(
    product_id: i32,
    dims: &OutboundDimensions,
    quantity: Decimal,
    stocks: &[inventory_stock::Model],
) -> Result<(), AppError> {
    let is_dyed = !dims.color_no.is_empty();
    let matched: Vec<&inventory_stock::Model> = stocks
        .iter()
        .filter(|s| {
            s.product_id == product_id
                && s.color_no == dims.color_no
                && s.batch_no == dims.batch_no
                // 白坯不强制缸号（is_dyed=false 时短路放行任意缸行）；染色布要求缸号四维齐全命中
                && (!is_dyed
                    || s.dye_lot_no.as_deref().map(str::trim) == dims.dye_lot_no.as_deref())
        })
        .collect();

    if matched.is_empty() {
        let color_disp = if dims.color_no.is_empty() {
            "白坯（无颜色）".to_string()
        } else {
            dims.color_no.clone()
        };
        let dye_disp = match dims.dye_lot_no.as_deref() {
            Some(lot) => format!("，缸号：{}", lot),
            None => String::new(),
        };
        return Err(AppError::business(format!(
            "调出仓库无匹配库存记录，产品 {}（色号：{}{}，批次：{}）",
            product_id, color_disp, dye_disp, dims.batch_no
        )));
    }

    let available: Decimal = matched.iter().map(|s| s.quantity_available).sum();
    if available < quantity {
        return Err(AppError::business(format!(
            "调出仓库库存不足，产品 {}，当前库存：{}，需要调拨：{}",
            product_id, available, quantity
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::inv::InventoryTransferItemRequest;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn item(
        product_id: i32,
        quantity: &str,
        color: Option<&str>,
        dye: Option<&str>,
        batch: Option<&str>,
    ) -> InventoryTransferItemRequest {
        InventoryTransferItemRequest {
            product_id: Some(product_id),
            quantity: Some(d(quantity)),
            notes: None,
            color_no: color.map(str::to_string),
            dye_lot_no: dye.map(str::to_string),
            batch_no: batch.map(str::to_string),
            unit_cost: None,
        }
    }

    fn stock(
        product_id: i32,
        color_no: &str,
        dye_lot_no: Option<&str>,
        batch_no: &str,
        available: &str,
    ) -> inventory_stock::Model {
        inventory_stock::Model {
            id: 1,
            warehouse_id: 1,
            product_id,
            quantity_on_hand: d(available),
            quantity_available: d(available),
            quantity_reserved: Decimal::ZERO,
            quantity_shipped: Decimal::ZERO,
            quantity_incoming: Decimal::ZERO,
            reorder_point: Decimal::ZERO,
            max_stock_point: Decimal::ZERO,
            reorder_quantity: Decimal::ZERO,
            bin_location: None,
            last_count_date: None,
            last_movement_date: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            batch_no: batch_no.to_string(),
            color_no: color_no.to_string(),
            dye_lot_no: dye_lot_no.map(str::to_string),
            grade: "一等品".to_string(),
            production_date: None,
            expiry_date: None,
            quantity_meters: d(available),
            quantity_kg: Decimal::ZERO,
            gram_weight: None,
            width: None,
            location_id: None,
            shelf_no: None,
            layer_no: None,
            stock_status: "正常".to_string(),
            quality_status: "合格".to_string(),
            version: 0,
            replenishment_strategy: "reorder_point".to_string(),
        }
    }

    /// 归一维度 + 内存匹配：复现 check 的核心判定（require_outbound_dimensions + match_single_item_against_stocks）
    fn run_check(
        it: &InventoryTransferItemRequest,
        stocks: &[inventory_stock::Model],
    ) -> Result<(), AppError> {
        let product_id = it.product_id.unwrap();
        let dims = require_outbound_dimensions(
            "调拨出库",
            product_id,
            it.color_no.as_deref(),
            it.dye_lot_no.as_deref(),
            it.batch_no.as_deref(),
        )?;
        match_single_item_against_stocks(
            product_id,
            &dims,
            it.quantity.unwrap_or(Decimal::ZERO),
            stocks,
        )
    }

    #[test]
    fn white_fabric_remains_loose_without_dye_lot() {
        // 白坯布（色号为空）：库存行无缸号也应放行，维持既有宽松行为
        let it = item(1, "50", None, None, Some("B1"));
        let stocks = vec![stock(1, "", None, "B1", "100")];
        run_check(&it, &stocks).expect("白坯无缸号应维持宽松放行");
    }

    #[test]
    fn white_fabric_matches_by_product_and_batch() {
        // 白坯按款号 + 批次匹配：批次不命中则无库存记录被拒
        let stocks = vec![stock(1, "", None, "B1", "100")];
        let ok = item(1, "50", Some(""), None, Some("B1"));
        run_check(&ok, &stocks).expect("白坯款号+批次匹配应放行");

        let wrong_batch = item(1, "50", Some(""), None, Some("B9"));
        let err = run_check(&wrong_batch, &stocks).expect_err("白坯批次不匹配应无库存记录被拒");
        assert!(err.to_string().contains("无匹配库存"), "实际：{}", err);
    }

    #[test]
    fn dyed_fabric_missing_dye_lot_rejected() {
        // 染色布（色号非空）缺缸号：判定复用 fabric_class 唯一口径，返回明确业务错误（非裸 500）
        let it = item(1, "50", Some("RED"), None, Some("B1"));
        let stocks = vec![stock(1, "RED", Some("DL-A"), "B1", "100")];
        let err = run_check(&it, &stocks).expect_err("染色布缺缸号必须被拒");
        assert!(err.to_string().contains("缸号"), "实际：{}", err);
    }

    #[test]
    fn dyed_fabric_requires_all_four_dimensions() {
        // 染色布四维齐全才命中放行
        let ok = item(1, "50", Some("RED"), Some("DL-A"), Some("B1"));
        let stocks = vec![stock(1, "RED", Some("DL-A"), "B1", "100")];
        run_check(&ok, &stocks).expect("染色布四维齐全应放行");

        // 缸号不同则四维不齐 → 不命中，被拒（证明缸号已纳入染色布匹配）
        let wrong_dye = item(1, "50", Some("RED"), Some("DL-Z"), Some("B1"));
        let err = run_check(&wrong_dye, &stocks).expect_err("染色布缸号不匹配应被拒");
        assert!(err.to_string().contains("无匹配库存"), "实际：{}", err);
    }

    #[test]
    fn dyed_fabric_four_dim_hit_but_insufficient_quantity_rejected() {
        // 染色布四维命中但数量不足 → 库存不足业务错误
        let it = item(1, "150", Some("RED"), Some("DL-A"), Some("B1"));
        let stocks = vec![stock(1, "RED", Some("DL-A"), "B1", "100")];
        let err = run_check(&it, &stocks).expect_err("数量不足应被拒");
        assert!(err.to_string().contains("库存不足"), "实际：{}", err);
    }
}
