//! 库存服务 - 库存检查辅助（inv/stock）
//!
//! 提供调拨单创建时的源仓库库存预检逻辑。
//! 由 `move::create_transfer` 调用以确保调出仓库有充足库存。
//!
//! 拆分自原 `inventory_transfer_service.rs` 的 `check_from_warehouse_inventory` 私有方法。

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::models::inventory_stock::{self, Entity as InventoryStockEntity};
use crate::utils::error::AppError;

use super::{InventoryTransferItemRequest, InventoryTransferService};

impl InventoryTransferService {
    /// 检查调出仓库库存是否充足
    ///
    /// 在调拨单创建事务中调用，确保所有调拨明细在源仓库有足够库存。
    /// 采用批量查询优化 N+1：先一次性查出所有相关 product 的库存记录，
    /// 再用 HashMap 内存匹配。
    pub(crate) async fn check_from_warehouse_inventory(
        &self,
        from_warehouse_id: &i32,
        items: &[InventoryTransferItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        // 批量获取库存记录（优化N+1查询）
        let product_ids: Vec<i32> = items
            .iter()
            .map(|item| item.product_id.unwrap_or(0))
            .collect();
        let stocks = InventoryStockEntity::find()
            .filter(inventory_stock::Column::WarehouseId.eq(*from_warehouse_id))
            .filter(inventory_stock::Column::ProductId.is_in(product_ids))
            .all(txn)
            .await?;
        let stock_map: std::collections::HashMap<i32, inventory_stock::Model> =
            stocks.into_iter().map(|s| (s.product_id, s)).collect();

        for item in items {
            let product_id = item.product_id.unwrap_or(0);
            let quantity = item.quantity.unwrap_or(rust_decimal::Decimal::ZERO);

            // 查询调出仓库的库存
            let stock = stock_map.get(&product_id);

            match stock {
                Some(s) if s.quantity_available >= quantity => {
                    // 库存充足，继续检查下一个产品
                    continue;
                }
                Some(s) => {
                    return Err(AppError::business(format!(
                        "调出仓库库存不足，产品 {}，当前库存：{}，需要调拨：{}",
                        product_id, s.quantity_available, quantity
                    )));
                }
                None => {
                    return Err(AppError::business(format!(
                        "产品 {} 在调出仓库没有库存记录",
                        product_id
                    )));
                }
            }
        }
        Ok(())
    }
}
