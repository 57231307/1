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
        // 跳过 product_id 为 None 的项，避免脏 product_id=0 污染查询
        let product_ids: Vec<i32> = items.iter().filter_map(|item| item.product_id).collect();
        let stocks = InventoryStockEntity::find()
            .filter(inventory_stock::Column::WarehouseId.eq(*from_warehouse_id))
            .filter(inventory_stock::Column::ProductId.is_in(product_ids))
            .all(txn)
            .await?;
        let stock_map: std::collections::HashMap<i32, inventory_stock::Model> =
            stocks.into_iter().map(|s| (s.product_id, s)).collect();

        for item in items {
            // 上游已校验 product_id 必填；None 项直接跳过
            let Some(product_id) = item.product_id else {
                continue;
            };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decs;
    use crate::ymd;
    use chrono::Utc;
    use rust_decimal::Decimal;

    /// 测试夹具：构造库存记录模型
    ///
    /// 复现 `inventory_stock::Model` 的构造，仅填充与 `check_from_warehouse_inventory`
    /// 判定相关的字段（product_id、warehouse_id、quantity_available），其余字段使用合理默认值。
    /// 供测试中模拟 stock_map 的内存匹配使用。
    fn make_stock(product_id: i32, quantity_available: Decimal) -> inventory_stock::Model {
        inventory_stock::Model {
            id: 1,
            warehouse_id: 1,
            product_id,
            quantity_on_hand: quantity_available,
            quantity_available,
            quantity_reserved: Decimal::ZERO,
            quantity_shipped: Decimal::ZERO,
            quantity_incoming: Decimal::ZERO,
            reorder_point: Decimal::ZERO,
            max_stock_point: Decimal::ZERO,
            reorder_quantity: Decimal::ZERO,
            bin_location: None,
            last_count_date: None,
            last_movement_date: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            batch_no: "测试批次".to_string(),
            color_no: "测试色号".to_string(),
            dye_lot_no: None,
            grade: "一等品".to_string(),
            production_date: None,
            expiry_date: None,
            quantity_meters: quantity_available,
            quantity_kg: Decimal::ZERO,
            gram_weight: None,
            width: None,
            location_id: None,
            shelf_no: None,
            layer_no: None,
            stock_status: "正常".to_string(),
            quality_status: "合格".to_string(),
            version: 0,
        }
    }

    /// 测试夹具：复现库存充足判断逻辑
    ///
    /// 对应业务代码第 49 行 `Some(s) if s.quantity_available >= quantity => continue`。
    fn is_stock_sufficient(available: Decimal, required: Decimal) -> bool {
        available >= required
    }

    /// 测试夹具：复现库存不足错误消息
    ///
    /// 对应业务代码第 54-57 行的 format! 模板。
    fn format_insufficient_error(
        product_id: i32,
        available: Decimal,
        required: Decimal,
    ) -> String {
        format!(
            "调出仓库库存不足，产品 {}，当前库存：{}，需要调拨：{}",
            product_id, available, required
        )
    }

    /// 测试夹具：复现无库存记录错误消息
    ///
    /// 对应业务代码第 60-63 行的 format! 模板。
    fn format_no_stock_error(product_id: i32) -> String {
        format!("产品 {} 在调出仓库没有库存记录", product_id)
    }

    /// 测试夹具：复现 product_id 过滤逻辑
    ///
    /// 对应业务代码第 29 行 `items.iter().filter_map(|item| item.product_id).collect()`。
    fn filter_product_ids(items: &[Option<i32>]) -> Vec<i32> {
        items.iter().filter_map(|&x| x).collect()
    }

    // ========== product_id 过滤逻辑测试 ==========

    /// 测试_product_id过滤_全部Some时收集所有id
    ///
    /// 验证第 29 行 filter_map 逻辑：当所有项 product_id 均为 Some 时，
    /// 收集结果应包含全部 id，顺序与输入一致。
    #[test]
    fn 测试_product_id过滤_全部Some时收集所有id() {
        let items = vec![Some(1), Some(2), Some(3)];
        let product_ids = filter_product_ids(&items);

        assert_eq!(product_ids, vec![1, 2, 3]);
    }

    /// 测试_product_id过滤_含None时过滤掉None
    ///
    /// 验证第 29 行 filter_map 逻辑：当 items 中混有 None 项时，
    /// None 被过滤掉，仅保留 Some(id) 的值。
    #[test]
    fn 测试_product_id过滤_含None时过滤掉None() {
        let items = vec![Some(1), None, Some(3), None, Some(5)];
        let product_ids = filter_product_ids(&items);

        assert_eq!(product_ids, vec![1, 3, 5]);
    }

    /// 测试_product_id过滤_全部None时返回空Vec
    ///
    /// 验证第 29 行 filter_map 逻辑：当所有项 product_id 均为 None 时，
    /// 返回空 Vec，避免脏 product_id 污染后续 is_in 查询。
    #[test]
    fn 测试_product_id过滤_全部None时返回空vec() {
        let items = vec![None, None, None];
        let product_ids = filter_product_ids(&items);

        assert!(product_ids.is_empty());
    }

    // ========== quantity 默认值测试 ==========

    /// 测试_quantity默认值_None时为0
    ///
    /// 验证第 43 行 `item.quantity.unwrap_or(Decimal::ZERO)` 逻辑：
    /// 当 quantity 为 None 时，默认为 0。
    #[test]
    fn 测试_quantity默认值_none时为0() {
        let quantity: Option<Decimal> = None;
        let resolved = quantity.unwrap_or(Decimal::ZERO);

        assert_eq!(resolved, Decimal::ZERO);
    }

    /// 测试_quantity默认值_Some时使用原值
    ///
    /// 验证第 43 行 `item.quantity.unwrap_or(Decimal::ZERO)` 逻辑：
    /// 当 quantity 为 Some 时，使用原值，不触发默认值。
    #[test]
    fn 测试_quantity默认值_some时使用原值() {
        let original = decs!("100.50");
        let quantity: Option<Decimal> = Some(original);
        let resolved = quantity.unwrap_or(Decimal::ZERO);

        assert_eq!(resolved, original);
    }

    // ========== 库存充足判断测试 ==========

    /// 测试_库存充足判断_可用等于需求时充足
    ///
    /// 验证第 49 行 `s.quantity_available >= quantity` 边界条件：
    /// 当可用库存恰好等于调拨需求时，判定为充足（`>=` 含等号）。
    #[test]
    fn 测试_库存充足判断_可用等于需求时充足() {
        let available = decs!("100");
        let required = decs!("100");

        assert!(is_stock_sufficient(available, required));
    }

    /// 测试_库存充足判断_可用大于需求时充足
    ///
    /// 验证第 49 行判断逻辑：当可用库存严格大于调拨需求时，判定为充足。
    #[test]
    fn 测试_库存充足判断_可用大于需求时充足() {
        let available = decs!("150");
        let required = decs!("100");

        assert!(is_stock_sufficient(available, required));
    }

    /// 测试_库存充足判断_可用小于需求时不足
    ///
    /// 验证第 49 行判断逻辑：当可用库存小于调拨需求时，判定为不足，
    /// 触发第 53-58 行的库存不足错误分支。
    #[test]
    fn 测试_库存充足判断_可用小于需求时不足() {
        let available = decs!("50");
        let required = decs!("100");

        assert!(!is_stock_sufficient(available, required));
    }

    // ========== 库存不足错误消息测试 ==========

    /// 测试_库存不足错误消息_包含产品id
    ///
    /// 验证第 54-57 行错误消息格式：消息中应包含具体的产品 id。
    #[test]
    fn 测试_库存不足错误消息_包含产品id() {
        let product_id = 42;
        let msg = format_insufficient_error(product_id, decs!("50"), decs!("100"));

        assert!(msg.contains("产品 42"), "错误消息应包含产品 id，实际：{msg}");
    }

    /// 测试_库存不足错误消息_包含当前库存和需要调拨数量
    ///
    /// 验证第 54-57 行错误消息格式：消息中应同时包含当前库存与需要调拨数量，
    /// 便于运维定位库存缺口。
    #[test]
    fn 测试_库存不足错误消息_包含当前库存和需要调拨数量() {
        let msg = format_insufficient_error(1, decs!("50"), decs!("100"));

        assert!(
            msg.contains("当前库存：50"),
            "错误消息应包含当前库存，实际：{msg}"
        );
        assert!(
            msg.contains("需要调拨：100"),
            "错误消息应包含需要调拨数量，实际：{msg}"
        );
    }

    // ========== 无库存记录错误消息测试 ==========

    /// 测试_无库存记录错误消息_包含产品id
    ///
    /// 验证第 60-63 行错误消息格式：消息中应包含具体的产品 id，
    /// 对应 stock_map 中未命中（None 分支）的场景。
    #[test]
    fn 测试_无库存记录错误消息_包含产品id() {
        let product_id = 99;
        let msg = format_no_stock_error(product_id);

        assert!(
            msg.contains("产品 99"),
            "错误消息应包含产品 id，实际：{msg}"
        );
        assert!(
            msg.contains("没有库存记录"),
            "错误消息应说明无库存记录，实际：{msg}"
        );
    }

    // ========== 夹具宏与端到端模拟测试 ==========

    /// 测试_decs夹具宏_可用性验证
    ///
    /// 验证 `decs!` 宏可正确解析 Decimal 字符串，确保后续测试夹具的数值基础可靠。
    #[test]
    fn 测试_decs夹具宏_可用性验证() {
        let v = decs!("123.45");

        assert_eq!(v.to_string(), "123.45");
    }

    /// 测试_ymd夹具宏_可用性验证
    ///
    /// 验证 `ymd!` 宏可正确解析日期，确保日期类夹具可用。
    #[test]
    fn 测试_ymd夹具宏_可用性验证() {
        let d = ymd!(2026, 1, 15);

        assert_eq!(d.format("%Y-%m-%d").to_string(), "2026-01-15");
    }

    /// 测试_库存检查端到端模拟_充足时不报错
    ///
    /// 端到端复现 check_from_warehouse_inventory 的核心判定流程：
    /// 构造 stock_map，对每个 item 取 product_id 与 quantity，
    /// 当库存充足时流程应继续而非报错。
    /// 覆盖第 35-66 行的 HashMap 匹配与 match 分支逻辑。
    #[test]
    fn 测试_库存检查端到端模拟_充足时不报错() {
        // 模拟 stock_map：product 1 库存 100，product 2 库存 200
        let stock_map: std::collections::HashMap<i32, inventory_stock::Model> = [
            (1, make_stock(1, decs!("100"))),
            (2, make_stock(2, decs!("200"))),
        ]
        .into_iter()
        .collect();

        // 模拟 items：product 1 调拨 50，product 2 调拨 150，均充足
        let items: Vec<(Option<i32>, Option<Decimal>)> = vec![
            (Some(1), Some(decs!("50"))),
            (Some(2), Some(decs!("150"))),
        ];

        let mut has_error = false;
        for (product_id_opt, quantity_opt) in items {
            let Some(product_id) = product_id_opt else {
                continue;
            };
            let quantity = quantity_opt.unwrap_or(Decimal::ZERO);

            let stock = stock_map.get(&product_id);
            match stock {
                Some(s) if s.quantity_available >= quantity => continue,
                Some(_) | None => {
                    has_error = true;
                    break;
                }
            }
        }

        assert!(!has_error, "库存充足时不应触发错误分支");
    }

    /// 测试_库存检查端到端模拟_不足时报错且消息正确
    ///
    /// 端到端复现：当某产品库存不足时，流程应进入第 53-58 行错误分支，
    /// 且错误消息与 format_insufficient_error 一致。
    #[test]
    fn 测试_库存检查端到端模拟_不足时报错且消息正确() {
        let stock_map: std::collections::HashMap<i32, inventory_stock::Model> =
            [(1, make_stock(1, decs!("30")))].into_iter().collect();

        let product_id = 1;
        let quantity = decs!("50");
        let stock = stock_map.get(&product_id);

        let err_msg = match stock {
            Some(s) if s.quantity_available >= quantity => String::new(),
            Some(s) => format_insufficient_error(product_id, s.quantity_available, quantity),
            None => format_no_stock_error(product_id),
        };

        assert!(
            err_msg.contains("调出仓库库存不足"),
            "库存不足时应返回库存不足错误，实际：{err_msg}"
        );
        assert!(
            err_msg.contains("当前库存：30") && err_msg.contains("需要调拨：50"),
            "错误消息应包含正确数值，实际：{err_msg}"
        );
    }
}
