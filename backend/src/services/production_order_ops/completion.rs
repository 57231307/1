//! 生产订单-完成与库存联动子模块（production_order_ops/completion）
//!
//! 批次 488 D10-2 拆分：从原 `production_order_service.rs` L626-1243 迁移。
//! 包含 20 个完成生产订单相关方法：
//! - check_capacity_for_scheduling（`pub(crate)` &self，被 crud 子模块 update_status 跨 impl 块调用）
//! - apply_status_update_with_audit（`pub(crate)` associated fn，被 crud 子模块 update_status 跨 impl 块调用）
//! - lock_and_validate_order_for_completion_txn / build_completed_active_model / publish_pending_events（私有 associated fn）
//! - record_production_cost（私有 &self）
//! - complete_production_order（`pub(crate)` &self，被 crud 子模块 update_status 跨 impl 块调用）
//! - handle_production_completion_inventory_txn / fetch_default_warehouse_txn / deduct_raw_materials_txn（私有 associated fn）
//! - lookup_default_bom / lookup_bom_items / batch_load_stock_records / deduct_for_each_bom_item（私有 associated fn）
//! - calculate_deduct_qty / record_production_consumption（私有 associated fn）
//! - increase_finished_goods_txn / update_existing_finished_stock_txn / create_new_finished_stock_txn（私有 associated fn）
//! - record_production_output_txn（私有 associated fn）
//!
//! 业务规则：
//! - COMPLETED 状态变更在事务内执行（状态变更 + 库存联动），任一失败回滚全部
//! - 原材料扣减按 BOM 用量 × 生产数量，加 FOR UPDATE 行锁防止并发丢失更新
//! - 成品入库按克重/幅宽换算 kg，加 FOR UPDATE 行锁防止并发丢失更新
//! - 库存流水事件收集后由 commit 成功后统一 publish，避免事务回滚时幻事件
//! - 排产状态变更走产能负荷校验（load_rate > 100 阻塞，> 80 warn）
//! - 成本归集失败仅 warn 不传播，保持原逻辑

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};

use crate::models::bom::{Column as BomColumn, Entity as BomEntity};
use crate::models::bom_item::{Column as BomItemColumn, Entity as BomItemEntity};
use crate::models::inventory_stock::Entity as InventoryStockEntity;
use crate::models::product::Entity as ProductEntity;
use crate::models::production_order::{
    ActiveModel, Entity as ProductionOrderEntity, Model as ProductionOrderModel,
};
use crate::models::warehouse::Entity as WarehouseEntity;
use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
use crate::utils::error::AppError;

use super::types::ProductionOutputRecord;
use crate::services::production_order_service::ProductionOrderService;

impl ProductionOrderService {
    /// 排产状态变更时进行产能负荷校验
    ///
    /// 批次 386 v13 复审 B-P2-5 修复：原 update_status 直接更新状态，
    /// 未校验工作中心产能负荷，导致超载工作中心仍可排产。
    pub(crate) async fn check_capacity_for_scheduling(
        &self,
        model: &ProductionOrderModel,
        order_id: i32,
        status: &str,
    ) -> Result<(), AppError> {
        if status != crate::models::status::production::PRODUCTION_SCHEDULED {
            return Ok(());
        }
        let Some(work_center_id) = model.work_center_id else {
            return Ok(());
        };
        let capacity_service =
            crate::services::capacity_service::CapacityService::new(self.db.clone());
        let analysis = capacity_service
            .load_analysis(crate::services::capacity_service::LoadAnalysisQuery {
                date_from: model.planned_start_date,
                date_to: model.planned_end_date,
                work_center_id: Some(work_center_id),
            })
            .await?;
        // 检查目标工作中心的负荷率（load_rate > 100 视为超载）
        if let Some(item) = analysis.iter().find(|i| i.work_center_id == work_center_id) {
            if item.load_rate > Decimal::from(100) {
                return Err(AppError::business(format!(
                    "工作中心 {}（{}）当前负荷率 {:.2}% 已超载，无法排产，请调整计划或分配至其他工作中心",
                    item.work_center_name, item.work_center_code, item.load_rate
                )));
            }
            if item.load_rate > Decimal::from(80) {
                tracing::warn!(
                    order_id,
                    work_center_id,
                    load_rate = %item.load_rate,
                    "批次 386 B-P2-5: 工作中心负荷率较高（>80%），排产成功但建议关注产能瓶颈"
                );
            }
        }
        Ok(())
    }

    /// 应用状态变更并写入审计日志（生产中状态同时设置实际开始日期）
    pub(crate) async fn apply_status_update_with_audit(
        txn: &sea_orm::DatabaseTransaction,
        model: ProductionOrderModel,
        status: String,
        audit_user_id: i32,
    ) -> Result<ProductionOrderModel, AppError> {
        let mut active_model: ActiveModel = model.into();
        active_model.status = Set(status.clone());
        active_model.updated_at = Set(Utc::now());
        // 如果状态变为生产中，设置实际开始日期
        if status == crate::models::status::production::PRODUCTION_IN_PROGRESS {
            active_model.actual_start_date = Set(Some(chrono::Utc::now().date_naive()));
        }
        // 走 update_with_audit 保留审计追溯
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            active_model,
            Some(audit_user_id),
        )
        .await?;
        Ok(updated)
    }

    /// FOR UPDATE 行锁查询订单并校验状态转换合法性
    async fn lock_and_validate_order_for_completion_txn(
        txn: &sea_orm::DatabaseTransaction,
        id: i32,
    ) -> Result<ProductionOrderModel, AppError> {
        let model = ProductionOrderEntity::find_by_id(id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found("生产订单不存在"))?;
        Self::validate_status_transition(
            &model.status,
            crate::models::status::common::STATUS_COMPLETED,
        )?;
        Ok(model)
    }

    /// 构造完成状态 ActiveModel（状态 + 完工日期 + 实际数量）
    fn build_completed_active_model(
        model: ProductionOrderModel,
        actual_quantity: Option<Decimal>,
    ) -> ActiveModel {
        let mut active_model: ActiveModel = model.into();
        active_model.status = Set(crate::models::status::common::STATUS_COMPLETED.to_string());
        active_model.actual_end_date = Set(Some(chrono::Utc::now().date_naive()));
        active_model.updated_at = Set(Utc::now());
        if let Some(qty) = actual_quantity {
            active_model.actual_quantity = Set(Some(qty));
        }
        active_model
    }

    /// commit 成功后统一发布库存流水事件，避免事务回滚时幻事件
    fn publish_pending_events(pending_events: Vec<BusinessEvent>) {
        for ev in pending_events {
            EVENT_BUS.publish(ev);
        }
    }

    /// 生产订单成本归集（失败仅 warn 不传播，保持原逻辑）
    ///
    /// 批次 356 v13 复审 B-P0-3 修复：commit 成功后调用 CostCollectionService 做成本归集，
    /// 避免生产成本无法归集导致产品成本失真、BI 报表成本数据缺失。
    async fn record_production_cost(&self, updated: &ProductionOrderModel) {
        let cost_service =
            crate::services::cost_collection_service::CostCollectionService::new(self.db.clone());
        let product = ProductEntity::find_by_id(updated.product_id)
            .one(&*self.db)
            .await
            .ok()
            .flatten();
        let cost_price = product
            .as_ref()
            .and_then(|p| p.cost_price)
            .unwrap_or(Decimal::ZERO);
        let actual_qty = updated.actual_quantity.unwrap_or(updated.planned_quantity);
        let total_material_cost = cost_price * actual_qty;
        let cost_req = crate::services::cost_collection_service::CreateCostCollectionRequest {
            collection_date: chrono::Utc::now().date_naive(),
            cost_object_type: Some("production_order".to_string()),
            cost_object_id: Some(updated.id),
            cost_object_no: Some(updated.order_no.clone()),
            batch_no: None,
            color_no: None,
            // v14 批次 422 T-P1-6：按缸号核算成本（生产订单当前无缸号，后续批次补全）
            dye_lot_no: None,
            workshop: None,
            direct_material: total_material_cost,
            direct_labor: Decimal::ZERO,
            manufacturing_overhead: Decimal::ZERO,
            processing_fee: Decimal::ZERO,
            dyeing_fee: Decimal::ZERO,
            output_quantity_meters: Some(actual_qty),
            output_quantity_kg: None,
        };
        if let Err(e) = cost_service.create(cost_req, updated.created_by).await {
            tracing::warn!(
                order_id = updated.id,
                error = %e,
                "批次 356 B-P0-3: 生产订单成本归集失败，请人工检查"
            );
        }
    }

    /// 完成生产订单（事务包裹状态变更 + 库存联动）
    ///
    /// 批次 9（2026-06-28）：原 `update_status` 在 COMPLETED 时先提交状态变更，
    /// 然后调用库存联动；如果库存联动失败，状态已变更但库存未扣减导致账实不符。
    /// 改为：在事务内更新状态 + 调用库存联动，任一失败回滚全部。
    /// 同时给订单查询加 FOR UPDATE 行锁，防止并发完成同一订单。
    pub(crate) async fn complete_production_order(
        &self,
        id: i32,
        actual_quantity: Option<Decimal>,
    ) -> Result<ProductionOrderModel, AppError> {
        let txn = self.db.begin().await?;

        let model =
            Self::lock_and_validate_order_for_completion_txn(&txn, id).await?;
        let active_model = Self::build_completed_active_model(model, actual_quantity);
        let updated = active_model.update(&txn).await?;

        // 在同一事务内执行库存联动（含原材料扣减 + 成品入库）
        // P0 5-2 修复：handle_production_completion_inventory_txn 不再在内部 publish 事件，
        // 改为返回收集到的库存流水事件，由本处在 commit 成功后统一 publish，避免事务回滚时幻事件
        let pending_events =
            Self::handle_production_completion_inventory_txn(&txn, &updated).await?;

        txn.commit().await?;

        Self::publish_pending_events(pending_events);
        self.record_production_cost(&updated).await;

        Ok(updated)
    }

    /// 处理生产完成时的库存联动（事务版本）
    ///
    /// 1. 查询产品默认BOM，扣减原材料库存（按BOM用量 × 生产数量）
    /// 2. 增加成品库存（生产数量）
    /// 3. 记录库存流水（PRODUCTION_CONSUMPTION 和 PRODUCTION_OUTPUT）
    ///
    /// 批次 9（2026-06-28）：从原 `handle_production_completion_inventory` 改造而来，
    /// 接受外部事务参数，所有查询/更新都在 `txn` 上执行；原材料库存查询加 FOR UPDATE 行锁，
    /// 防止并发完成多个生产订单时原材料库存被并发扣减导致丢失更新。
    ///
    /// P2 1-4 修复：原函数 275 行混合 5 职责（仓库查询+数量校验+原材料扣减+成品入库+日志），
    /// 拆为 fetch_default_warehouse_txn / deduct_raw_materials_txn / increase_finished_goods_txn 3 个私有方法
    async fn handle_production_completion_inventory_txn(
        txn: &sea_orm::DatabaseTransaction,
        order: &ProductionOrderModel,
    ) -> Result<Vec<BusinessEvent>, AppError> {
        // P0 5-2 修复：本函数不 commit 事务（由调用方 complete_production_order commit），
        // 收集 record_transaction_txn 返回的库存流水事件交给调用方，
        // 在 commit 成功后统一 publish，避免事务回滚时幻事件
        let mut pending_events: Vec<BusinessEvent> = Vec::new();

        // 1. 查询默认成品仓库
        let default_warehouse = Self::fetch_default_warehouse_txn(txn).await?;

        // 2. 校验生产数量
        let production_qty = order.actual_quantity.unwrap_or(order.planned_quantity);
        if production_qty.is_zero() {
            return Err(AppError::business(
                "生产数量为零，无法执行库存联动".to_string(),
            ));
        }

        // 3. 扣减原材料库存
        pending_events.extend(
            Self::deduct_raw_materials_txn(txn, order, &default_warehouse, production_qty).await?,
        );

        // 4. 增加成品库存
        pending_events.extend(
            Self::increase_finished_goods_txn(txn, order, &default_warehouse, production_qty)
                .await?,
        );

        tracing::info!(
            "生产订单 {} 完成库存联动：成品入库 {}，已扣减原材料库存",
            order.order_no,
            production_qty
        );

        Ok(pending_events)
    }

    /// P2 1-4 修复：查询默认成品仓库（取第一个激活的仓库，从 handle_production_completion_inventory_txn 抽取）
    async fn fetch_default_warehouse_txn(
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<crate::models::warehouse::Model, AppError> {
        WarehouseEntity::find()
            .filter(crate::models::warehouse::Column::IsActive.eq(true))
            .one(txn)
            .await?
            .ok_or_else(|| AppError::business("未找到可用仓库，无法执行库存联动"))
    }

    /// P2 1-4 修复：扣减原材料库存（从 handle_production_completion_inventory_txn 抽取）
    ///
    /// 查询产品默认BOM，按BOM用量 × 生产数量扣减原材料库存，记录 PRODUCTION_CONSUMPTION 流水
    /// v16 批次 43 修复：循环外批量查询并锁定所有原材料库存记录，避免 N+1 查询
    /// 批次 9（2026-06-28）：FOR UPDATE 行锁批量获取，防止并发扣减丢失更新
    async fn deduct_raw_materials_txn(
        txn: &sea_orm::DatabaseTransaction,
        order: &ProductionOrderModel,
        default_warehouse: &crate::models::warehouse::Model,
        production_qty: Decimal,
    ) -> Result<Vec<BusinessEvent>, AppError> {
        let bom = Self::lookup_default_bom(txn, order.product_id).await?;

        if let Some(bom) = bom {
            let bom_items = Self::lookup_bom_items(txn, bom.id).await?;
            let stock_map = Self::batch_load_stock_records(txn, &bom_items, default_warehouse).await?;
            Self::deduct_for_each_bom_item(txn, order, default_warehouse, production_qty, &bom_items, &stock_map).await
        } else {
            Ok(Vec::new())
        }
    }

    async fn lookup_default_bom(
        txn: &sea_orm::DatabaseTransaction,
        product_id: i32,
    ) -> Result<Option<crate::models::bom::Model>, AppError> {
        BomEntity::find()
            .filter(BomColumn::ProductId.eq(product_id))
            .filter(BomColumn::IsDefault.eq(true))
            .filter(BomColumn::Status.eq(crate::models::status::common::STATUS_ACTIVE))
            .one(txn)
            .await
            .map_err(AppError::from)
    }

    async fn lookup_bom_items(
        txn: &sea_orm::DatabaseTransaction,
        bom_id: i32,
    ) -> Result<Vec<crate::models::bom_item::Model>, AppError> {
        BomItemEntity::find()
            .filter(BomItemColumn::BomId.eq(bom_id))
            .all(txn)
            .await
            .map_err(AppError::from)
    }

    async fn batch_load_stock_records(
        txn: &sea_orm::DatabaseTransaction,
        bom_items: &[crate::models::bom_item::Model],
        default_warehouse: &crate::models::warehouse::Model,
    ) -> Result<std::collections::HashMap<i32, crate::models::inventory_stock::Model>, AppError> {
        let material_ids: Vec<i32> = bom_items.iter().map(|b| b.material_id).collect();

        if material_ids.is_empty() {
            Ok(std::collections::HashMap::new())
        } else {
            let stocks = InventoryStockEntity::find()
                .filter(
                    crate::models::inventory_stock::Column::ProductId.is_in(material_ids),
                )
                .filter(
                    crate::models::inventory_stock::Column::WarehouseId
                        .eq(default_warehouse.id),
                )
                .lock_exclusive()
                .all(txn)
                .await?;

            Ok(stocks.into_iter().map(|s| (s.product_id, s)).collect())
        }
    }

    async fn deduct_for_each_bom_item(
        txn: &sea_orm::DatabaseTransaction,
        order: &ProductionOrderModel,
        default_warehouse: &crate::models::warehouse::Model,
        production_qty: Decimal,
        bom_items: &[crate::models::bom_item::Model],
        stock_map: &std::collections::HashMap<i32, crate::models::inventory_stock::Model>,
    ) -> Result<Vec<BusinessEvent>, AppError> {
        use crate::services::inventory_stock_service::InventoryStockService;

        let mut pending_events: Vec<BusinessEvent> = Vec::new();

        for bom_item in bom_items {
            let consumption_qty = (bom_item.quantity * production_qty).round_dp(4);

            let stock_record = stock_map
                .get(&bom_item.material_id)
                .cloned()
                .ok_or_else(|| {
                    AppError::business(format!(
                        "原材料(ID={})在默认仓库中无库存记录，无法扣减",
                        bom_item.material_id
                    ))
                })?;

            let (qty_after_meters, qty_after_kg) = Self::calculate_deduct_qty(
                bom_item.material_id,
                stock_record.quantity_meters,
                stock_record.quantity_kg,
                consumption_qty,
            )?;

            InventoryStockService::update_stock_quantity_with_optimistic_lock_txn(
                txn,
                stock_record.id,
                qty_after_meters,
                qty_after_kg,
                stock_record.version,
            )
            .await?;

            if let Some(ev) = Self::record_production_consumption(
                txn,
                bom_item.material_id,
                default_warehouse.id,
                &stock_record,
                consumption_qty,
                qty_after_meters,
                qty_after_kg,
                order,
            ).await? {
                pending_events.push(ev);
            }
        }

        Ok(pending_events)
    }

    fn calculate_deduct_qty(
        material_id: i32,
        qty_before_meters: Decimal,
        qty_before_kg: Decimal,
        consumption_qty: Decimal,
    ) -> Result<(Decimal, Decimal), AppError> {
        if qty_before_meters < consumption_qty {
            return Err(AppError::business(format!(
                "原材料(ID={})库存不足，需要 {}，当前库存 {}",
                material_id, consumption_qty, qty_before_meters
            )));
        }

        let qty_after_meters = qty_before_meters - consumption_qty;

        let qty_after_kg = if qty_before_meters > Decimal::ZERO {
            qty_before_kg - (qty_before_kg * consumption_qty / qty_before_meters)
        } else {
            qty_before_kg
        };

        Ok((qty_after_meters, qty_after_kg))
    }

    async fn record_production_consumption(
        txn: &sea_orm::DatabaseTransaction,
        material_id: i32,
        warehouse_id: i32,
        stock_record: &crate::models::inventory_stock::Model,
        consumption_qty: Decimal,
        qty_after_meters: Decimal,
        qty_after_kg: Decimal,
        order: &ProductionOrderModel,
    ) -> Result<Option<BusinessEvent>, AppError> {
        use crate::services::inventory_stock_query::RecordTransactionArgs;
        use crate::services::inventory_stock_service::InventoryStockService;

        let (_, txn_event) = InventoryStockService::record_transaction_txn(
            txn,
            RecordTransactionArgs {
                transaction_type: "PRODUCTION_CONSUMPTION".to_string(),
                product_id: material_id,
                warehouse_id,
                batch_no: stock_record.batch_no.clone(),
                color_no: stock_record.color_no.clone(),
                dye_lot_no: stock_record.dye_lot_no.clone(),
                grade: stock_record.grade.clone(),
                quantity_meters: consumption_qty,
                quantity_kg: Decimal::ZERO,
                source_bill_type: Some("production_order".to_string()),
                source_bill_no: Some(order.order_no.clone()),
                source_bill_id: Some(order.id),
                quantity_before_meters: Some(stock_record.quantity_meters),
                quantity_before_kg: Some(stock_record.quantity_kg),
                quantity_after_meters: Some(qty_after_meters),
                quantity_after_kg: Some(qty_after_kg),
                notes: Some(format!("生产消耗 - 订单 {}", order.order_no)),
                created_by: Some(order.created_by),
            },
        )
        .await?;

        Ok(txn_event)
    }

    /// P2 1-4 修复：增加成品库存（从 handle_production_completion_inventory_txn 抽取）
    ///
    /// 查询成品产品信息（克重/幅宽），在默认仓库更新或创建库存记录，记录 PRODUCTION_OUTPUT 流水
    /// 批次 9（2026-06-28）：加 FOR UPDATE 行锁，防止并发入库丢失更新
    async fn increase_finished_goods_txn(
        txn: &sea_orm::DatabaseTransaction,
        order: &ProductionOrderModel,
        default_warehouse: &crate::models::warehouse::Model,
        production_qty: Decimal,
    ) -> Result<Vec<BusinessEvent>, AppError> {
        let mut pending_events: Vec<BusinessEvent> = Vec::new();

        let product = ProductEntity::find_by_id(order.product_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::business(format!("产品ID {} 不存在", order.product_id)))?;

        // 批次 9（2026-06-28）：加 FOR UPDATE 行锁，防止并发入库丢失更新
        let existing_stock = InventoryStockEntity::find()
            .filter(crate::models::inventory_stock::Column::ProductId.eq(order.product_id))
            .filter(crate::models::inventory_stock::Column::WarehouseId.eq(default_warehouse.id))
            .lock_exclusive()
            .one(txn)
            .await?;

        let txn_event = match existing_stock {
            Some(stock_record) => Self::update_existing_finished_stock_txn(
                txn, order, default_warehouse, &product, stock_record, production_qty,
            )
            .await?,
            None => Self::create_new_finished_stock_txn(
                txn, order, default_warehouse, &product, production_qty,
            )
            .await?,
        };

        if let Some(ev) = txn_event {
            pending_events.push(ev);
        }

        Ok(pending_events)
    }

    /// P0-D08 拆分：更新已有成品库存并记录 PRODUCTION_OUTPUT 流水
    async fn update_existing_finished_stock_txn(
        txn: &sea_orm::DatabaseTransaction,
        order: &ProductionOrderModel,
        default_warehouse: &crate::models::warehouse::Model,
        product: &crate::models::product::Model,
        stock_record: crate::models::inventory_stock::Model,
        production_qty: Decimal,
    ) -> Result<Option<BusinessEvent>, AppError> {
        use crate::services::inventory_stock_service::InventoryStockService;

        let qty_before_meters = stock_record.quantity_meters;
        let qty_before_kg = stock_record.quantity_kg;
        let qty_after_meters = qty_before_meters + production_qty;

        let added_kg = if let (Some(gw), Some(w)) = (product.gram_weight, product.width) {
            production_qty * gw * w / Decimal::new(100000, 0)
        } else {
            Decimal::ZERO
        };
        let qty_after_kg = qty_before_kg + added_kg;

        InventoryStockService::update_stock_quantity_with_optimistic_lock_txn(
            txn,
            stock_record.id,
            qty_after_meters,
            qty_after_kg,
            stock_record.version,
        )
        .await?;

        let record = ProductionOutputRecord {
            batch_no: stock_record.batch_no.clone(),
            color_no: stock_record.color_no.clone(),
            dye_lot_no: stock_record.dye_lot_no.clone(),
            grade: stock_record.grade.clone(),
            added_kg,
            qty_before_meters,
            qty_before_kg,
            qty_after_meters,
            qty_after_kg,
        };

        Self::record_production_output_txn(txn, order, default_warehouse, production_qty, record).await
    }

    /// P0-D08 拆分：创建新成品库存记录并记录 PRODUCTION_OUTPUT 流水
    async fn create_new_finished_stock_txn(
        txn: &sea_orm::DatabaseTransaction,
        order: &ProductionOrderModel,
        default_warehouse: &crate::models::warehouse::Model,
        product: &crate::models::product::Model,
        production_qty: Decimal,
    ) -> Result<Option<BusinessEvent>, AppError> {
        use crate::services::inventory_stock_service::{CreateStockFabricArgs, InventoryStockService};
        let kg = if let (Some(gw), Some(w)) = (product.gram_weight, product.width) {
            production_qty * gw * w / Decimal::new(100000, 0)
        } else {
            Decimal::ZERO
        };

        // v14 批次 419 修复 F-P0-1：从订单获取缸号/色号/批号，替代原 "DEFAULT" 硬编码
        let new_stock = InventoryStockService::create_stock_fabric_txn(
            txn,
            CreateStockFabricArgs {
                warehouse_id: default_warehouse.id,
                product_id: order.product_id,
                batch_no: order.batch_no.clone().unwrap_or_else(|| order.order_no.clone()),
                color_no: order.color_no.clone().unwrap_or_default(),
                dye_lot_no: order.dye_lot_no.clone(),
                grade: "一等品".to_string(),
                quantity_meters: production_qty,
                quantity_kg: kg,
                gram_weight: product.gram_weight,
                width: product.width,
                location_id: None,
                shelf_no: None,
                layer_no: None,
            },
        )
        .await?;

        let record = ProductionOutputRecord {
            batch_no: new_stock.batch_no.clone(),
            color_no: new_stock.color_no.clone(),
            dye_lot_no: new_stock.dye_lot_no.clone(),
            grade: new_stock.grade.clone(),
            added_kg: kg,
            qty_before_meters: Decimal::ZERO,
            qty_before_kg: Decimal::ZERO,
            qty_after_meters: production_qty,
            qty_after_kg: kg,
        };

        Self::record_production_output_txn(txn, order, default_warehouse, production_qty, record).await
    }

    /// P0-D08 拆分：记录 PRODUCTION_OUTPUT 库存流水（P0 5-2：返回事件由调用方收集后 commit 后统一 publish）
    async fn record_production_output_txn(
        txn: &sea_orm::DatabaseTransaction,
        order: &ProductionOrderModel,
        default_warehouse: &crate::models::warehouse::Model,
        production_qty: Decimal,
        record: ProductionOutputRecord,
    ) -> Result<Option<BusinessEvent>, AppError> {
        use crate::services::inventory_stock_query::RecordTransactionArgs;
        use crate::services::inventory_stock_service::InventoryStockService;

        let (_, txn_event) = InventoryStockService::record_transaction_txn(
            txn,
            RecordTransactionArgs {
                transaction_type: "PRODUCTION_OUTPUT".to_string(),
                product_id: order.product_id,
                warehouse_id: default_warehouse.id,
                batch_no: record.batch_no,
                color_no: record.color_no,
                dye_lot_no: record.dye_lot_no,
                grade: record.grade,
                quantity_meters: production_qty,
                quantity_kg: record.added_kg,
                source_bill_type: Some("production_order".to_string()),
                source_bill_no: Some(order.order_no.clone()),
                source_bill_id: Some(order.id),
                quantity_before_meters: Some(record.qty_before_meters),
                quantity_before_kg: Some(record.qty_before_kg),
                quantity_after_meters: Some(record.qty_after_meters),
                quantity_after_kg: Some(record.qty_after_kg),
                notes: Some(format!("生产入库 - 订单 {}", order.order_no)),
                created_by: Some(order.created_by),
            },
        )
        .await?;

        Ok(txn_event)
    }
}
