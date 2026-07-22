//! 库存事务版本方法（带 _txn 后缀，与外层同名方法行为一致但接受外部事务）
//!
//! 拆分自 inventory_stock_service.rs：原 4 个 _txn 方法独立成文件。

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set,
};
use chrono::Utc;

use crate::models::inventory_stock;
use crate::models::inventory_transaction;
use crate::services::event_bus::BusinessEvent;
use crate::utils::error::AppError;

use super::inventory_stock_query::RecordTransactionArgs;
use super::inventory_stock_service::{CreateStockFabricArgs, InventoryStockService};

impl InventoryStockService {
    pub async fn update_stock_quantity_with_optimistic_lock_txn(
        txn: &sea_orm::DatabaseTransaction,
        id: i32,
        quantity_meters: Decimal,
        quantity_kg: Decimal,
        expected_version: i32,
    ) -> Result<inventory_stock::Model, AppError> {
        let update_result = inventory_stock::Entity::update_many()
            .col_expr(
                inventory_stock::Column::QuantityOnHand,
                sea_orm::sea_query::Expr::val(quantity_meters).into(),
            )
            .col_expr(
                inventory_stock::Column::QuantityAvailable,
                sea_orm::sea_query::Expr::val(quantity_meters).into(),
            )
            .col_expr(
                inventory_stock::Column::QuantityMeters,
                sea_orm::sea_query::Expr::val(quantity_meters).into(),
            )
            .col_expr(
                inventory_stock::Column::QuantityKg,
                sea_orm::sea_query::Expr::val(quantity_kg).into(),
            )
            .col_expr(
                inventory_stock::Column::Version,
                sea_orm::sea_query::Expr::col(inventory_stock::Column::Version).add(1),
            )
            .col_expr(
                inventory_stock::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
            )
            .filter(inventory_stock::Column::Id.eq(id))
            .filter(inventory_stock::Column::Version.eq(expected_version))
            .exec(txn)
            .await?;

        if update_result.rows_affected == 0 {
            return Err(AppError::business(format!(
                "并发冲突：库存记录 ID {} 已被其他用户修改，期望版本 {}",
                id, expected_version
            )));
        }

        inventory_stock::Entity::find_by_id(id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存记录 ID {} 不存在", id)))
    }

    /// 创建面料库存记录（事务版本）
    ///
    /// 批次 338 v10 复审 P3 修复：签名从 14 参数改为 2 参数（txn + args），
    /// 复用 `CreateStockFabricArgs` 参数对象，消除 `clippy::too_many_arguments` 警告。
    pub async fn create_stock_fabric_txn(
        txn: &sea_orm::DatabaseTransaction,
        args: CreateStockFabricArgs,
    ) -> Result<inventory_stock::Model, AppError> {
        let CreateStockFabricArgs {
            warehouse_id,
            product_id,
            batch_no,
            color_no,
            dye_lot_no,
            grade,
            quantity_meters,
            quantity_kg,
            gram_weight,
            width,
            location_id,
            shelf_no,
            layer_no,
        } = args;
        let _final_quantity_kg =
            Self::calculate_quantity_kg(quantity_meters, gram_weight, width, quantity_kg);

        let active_stock = inventory_stock::ActiveModel {
            id: Default::default(),
            warehouse_id: Set(warehouse_id),
            product_id: Set(product_id),
            quantity_on_hand: Set(quantity_meters),
            quantity_available: Set(quantity_meters),
            quantity_reserved: Set(Decimal::ZERO),
            quantity_incoming: Set(Decimal::ZERO),
            reorder_point: Set(Decimal::ZERO),
            max_stock_point: Set(Decimal::ZERO),
            reorder_quantity: Set(Decimal::ZERO),
            last_count_date: Set(None),
            last_movement_date: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            batch_no: Set(batch_no),
            color_no: Set(color_no),
            dye_lot_no: Set(dye_lot_no),
            grade: Set(grade),
            production_date: Set(None),
            expiry_date: Set(None),
            quantity_meters: Set(quantity_meters),
            quantity_kg: Set(quantity_kg),
            gram_weight: Set(gram_weight),
            width: Set(width),
            quantity_shipped: Set(Decimal::ZERO),
            location_id: Set(location_id),
            shelf_no: Set(shelf_no),
            layer_no: Set(layer_no),
            bin_location: Set(None),
            stock_status: Set("正常".to_string()),
            quality_status: Set("合格".to_string()),
            version: Set(0),
        };

        active_stock.insert(txn).await.map_err(AppError::from)
    }

    /// 记录库存流水（事务版本）
    ///
    /// 批次 338 v10 复审 P3 修复：签名从 19 参数改为 2 参数（txn + args），
    /// 复用 `RecordTransactionArgs` 参数对象，消除 `clippy::too_many_arguments` 警告。
    pub async fn record_transaction_txn(
        txn: &sea_orm::DatabaseTransaction,
        args: RecordTransactionArgs,
    ) -> Result<(inventory_transaction::Model, Option<BusinessEvent>), AppError> {
        let active_transaction = Self::build_transaction_active_model(args);
        let transaction = active_transaction.insert(txn).await?;
        let event = Self::build_transaction_event(&transaction);
        // P0 5-2 修复：事件由调用方在 commit 成功后统一 publish，避免幻事件
        Ok((transaction, Some(event)))
    }

    /// 构建库存流水 ActiveModel
    fn build_transaction_active_model(
        args: RecordTransactionArgs,
    ) -> inventory_transaction::ActiveModel {
        let RecordTransactionArgs {
            transaction_type,
            product_id,
            warehouse_id,
            batch_no,
            color_no,
            dye_lot_no,
            grade,
            quantity_meters,
            quantity_kg,
            source_bill_type,
            source_bill_no,
            source_bill_id,
            quantity_before_meters,
            quantity_before_kg,
            quantity_after_meters,
            quantity_after_kg,
            notes,
            created_by,
        } = args;
        inventory_transaction::ActiveModel {
            id: Default::default(),
            transaction_type: Set(transaction_type),
            product_id: Set(product_id),
            warehouse_id: Set(warehouse_id),
            batch_no: Set(batch_no),
            color_no: Set(color_no),
            dye_lot_no: Set(dye_lot_no),
            grade: Set(grade),
            quantity_meters: Set(quantity_meters),
            quantity_kg: Set(quantity_kg),
            source_bill_type: Set(source_bill_type),
            source_bill_no: Set(source_bill_no),
            source_bill_id: Set(source_bill_id),
            quantity_before_meters: Set(quantity_before_meters),
            quantity_before_kg: Set(quantity_before_kg),
            quantity_after_meters: Set(quantity_after_meters),
            quantity_after_kg: Set(quantity_after_kg),
            notes: Set(notes),
            created_by: Set(created_by),
            created_at: Set(Utc::now()),
        }
    }

    /// 从已插入的流水 Model 构建业务事件
    fn build_transaction_event(
        transaction: &inventory_transaction::Model,
    ) -> BusinessEvent {
        BusinessEvent::InventoryTransactionCreated {
            transaction_id: transaction.id,
            transaction_type: transaction.transaction_type.clone(),
            product_id: transaction.product_id,
            warehouse_id: transaction.warehouse_id,
            quantity_meters: transaction.quantity_meters,
            quantity_kg: transaction.quantity_kg,
            source_bill_type: transaction.source_bill_type.clone(),
            source_bill_no: transaction.source_bill_no.clone(),
            source_bill_id: transaction.source_bill_id,
            batch_no: transaction.batch_no.clone(),
            color_no: transaction.color_no.clone(),
            created_by: transaction.created_by,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // 引入 decs! 测试夹具宏，用于从字符串解析 Decimal
    use crate::decs;
    // decs! 宏展开后调用 Decimal::from_str，需要 FromStr trait 在作用域内
    use std::str::FromStr;

    // ========== RecordTransactionArgs 构造测试 ==========

    /// 验证 RecordTransactionArgs 入库场景下所有字段被正确设置
    #[test]
    fn 测试_record_transaction_args_入库场景_字段完整() {
        let args = RecordTransactionArgs {
            transaction_type: "IN".to_string(),
            product_id: 1001,
            warehouse_id: 2002,
            batch_no: "BATCH-2024-001".to_string(),
            color_no: "COLOR-RED".to_string(),
            dye_lot_no: Some("LOT-A1".to_string()),
            grade: "A".to_string(),
            quantity_meters: decs!("500.5"),
            quantity_kg: decs!("125.25"),
            source_bill_type: Some("PURCHASE".to_string()),
            source_bill_no: Some("PO-2024-0001".to_string()),
            source_bill_id: Some(9001),
            quantity_before_meters: Some(decs!("0")),
            quantity_before_kg: Some(decs!("0")),
            quantity_after_meters: Some(decs!("500.5")),
            quantity_after_kg: Some(decs!("125.25")),
            notes: Some("采购入库".to_string()),
            created_by: Some(1),
        };

        // 逐字段断言，验证入库场景的完整构造
        assert_eq!(args.transaction_type, "IN");
        assert_eq!(args.product_id, 1001);
        assert_eq!(args.warehouse_id, 2002);
        assert_eq!(args.batch_no, "BATCH-2024-001");
        assert_eq!(args.color_no, "COLOR-RED");
        assert_eq!(args.dye_lot_no.as_deref(), Some("LOT-A1"));
        assert_eq!(args.grade, "A");
        assert_eq!(args.quantity_meters, decs!("500.5"));
        assert_eq!(args.quantity_kg, decs!("125.25"));
        assert_eq!(args.source_bill_type.as_deref(), Some("PURCHASE"));
        assert_eq!(args.source_bill_no.as_deref(), Some("PO-2024-0001"));
        assert_eq!(args.source_bill_id, Some(9001));
        // 入库前为 0，入库后等于本次入库量
        assert_eq!(args.quantity_before_meters, Some(decs!("0")));
        assert_eq!(args.quantity_before_kg, Some(decs!("0")));
        assert_eq!(args.quantity_after_meters, Some(decs!("500.5")));
        assert_eq!(args.quantity_after_kg, Some(decs!("125.25")));
        assert_eq!(args.notes.as_deref(), Some("采购入库"));
        assert_eq!(args.created_by, Some(1));
    }

    /// 验证 RecordTransactionArgs 出库场景下所有字段被正确设置
    #[test]
    fn 测试_record_transaction_args_出库场景_字段完整() {
        let args = RecordTransactionArgs {
            transaction_type: "OUT".to_string(),
            product_id: 3003,
            warehouse_id: 4004,
            batch_no: "BATCH-2024-002".to_string(),
            color_no: "COLOR-BLUE".to_string(),
            dye_lot_no: None,
            grade: "B".to_string(),
            quantity_meters: decs!("100.0"),
            quantity_kg: decs!("25.0"),
            source_bill_type: Some("SALES".to_string()),
            source_bill_no: Some("SO-2024-0002".to_string()),
            source_bill_id: Some(8002),
            quantity_before_meters: Some(decs!("500.0")),
            quantity_before_kg: Some(decs!("125.0")),
            quantity_after_meters: Some(decs!("400.0")),
            quantity_after_kg: Some(decs!("100.0")),
            notes: None,
            created_by: Some(2),
        };

        // 逐字段断言，验证出库场景的完整构造
        assert_eq!(args.transaction_type, "OUT");
        assert_eq!(args.product_id, 3003);
        assert_eq!(args.warehouse_id, 4004);
        assert_eq!(args.batch_no, "BATCH-2024-002");
        assert_eq!(args.color_no, "COLOR-BLUE");
        // 出库场景下 dye_lot_no 可为 None
        assert_eq!(args.dye_lot_no, None);
        assert_eq!(args.grade, "B");
        assert_eq!(args.quantity_meters, decs!("100.0"));
        assert_eq!(args.quantity_kg, decs!("25.0"));
        assert_eq!(args.source_bill_type.as_deref(), Some("SALES"));
        assert_eq!(args.source_bill_no.as_deref(), Some("SO-2024-0002"));
        assert_eq!(args.source_bill_id, Some(8002));
        // 出库前为 500，出库 100 后剩余 400
        assert_eq!(args.quantity_before_meters, Some(decs!("500.0")));
        assert_eq!(args.quantity_before_kg, Some(decs!("125.0")));
        assert_eq!(args.quantity_after_meters, Some(decs!("400.0")));
        assert_eq!(args.quantity_after_kg, Some(decs!("100.0")));
        // 出库场景下 notes 可为 None
        assert_eq!(args.notes, None);
        assert_eq!(args.created_by, Some(2));
    }

    // ========== CreateStockFabricArgs 构造测试 ==========

    /// 验证 CreateStockFabricArgs 含缸号场景下所有字段被正确设置
    #[test]
    fn 测试_create_stock_fabric_args_含缸号_字段完整() {
        let args = CreateStockFabricArgs {
            warehouse_id: 1001,
            product_id: 2002,
            batch_no: "BATCH-2024-A001".to_string(),
            color_no: "RED-001".to_string(),
            dye_lot_no: Some("LOT-D01".to_string()),
            grade: "A".to_string(),
            quantity_meters: decs!("1000.0"),
            quantity_kg: decs!("250.0"),
            gram_weight: Some(decs!("250.0")),
            width: Some(decs!("150.0")),
            location_id: Some(5001),
            shelf_no: Some("A-01".to_string()),
            layer_no: Some("L1".to_string()),
        };

        // 逐字段断言，验证含缸号场景的完整构造
        assert_eq!(args.warehouse_id, 1001);
        assert_eq!(args.product_id, 2002);
        assert_eq!(args.batch_no, "BATCH-2024-A001");
        assert_eq!(args.color_no, "RED-001");
        assert_eq!(args.dye_lot_no.as_deref(), Some("LOT-D01"));
        assert_eq!(args.grade, "A");
        assert_eq!(args.quantity_meters, decs!("1000.0"));
        assert_eq!(args.quantity_kg, decs!("250.0"));
        assert_eq!(args.gram_weight, Some(decs!("250.0")));
        assert_eq!(args.width, Some(decs!("150.0")));
        assert_eq!(args.location_id, Some(5001));
        assert_eq!(args.shelf_no.as_deref(), Some("A-01"));
        assert_eq!(args.layer_no.as_deref(), Some("L1"));
    }

    /// 验证 CreateStockFabricArgs 不含缸号场景下所有可选字段为 None
    #[test]
    fn 测试_create_stock_fabric_args_不含缸号_可选字段为_none() {
        let args = CreateStockFabricArgs {
            warehouse_id: 3003,
            product_id: 4004,
            batch_no: "BATCH-2024-B002".to_string(),
            color_no: "BLUE-002".to_string(),
            dye_lot_no: None,
            grade: "B".to_string(),
            quantity_meters: decs!("500.0"),
            quantity_kg: decs!("125.0"),
            gram_weight: None,
            width: None,
            location_id: None,
            shelf_no: None,
            layer_no: None,
        };

        // 逐字段断言，验证不含缸号场景的可选字段均为 None
        assert_eq!(args.warehouse_id, 3003);
        assert_eq!(args.product_id, 4004);
        assert_eq!(args.batch_no, "BATCH-2024-B002");
        assert_eq!(args.color_no, "BLUE-002");
        // 不含缸号场景下 dye_lot_no 为 None
        assert_eq!(args.dye_lot_no, None);
        assert_eq!(args.grade, "B");
        assert_eq!(args.quantity_meters, decs!("500.0"));
        assert_eq!(args.quantity_kg, decs!("125.0"));
        // 其他可选字段也应为 None
        assert_eq!(args.gram_weight, None);
        assert_eq!(args.width, None);
        assert_eq!(args.location_id, None);
        assert_eq!(args.shelf_no, None);
        assert_eq!(args.layer_no, None);
    }

    // ========== BusinessEvent::InventoryTransactionCreated 变体存在性验证 ==========

    /// 验证 BusinessEvent::InventoryTransactionCreated 变体可被 match 匹配
    ///
    /// 此测试通过穷举 match 确认枚举变体存在，避免重构时该变体被误删导致编译错误延后暴露。
    #[test]
    fn 测试_business_event_inventory_transaction_created_变体存在() {
        let event = BusinessEvent::InventoryTransactionCreated {
            transaction_id: 1,
            transaction_type: "IN".to_string(),
            product_id: 100,
            warehouse_id: 200,
            quantity_meters: decs!("100.0"),
            quantity_kg: decs!("25.0"),
            source_bill_type: Some("PURCHASE".to_string()),
            source_bill_no: Some("PO-001".to_string()),
            source_bill_id: Some(1),
            batch_no: "BATCH-001".to_string(),
            color_no: "RED-001".to_string(),
            created_by: Some(1),
        };

        // 通过 match 确认 InventoryTransactionCreated 变体可被匹配
        let matched = match event {
            BusinessEvent::InventoryTransactionCreated {
                transaction_id,
                transaction_type,
                product_id,
                warehouse_id,
                quantity_meters,
                quantity_kg,
                source_bill_type,
                source_bill_no,
                source_bill_id,
                batch_no,
                color_no,
                created_by,
            } => {
                // 逐一验证字段被正确读取
                assert_eq!(transaction_id, 1);
                assert_eq!(transaction_type, "IN");
                assert_eq!(product_id, 100);
                assert_eq!(warehouse_id, 200);
                assert_eq!(quantity_meters, decs!("100.0"));
                assert_eq!(quantity_kg, decs!("25.0"));
                assert_eq!(source_bill_type.as_deref(), Some("PURCHASE"));
                assert_eq!(source_bill_no.as_deref(), Some("PO-001"));
                assert_eq!(source_bill_id, Some(1));
                assert_eq!(batch_no, "BATCH-001");
                assert_eq!(color_no, "RED-001");
                assert_eq!(created_by, Some(1));
                true
            }
            _ => false,
        };

        // 确认进入了正确变体的分支
        assert!(matched, "BusinessEvent 应匹配到 InventoryTransactionCreated 变体");
    }
}
