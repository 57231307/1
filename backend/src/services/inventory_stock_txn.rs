//! 库存事务版本方法（带 _txn 后缀，与外层同名方法行为一致但接受外部事务）
//!
//! 拆分自 inventory_stock_service.rs：原 4 个 _txn 方法独立成文件。

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, ExprTrait, QueryFilter, Set, TransactionTrait,
};

use crate::models::inventory_stock;
use crate::models::inventory_transaction;
use crate::models::status::purchase_inventory::inventory_stock_quality_status as quality_status;
use crate::models::status::purchase_inventory::inventory_stock_status;
use crate::services::event_bus::BusinessEvent;
use crate::utils::error::AppError;

use super::inventory_stock_query::RecordTransactionArgs;
use super::inventory_stock_service::{
    CreateStockFabricArgs, InventoryStockService, StockInboundFlowRef,
};

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
                sea_orm::sea_query::Expr::val(quantity_meters),
            )
            .col_expr(
                inventory_stock::Column::QuantityAvailable,
                sea_orm::sea_query::Expr::val(quantity_meters),
            )
            .col_expr(
                inventory_stock::Column::QuantityMeters,
                sea_orm::sea_query::Expr::val(quantity_meters),
            )
            .col_expr(
                inventory_stock::Column::QuantityKg,
                sea_orm::sea_query::Expr::val(quantity_kg),
            )
            .col_expr(
                inventory_stock::Column::Version,
                sea_orm::sea_query::Expr::col(inventory_stock::Column::Version).add(1),
            )
            .col_expr(
                inventory_stock::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()),
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
    /// 入参 `args` 是 `CreateStockFabricArgs` 参数对象（txn + args 两参数签名，避免
    /// `clippy::too_many_arguments`）。
    ///
    /// `quantity_kg` 按调用方给出的权威辅量原样落库，不在本函数再做第二次推算。公斤数的
    /// 解析唯一入口是 `services::inv::inbound_admission::resolve_inbound_quantity_kg`
    /// （实称优先、缺失时按克重与幅宽经 `DualUnitConverter` 单入口推算、两条来源都缺即拒绝）。
    /// 此前本函数推算出一个 `_final_quantity_kg` 后弃用、仍写 `quantity_kg`，等于换算结果
    /// 从不生效而调用方以为生效，属双计量的假象口径，故删除该次无效计算。
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
            stock_status: Set(inventory_stock_status::NORMAL.to_string()),
            quality_status: Set(quality_status::PASS.to_string()),
            version: Set(0),
            replenishment_strategy: Set("reorder_point".to_string()),
        };

        active_stock.insert(txn).await.map_err(AppError::from)
    }

    /// 记录库存流水（事务版本）
    /// 批次 338 v10 复审 P3 修复：签名从 19 参数改为 2 参数（txn + args），；复用 `RecordTransactionArgs` 参数对象，消除 `clippy::too_many_arguments` 警告。
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
    fn build_transaction_event(transaction: &inventory_transaction::Model) -> BusinessEvent {
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

    /// 创建库存行并在同一事务内写入库流水（库存直建入库的唯一写入口）
    ///
    /// 调用方是 `handlers::inventory_stock_handler::create_stock`（POST /inventory/stock）与
    /// `handlers::inventory_stock_handler_fabric::create_stock_fabric`（POST /inventory/stock/fabric）。
    /// 入参 `stock_args` 是库存行的全部写入值（含已由 `inbound_admission` 解析出的公斤数），
    /// `flow` 是这笔入库的来源单据引用与登记人（`StockInboundFlowRef`）。
    /// 传给谁是先写 `inventory_stocks` 一行、再以同一组四维与数量写 `inventory_transactions`
    /// 一行，事务提交成功后发布 `BusinessEvent::InventoryTransactionCreated`；任一步失败整个
    /// 事务回滚，不留下"有台账无流水"或"有流水无台账"的半态。
    /// 存什么存哪里是流水行的方向类型、来源单号与单 ID、四维（批次/色号/缸号/等级）、
    /// 双计量数量与前后值。期初固定为 0，因为本入口每次新增一行而非累加既有行。
    ///
    /// 流水维度一律取回读到的 `stock` 行本身而不是请求原文，保证库存台账与流水逐字符同源；
    /// 此前两条直建端点完全不写流水，入库无痕导致同仓 9 条出入库链路共用的流水视图、
    /// 库存财务桥与批次追溯链都看不到这笔入库。
    pub async fn create_stock_fabric_with_flow(
        &self,
        stock_args: CreateStockFabricArgs,
        flow: StockInboundFlowRef,
    ) -> Result<inventory_stock::Model, AppError> {
        let txn = (*self.db).begin().await?;

        let stock = Self::create_stock_fabric_txn(&txn, stock_args).await?;

        let (_, flow_event) = Self::record_transaction_txn(
            &txn,
            RecordTransactionArgs {
                transaction_type: flow.transaction_type,
                product_id: stock.product_id,
                warehouse_id: stock.warehouse_id,
                batch_no: stock.batch_no.clone(),
                color_no: stock.color_no.clone(),
                dye_lot_no: stock.dye_lot_no.clone(),
                grade: stock.grade.clone(),
                quantity_meters: stock.quantity_meters,
                quantity_kg: stock.quantity_kg,
                source_bill_type: flow.source_bill_type,
                source_bill_no: flow.source_bill_no,
                source_bill_id: flow.source_bill_id,
                quantity_before_meters: Some(Decimal::ZERO),
                quantity_before_kg: Some(Decimal::ZERO),
                quantity_after_meters: Some(stock.quantity_meters),
                quantity_after_kg: Some(stock.quantity_kg),
                notes: flow.notes,
                created_by: flow.created_by,
            },
        )
        .await?;

        txn.commit().await?;

        if let Some(event) = flow_event {
            crate::services::event_bus::EVENT_BUS.publish(event);
        }

        Ok(stock)
    }
}
