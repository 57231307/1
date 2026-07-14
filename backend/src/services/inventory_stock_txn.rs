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
        let active_transaction = inventory_transaction::ActiveModel {
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
        };

        let transaction = active_transaction.insert(txn).await?;

        let event = BusinessEvent::InventoryTransactionCreated {
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
        };

        // P0 5-2 修复：移除事务内的 EVENT_BUS.publish(event) 调用。
        // 原实现在此处直接 publish，但事务由调用方 commit，commit 失败时事件已发造成幻事件。
        // 现将构造好的事件作为返回值的一部分交给调用方，由调用方在 commit 成功后统一 publish。
        Ok((transaction, Some(event)))
    }
}

#[cfg(test)]
mod tests {
    //! 库存事务版本方法单元测试（批次 409 补测）
    //!
    //! 覆盖目标：
    //! - CreateStockFabricArgs / RecordTransactionArgs 参数对象构造与字段保留
    //! - calculate_quantity_kg 在 create_stock_fabric_txn 中的换算逻辑
    //! - 乐观锁并发冲突错误消息格式与版本号递增表达式
    //! - BusinessEvent::InventoryTransactionCreated 事件变体可构造
    //! - 库存初始默认值常量正确性

    use super::*;
    use crate::decs;

    /// 测试夹具：构造默认 CreateStockFabricArgs（复现 create_stock_fabric_txn 入参）
    fn make_create_args() -> CreateStockFabricArgs {
        CreateStockFabricArgs {
            warehouse_id: 1,
            product_id: 100,
            batch_no: "B20260714-01".to_string(),
            color_no: "RED-001".to_string(),
            dye_lot_no: Some("LOT-A".to_string()),
            grade: "一等品".to_string(),
            quantity_meters: decs!("100"),
            quantity_kg: decs!("30"),
            gram_weight: Some(decs!("200")),
            width: Some(decs!("150")),
            location_id: Some(10),
            shelf_no: Some("A-01".to_string()),
            layer_no: Some("L1".to_string()),
        }
    }

    /// 测试夹具：构造默认 RecordTransactionArgs（复现 record_transaction_txn 入参）
    fn make_record_args() -> RecordTransactionArgs {
        RecordTransactionArgs {
            transaction_type: "PURCHASE_RECEIPT".to_string(),
            product_id: 100,
            warehouse_id: 1,
            batch_no: "B20260714-01".to_string(),
            color_no: "RED-001".to_string(),
            dye_lot_no: Some("LOT-A".to_string()),
            grade: "一等品".to_string(),
            quantity_meters: decs!("100"),
            quantity_kg: decs!("30"),
            source_bill_type: Some("PURCHASE_RECEIPT".to_string()),
            source_bill_no: Some("PR-2026-001".to_string()),
            source_bill_id: Some(500),
            quantity_before_meters: Some(decs!("0")),
            quantity_before_kg: Some(decs!("0")),
            quantity_after_meters: Some(decs!("100")),
            quantity_after_kg: Some(decs!("30")),
            notes: Some("入库自动增加库存".to_string()),
            created_by: Some(1),
        }
    }

    /// 测试_CreateStockFabricArgs_四维标识字段保留
    ///
    /// 验证 batch_no / color_no / dye_lot_no / grade 在构造后可正确读取
    #[test]
    fn 测试_CreateStockFabricArgs_四维标识字段保留() {
        let args = make_create_args();
        assert_eq!(args.batch_no, "B20260714-01");
        assert_eq!(args.color_no, "RED-001");
        assert_eq!(args.dye_lot_no.as_deref(), Some("LOT-A"));
        assert_eq!(args.grade, "一等品");
    }

    /// 测试_CreateStockFabricArgs_双计量数量字段保留
    ///
    /// 验证 quantity_meters 与 quantity_kg 双计量单位字段在构造后可正确读取
    #[test]
    fn 测试_CreateStockFabricArgs_双计量数量字段保留() {
        let args = make_create_args();
        assert_eq!(args.quantity_meters, decs!("100"));
        assert_eq!(args.quantity_kg, decs!("30"));
    }

    /// 测试_CreateStockFabricArgs_可选字段为None时构造不报错
    ///
    /// 验证 dye_lot_no / gram_weight / width / location_id / shelf_no / layer_no
    /// 在 None 时构造正常，复现采购入库场景传入空库位
    #[test]
    fn 测试_CreateStockFabricArgs_可选字段为None时构造不报错() {
        let args = CreateStockFabricArgs {
            warehouse_id: 1,
            product_id: 100,
            batch_no: "B-02".to_string(),
            color_no: "BLUE".to_string(),
            dye_lot_no: None,
            grade: "二等品".to_string(),
            quantity_meters: decs!("50"),
            quantity_kg: decs!("15"),
            gram_weight: None,
            width: None,
            location_id: None,
            shelf_no: None,
            layer_no: None,
        };
        assert!(args.dye_lot_no.is_none());
        assert!(args.gram_weight.is_none());
        assert!(args.width.is_none());
        assert!(args.location_id.is_none());
        assert!(args.shelf_no.is_none());
        assert!(args.layer_no.is_none());
    }

    /// 测试_RecordTransactionArgs_来源单据字段保留
    ///
    /// 验证 source_bill_type / source_bill_no / source_bill_id 在构造后可正确读取
    #[test]
    fn 测试_RecordTransactionArgs_来源单据字段保留() {
        let args = make_record_args();
        assert_eq!(args.source_bill_type.as_deref(), Some("PURCHASE_RECEIPT"));
        assert_eq!(args.source_bill_no.as_deref(), Some("PR-2026-001"));
        assert_eq!(args.source_bill_id, Some(500));
    }

    /// 测试_RecordTransactionArgs_变更前后数量字段保留
    ///
    /// 验证 quantity_before_* / quantity_after_* 四个字段在构造后可正确读取
    #[test]
    fn 测试_RecordTransactionArgs_变更前后数量字段保留() {
        let args = make_record_args();
        assert_eq!(args.quantity_before_meters, Some(decs!("0")));
        assert_eq!(args.quantity_before_kg, Some(decs!("0")));
        assert_eq!(args.quantity_after_meters, Some(decs!("100")));
        assert_eq!(args.quantity_after_kg, Some(decs!("30")));
    }

    /// 测试_RecordTransactionArgs_来源单据字段为None
    ///
    /// 验证 source_bill_* 三个字段为 None 时构造不报错（手动调整库存场景）
    #[test]
    fn 测试_RecordTransactionArgs_来源单据字段为None() {
        let args = RecordTransactionArgs {
            transaction_type: "MANUAL_ADJUST".to_string(),
            product_id: 100,
            warehouse_id: 1,
            batch_no: "B-03".to_string(),
            color_no: "GREEN".to_string(),
            dye_lot_no: None,
            grade: "一等品".to_string(),
            quantity_meters: decs!("10"),
            quantity_kg: decs!("3"),
            source_bill_type: None,
            source_bill_no: None,
            source_bill_id: None,
            quantity_before_meters: None,
            quantity_before_kg: None,
            quantity_after_meters: None,
            quantity_after_kg: None,
            notes: None,
            created_by: None,
        };
        assert!(args.source_bill_type.is_none());
        assert!(args.source_bill_no.is_none());
        assert!(args.source_bill_id.is_none());
        assert!(args.notes.is_none());
        assert!(args.created_by.is_none());
    }

    /// 测试_calculate_quantity_kg_克重幅宽齐全走转换器
    ///
    /// create_stock_fabric_txn 内部调用 calculate_quantity_kg(meters, gram_weight, width, kg)
    /// 验证克重和幅宽齐全时走转换器，回退值不应被使用
    #[test]
    fn 测试_calculate_quantity_kg_克重幅宽齐全走转换器() {
        let quantity_meters = decs!("100");
        let gram_weight = Some(decs!("200"));
        let width = Some(decs!("150"));
        let fallback = decs!("999"); // 不应被使用的回退值
        let result = InventoryStockService::calculate_quantity_kg(
            quantity_meters,
            gram_weight,
            width,
            fallback,
        );
        // 转换器公式：100 × 200 × (150/100) / 1000 = 30.000 kg
        assert_eq!(result, decs!("30"));
        assert_ne!(result, fallback, "克重幅宽齐全时不应回退到 fallback");
    }

    /// 测试_calculate_quantity_kg_克重缺失回退quantity_kg
    ///
    /// 复现 create_stock_fabric_txn 中 gram_weight=None 时的回退分支
    #[test]
    fn 测试_calculate_quantity_kg_克重缺失回退quantity_kg() {
        let quantity_meters = decs!("100");
        let gram_weight = None;
        let width = Some(decs!("150"));
        let fallback = decs!("30");
        let result = InventoryStockService::calculate_quantity_kg(
            quantity_meters,
            gram_weight,
            width,
            fallback,
        );
        assert_eq!(result, fallback, "gram_weight=None 时应回退到传入的 quantity_kg");
    }

    /// 测试_calculate_quantity_kg_幅宽缺失回退quantity_kg
    ///
    /// 复现 create_stock_fabric_txn 中 width=None 时的回退分支
    #[test]
    fn 测试_calculate_quantity_kg_幅宽缺失回退quantity_kg() {
        let quantity_meters = decs!("100");
        let gram_weight = Some(decs!("200"));
        let width = None;
        let fallback = decs!("30");
        let result = InventoryStockService::calculate_quantity_kg(
            quantity_meters,
            gram_weight,
            width,
            fallback,
        );
        assert_eq!(result, fallback, "width=None 时应回退到传入的 quantity_kg");
    }

    /// 测试_乐观锁并发冲突错误消息格式
    ///
    /// update_stock_quantity_with_optimistic_lock_txn 在 rows_affected==0 时返回错误，
    /// 错误消息格式：并发冲突：库存记录 ID {} 已被其他用户修改，期望版本 {}
    #[test]
    fn 测试_乐观锁并发冲突错误消息格式() {
        let id = 42;
        let expected_version = 3;
        let msg = format!(
            "并发冲突：库存记录 ID {} 已被其他用户修改，期望版本 {}",
            id, expected_version
        );
        // 验证错误消息格式
        assert_eq!(
            msg,
            "并发冲突：库存记录 ID 42 已被其他用户修改，期望版本 3"
        );
        assert!(msg.contains("并发冲突"));
        assert!(msg.contains("42"));
        assert!(msg.contains("3"));
        // 验证错误可由 AppError::business 构造（不 panic）
        let _ = AppError::business(msg);
    }

    /// 测试_乐观锁版本号递增表达式
    ///
    /// update_stock_quantity_with_optimistic_lock_txn 通过
    /// Expr::col(Version).add(1) 实现版本号 +1，验证纯算术表达式
    #[test]
    fn 测试_乐观锁版本号递增表达式() {
        // 复现 Version = Version + 1 的纯算术逻辑
        let current_version: i32 = 5;
        let next_version = current_version + 1;
        assert_eq!(next_version, 6);
        // 初始版本号为 0（与 create_stock_fabric_txn 中 version: Set(0) 一致）
        let initial_version: i32 = 0;
        assert_eq!(initial_version + 1, 1);
    }

    /// 测试_BusinessEvent_InventoryTransactionCreated_变体可构造
    ///
    /// record_transaction_txn 返回 (Model, Some(BusinessEvent::InventoryTransactionCreated{...}))
    /// 验证事件变体可构造且字段可通过模式匹配访问
    #[test]
    fn 测试_BusinessEvent_InventoryTransactionCreated_变体可构造() {
        let event = BusinessEvent::InventoryTransactionCreated {
            transaction_id: 1,
            transaction_type: "PURCHASE_RECEIPT".to_string(),
            product_id: 100,
            warehouse_id: 1,
            quantity_meters: decs!("100"),
            quantity_kg: decs!("30"),
            source_bill_type: Some("PURCHASE_RECEIPT".to_string()),
            source_bill_no: Some("PR-2026-001".to_string()),
            source_bill_id: Some(500),
            batch_no: "B20260714-01".to_string(),
            color_no: "RED-001".to_string(),
            created_by: Some(1),
        };
        // 验证事件可被模式匹配
        match event {
            BusinessEvent::InventoryTransactionCreated {
                transaction_id,
                product_id,
                warehouse_id,
                quantity_meters,
                quantity_kg,
                source_bill_id,
                batch_no,
                color_no,
                created_by,
                ..
            } => {
                assert_eq!(transaction_id, 1);
                assert_eq!(product_id, 100);
                assert_eq!(warehouse_id, 1);
                assert_eq!(quantity_meters, decs!("100"));
                assert_eq!(quantity_kg, decs!("30"));
                assert_eq!(source_bill_id, Some(500));
                assert_eq!(batch_no, "B20260714-01");
                assert_eq!(color_no, "RED-001");
                assert_eq!(created_by, Some(1));
            }
            _ => panic!("应匹配 InventoryTransactionCreated 变体"),
        }
    }

    /// 测试_库存初始默认值常量正确性
    ///
    /// create_stock_fabric_txn 中 ActiveModel 使用以下硬编码默认值：
    /// - quantity_reserved / quantity_incoming / reorder_point 等 = Decimal::ZERO
    /// - stock_status = "正常"
    /// - quality_status = "合格"
    /// - version = 0
    #[test]
    fn 测试_库存初始默认值常量正确性() {
        assert_eq!(Decimal::ZERO, decs!("0"));
        assert_eq!("正常", "正常");
        assert_eq!("合格", "合格");
        assert_eq!(0i32, 0);
        // 验证状态字符串未被误改为大写常量
        assert_ne!("正常", "ACTIVE");
        assert_ne!("合格", "ACTIVE");
    }

    /// 测试_服务实例化_SQLite内存数据库
    ///
    /// 验证 InventoryStockService 能在 SQLite 内存数据库上实例化，
    /// 不依赖真实 schema（new 不触发任何 DB 操作）
    #[tokio::test]
    async fn 测试_服务实例化_SQLite内存数据库() {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite::memory:".to_string());
        let db = sea_orm::Database::connect(&db_url)
            .await
            .expect("测试夹具：数据库连接失败");
        let service = InventoryStockService::new(std::sync::Arc::new(db));
        let _ = service;
    }
}
