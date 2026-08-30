use crate::models::inventory_stock;
use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
use crate::utils::dual_unit_converter::DualUnitConverter;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, ExprTrait, PaginatorTrait, QueryFilter, Set,
    TransactionTrait,
};
use std::sync::Arc;

/// 库存汇总项（用于返回汇总数据）
#[derive(Debug, Clone)]
#[allow(dead_code, reason = "预留：库存汇总项，待接入")]
pub struct InventorySummaryItem {
    pub product_id: i32,
    pub product_name: String,
    pub specification: Option<String>,
    pub color_no: String,
    pub batch_no: String,
    pub grade: String,
    pub warehouse_id: i32,
    pub warehouse_name: String,
    pub quantity: Decimal,
    pub unit: String,
    pub total_value: Option<Decimal>,
    pub total_quantity_meters: Decimal,
    pub total_quantity_kg: Decimal,
}

/// 库存汇总查询结果（内部使用）
#[derive(Debug, Clone, sea_orm::FromQueryResult)]
pub struct InventorySummaryQueryResult {
    pub product_id: i32,
    pub product_name: String,
    pub warehouse_id: i32,
    pub warehouse_name: String,
    pub batch_no: String,
    pub color_no: String,
    pub grade: String,
    pub total_quantity_meters: Decimal,
    pub total_quantity_kg: Decimal,
}

/// 库存服务（面料行业版）
///
/// P1 batch-18 缺陷 7.2：检测到告警时同步推送站内信+邮件给计划员/仓管员
pub struct InventoryStockService {
    pub db: Arc<DatabaseConnection>,
    /// 事件通知服务（用于库存告警主动通知）
    pub(crate) notification_service:
        Option<crate::services::event_notification_service::EventNotificationService>,
}

/// 创建库存参数对象（批次 338 v10 复审 P3 修复：引入参数对象消除 create_stock 的 too_many_arguments 警告。；聚合创建库存记录所需的全部字段，避免函数签名携带 12 个参数。）
#[derive(Debug, Clone)]
pub struct CreateStockArgs {
    /// 仓库 ID
    pub warehouse_id: i32,
    /// 产品 ID
    pub product_id: i32,
    /// 批次号
    pub batch_no: String,
    /// 色号
    pub color_no: String,
    /// 数量（米）
    pub quantity_meters: Decimal,
    /// 数量（公斤）
    pub quantity_kg: Decimal,
    /// 等级
    pub grade: String,
    /// 染缸批号（可选）
    pub dye_lot_no: Option<String>,
    /// 克重（可选）
    pub gram_weight: Option<Decimal>,
    /// 幅宽（可选）
    pub width: Option<Decimal>,
    /// 库存状态
    pub stock_status: String,
    /// 质量状态
    pub quality_status: String,
}

/// 创建面料库存参数对象
/// 批次 338 v10 复审 P3 修复：引入参数对象消除 create_stock_fabric 的 too_many_arguments 警告。；聚合创建面料库存记录所需的全部字段，避免函数签名携带 13 个参数。
#[derive(Debug, Clone)]
pub struct CreateStockFabricArgs {
    /// 仓库 ID
    pub warehouse_id: i32,
    /// 产品 ID
    pub product_id: i32,
    /// 批次号
    pub batch_no: String,
    /// 色号
    pub color_no: String,
    /// 染缸批号（可选）
    pub dye_lot_no: Option<String>,
    /// 等级
    pub grade: String,
    /// 数量（米）
    pub quantity_meters: Decimal,
    /// 数量（公斤）
    pub quantity_kg: Decimal,
    /// 克重（可选）
    pub gram_weight: Option<Decimal>,
    /// 幅宽（可选）
    pub width: Option<Decimal>,
    /// 库位 ID（可选）
    pub location_id: Option<i32>,
    /// 货架号（可选）
    pub shelf_no: Option<String>,
    /// 层号（可选）
    pub layer_no: Option<String>,
}

/// build_stock_fabric_active 内部传递字段（解构 CreateStockFabricArgs 后 + 计算的 final_quantity_kg）
/// 避免主函数调用 helper 时传递 13 个参数，使用结构体封装更整洁。

/// 创建面料批次参数对象（缺陷 3 修复：消除 create_batch_fabric 的 too_many_arguments 警告）
#[derive(Debug, Clone)]
pub struct CreateBatchFabricArgs {
    /// 批次号
    pub batch_no: String,
    /// 产品 ID
    pub product_id: i32,
    /// 仓库 ID
    pub warehouse_id: i32,
    /// 色号
    pub color_no: String,
    /// 染缸批号（可选）
    pub dye_lot_no: Option<String>,
    /// 等级
    pub grade: String,
    /// 数量（米）
    pub quantity_meters: f64,
    /// 数量（公斤）
    pub quantity_kg: f64,
    /// 克重（可选）
    pub gram_weight: Option<f64>,
    /// 幅宽（可选）
    pub width: Option<f64>,
    /// 生产日期（可选）
    pub production_date: Option<chrono::DateTime<Utc>>,
    /// 到期日期（可选）
    pub expiry_date: Option<chrono::DateTime<Utc>>,
}

struct StockFabricFields {
    warehouse_id: i32,
    product_id: i32,
    batch_no: String,
    color_no: String,
    dye_lot_no: Option<String>,
    grade: String,
    quantity_meters: Decimal,
    final_quantity_kg: Decimal,
    gram_weight: Option<Decimal>,
    width: Option<Decimal>,
    location_id: Option<i32>,
    shelf_no: Option<String>,
    layer_no: Option<String>,
}

impl InventoryStockService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db: db.clone(),
            // P1 batch-18 缺陷 7.2：默认注入 EventNotificationService 用于告警主动通知
            notification_service: Some(
                crate::services::event_notification_service::EventNotificationService::new(db),
            ),
        }
    }

    /// 构造不启用主动通知的服务实例（用于不需要通知的场景，如定时任务批量查询）
    pub fn without_notification(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            notification_service: None,
        }
    }

    pub async fn find_by_id(&self, id: i32) -> Result<inventory_stock::Model, AppError> {
        inventory_stock::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存记录 ID {} 不存在", id)))
    }

    /// 创建库存（批次 338 v10 复审 P3 修复：签名从 12 参数改为单一参数对象 `CreateStockArgs`，；消除 `clippy::too_many_arguments` 警告。）
    pub async fn create_stock(
        &self,
        args: CreateStockArgs,
    ) -> Result<inventory_stock::Model, AppError> {
        let CreateStockArgs {
            warehouse_id,
            product_id,
            batch_no,
            color_no,
            quantity_meters,
            quantity_kg,
            grade,
            dye_lot_no,
            gram_weight,
            width,
            stock_status,
            quality_status,
        } = args;
        // P2 5-23 修复：service 层校验仓库/产品存在性，外键完整性不再仅依赖数据库
        use crate::models::{product, warehouse};
        use sea_orm::EntityTrait;

        let _warehouse = warehouse::Entity::find_by_id(warehouse_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::validation(format!("仓库不存在: {}", warehouse_id)))?;
        let _product = product::Entity::find_by_id(product_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::validation(format!("产品不存在: {}", product_id)))?;

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
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
            // 面料行业特色字段
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
            location_id: Set(None),
            shelf_no: Set(None),
            layer_no: Set(None),
            bin_location: Set(None),
            stock_status: Set(stock_status),
            quality_status: Set(quality_status),
            version: Set(0),
            replenishment_strategy: Set("reorder_point".to_string()),
        };

        active_stock.insert(&*self.db).await.map_err(AppError::from)
    }

    pub async fn list_stock(
        &self,
        page: u64,
        page_size: u64,
        warehouse_id: Option<i32>,
        product_id: Option<i32>,
    ) -> Result<(Vec<inventory_stock::Model>, u64), AppError> {
        let mut query = inventory_stock::Entity::find();

        if let Some(wid) = warehouse_id {
            query = query.filter(inventory_stock::Column::WarehouseId.eq(wid));
        }

        if let Some(pid) = product_id {
            query = query.filter(inventory_stock::Column::ProductId.eq(pid));
        }

        // 批次 97 P1-15 修复（v5 复审）：接入 SlowQueryRecorder 真实使用，
        // 慢查询（>100ms）将记录到 tracing::warn! 与 Prometheus 指标。
        // 批次 263 修复：接入 paginate_with_total 工具函数，消除手写 num_items + fetch_page 重复。
        // paginate_with_total 内部已做 page.saturating_sub(1) 偏移，调用方不可再减 1。
        // 补 clamp(1, 1000) 防 DoS（恶意请求 page=999999 不会导致超大偏移查询）。
        let rec = crate::middleware::slow_query::SlowQueryRecorder::start(
            "inventory_stock_list",
            None,
            None,
        );
        let paginator = query.paginate(&*self.db, page_size);
        let (stock_list, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
        rec.finish();

        Ok((stock_list, total))
    }

    pub async fn check_low_stock(
        &self,
        warehouse_id: Option<i32>,
        product_id: Option<i32>,
        batch_no: Option<String>,
    ) -> Result<Vec<inventory_stock::Model>, AppError> {
        // P2 5-15/3-21 修复：查询改为 txn 内执行，commit 后批量 publish
        // 原实现查询无 txn 包裹，事件可能在查询后、发布前数据已变化（幻事件/过期值）
        let txn = self.db.begin().await?;

        // 实现基于仓库和批次的精确低库存检查
        let mut query = inventory_stock::Entity::find()
            // 只检查正常状态的库存
            .filter(inventory_stock::Column::StockStatus.eq("正常"))
            .filter(inventory_stock::Column::QualityStatus.eq("合格"))
            // 检查可用库存低于重新订购点
            .filter(
                sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityAvailable).lt(
                    sea_orm::sea_query::Expr::col(inventory_stock::Column::ReorderPoint),
                ),
            )
            // 只检查重新订购点大于0的记录
            .filter(inventory_stock::Column::ReorderPoint.gt(rust_decimal::Decimal::ZERO));

        if let Some(wid) = warehouse_id {
            query = query.filter(inventory_stock::Column::WarehouseId.eq(wid));
        }

        if let Some(pid) = product_id {
            query = query.filter(inventory_stock::Column::ProductId.eq(pid));
        }

        if let Some(batch) = batch_no {
            query = query.filter(inventory_stock::Column::BatchNo.eq(batch));
        }

        // txn 内查询，保证一致性快照
        let low_stock_items = query.all(&txn).await?;

        // 收集待发布事件，commit 成功后再批量 publish
        let pending_events: Vec<BusinessEvent> = low_stock_items
            .iter()
            .map(|item| {
                tracing::info!(
                    "检测到低库存: 产品ID={}, 仓库ID={}, 当前库存={}, 补货点={}, 补货量={}",
                    item.product_id,
                    item.warehouse_id,
                    item.quantity_available,
                    item.reorder_point,
                    item.reorder_quantity
                );
                BusinessEvent::LowStockAlert {
                    product_id: item.product_id,
                    warehouse_id: item.warehouse_id,
                    current_quantity: item.quantity_available,
                    reorder_point: item.reorder_point,
                    reorder_quantity: item.reorder_quantity,
                }
            })
            .collect();

        txn.commit().await?;

        // P2 5-15/3-21 修复：commit 成功后批量 publish，避免幻事件
        for event in pending_events {
            EVENT_BUS.publish(event);
        }

        Ok(low_stock_items)
    }

    pub async fn delete_stock(&self, id: i32, user_id: Option<i32>) -> Result<(), AppError> {
        // P3 3-31/5-28 修复：软删除改用 update_with_audit，补审计日志
        // 原实现直接 active_model.update(&*self.db) 绕过审计中间件
        let stock = self.find_by_id(id).await?;
        let mut active_model: inventory_stock::ActiveModel = stock.into();
        active_model.stock_status = Set("已删除".to_string());
        active_model.updated_at = Set(Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit::<
            inventory_stock::Entity,
            _,
            _,
        >(&*self.db, "inventory_stock", active_model, user_id)
        .await?;
        Ok(())
    }

    // ========== 面料行业特色方法 ==========

    /// 按批次 + 色号查询库存
    pub async fn find_by_batch_and_color(
        &self,
        batch_no: &str,
        color_no: &str,
        warehouse_id: Option<i32>,
    ) -> Result<Vec<inventory_stock::Model>, AppError> {
        let mut query = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::BatchNo.eq(batch_no))
            .filter(inventory_stock::Column::ColorNo.eq(color_no));

        if let Some(wid) = warehouse_id {
            query = query.filter(inventory_stock::Column::WarehouseId.eq(wid));
        }

        query.all(&*self.db).await.map_err(AppError::from)
    }

    // ========== 双计量单位自动计算辅助方法 ==========

    /// 自动计算公斤数（如果提供了克重和幅宽）
    pub fn calculate_quantity_kg(
        quantity_meters: Decimal,
        gram_weight: Option<Decimal>,
        width: Option<Decimal>,
        fallback_quantity_kg: Decimal,
    ) -> Decimal {
        if let Some(gram_weight) = gram_weight {
            if let Some(width) = width {
                // 使用双计量单位转换器进行精确计算
                match DualUnitConverter::meters_to_kg(quantity_meters, gram_weight, width) {
                    Ok(kg) => return kg,
                    Err(e) => {
                        // 如果计算失败，回退到直接传入的公斤数
                        tracing::warn!("双计量单位换算失败: {:?}，使用原始公斤数", e);
                        return fallback_quantity_kg;
                    }
                }
            }
        }
        fallback_quantity_kg
    }

    /// 创建库存（面料行业版）（批次 338 v10 复审 P3 修复：签名从 13 参数改为单一参数对象 `CreateStockFabricArgs`，；消除 `clippy::too_many_arguments` 警告。）
    pub async fn create_stock_fabric(
        &self,
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

        // P2 5-23 修复：service 层校验仓库/产品存在性，外键完整性不再仅依赖数据库
        Self::validate_stock_references(&*self.db, warehouse_id, product_id).await?;

        // 自动计算公斤数（如果提供了克重和幅宽）
        let final_quantity_kg =
            Self::calculate_quantity_kg(quantity_meters, gram_weight, width, quantity_kg);

        let active_stock = Self::build_stock_fabric_active(StockFabricFields {
            warehouse_id,
            product_id,
            batch_no,
            color_no,
            dye_lot_no,
            grade,
            quantity_meters,
            final_quantity_kg,
            gram_weight,
            width,
            location_id,
            shelf_no,
            layer_no,
        });

        active_stock.insert(&*self.db).await.map_err(AppError::from)
    }

    /// 校验仓库和产品存在性（service 层外键完整性防御）
    async fn validate_stock_references(
        db: &sea_orm::DatabaseConnection,
        warehouse_id: i32,
        product_id: i32,
    ) -> Result<(), AppError> {
        use crate::models::{product, warehouse};
        use sea_orm::EntityTrait;

        let _warehouse = warehouse::Entity::find_by_id(warehouse_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::validation(format!("仓库不存在: {}", warehouse_id)))?;
        let _product = product::Entity::find_by_id(product_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::validation(format!("产品不存在: {}", product_id)))?;
        Ok(())
    }

    /// 构建面料库存 ActiveModel（初始状态：正常/合格，version=0）
    fn build_stock_fabric_active(fields: StockFabricFields) -> inventory_stock::ActiveModel {
        let StockFabricFields {
            warehouse_id,
            product_id,
            batch_no,
            color_no,
            dye_lot_no,
            grade,
            quantity_meters,
            final_quantity_kg,
            gram_weight,
            width,
            location_id,
            shelf_no,
            layer_no,
        } = fields;
        inventory_stock::ActiveModel {
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
            // 面料行业字段
            batch_no: Set(batch_no),
            color_no: Set(color_no),
            dye_lot_no: Set(dye_lot_no),
            grade: Set(grade),
            production_date: Set(None),
            expiry_date: Set(None),
            quantity_meters: Set(quantity_meters),
            quantity_kg: Set(final_quantity_kg),
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
            replenishment_strategy: Set("reorder_point".to_string()),
        }
    }

    // ========== V15 Batch 479 P0-F18：返工/降级/报废联动 ==========

    /// P0-F18: 更新库存等级（降级处理）
    /// 业务场景：bulk_color_approval.downgrade() 触发，；将关联库存的 grade 从"一等品"降为"二等品"或"等外品"。；同时将 quality_status 改为"待检"（降级后需重新质检）。；参数：`stock_id`：库存记录 ID；`new_grade`：新等级值，仅允许 "一等品" / "二等品" / "等外品"；`user_id`：操作人 ID（用于审计日志）
    pub async fn update_stock_grade(
        &self,
        stock_id: i32,
        new_grade: String,
        user_id: Option<i32>,
    ) -> Result<inventory_stock::Model, AppError> {
        // 校验 new_grade 合法值（与 inventory_stock.rs Model.grade 注释一致）
        if !matches!(new_grade.as_str(), "一等品" | "二等品" | "等外品") {
            return Err(AppError::validation(format!(
                "非法等级值 {}，仅允许 一等品/二等品/等外品",
                new_grade
            )));
        }

        let stock = self.find_by_id(stock_id).await?;
        let mut active: inventory_stock::ActiveModel = stock.into();
        active.grade = Set(new_grade);
        // 降级后质量状态自动降为"待检"（需重新质检判定合格/不合格）
        active.quality_status = Set("待检".to_string());
        active.updated_at = Set(Utc::now());

        crate::services::audit_log_service::AuditLogService::update_with_audit::<
            inventory_stock::Entity,
            _,
            _,
        >(&*self.db, "inventory_stock", active, user_id)
        .await
    }

    /// P0-F18: 标记库存为报废
    /// 业务场景：bulk_color_approval.scrap() 触发，；将关联库存的 stock_status 改为"报废"、quality_status 改为"不合格"。；报废原因追加到 bin_location 字段保留可追溯性（不覆盖原有库位信息）。；参数：`stock_id`：库存记录 ID；`reason`：报废原因（写入 bin_location 末尾便于追溯）；`user_id`：操作人 ID（用于审计日志）
    pub async fn mark_stock_as_scrapped(
        &self,
        stock_id: i32,
        reason: String,
        user_id: Option<i32>,
    ) -> Result<inventory_stock::Model, AppError> {
        let stock = self.find_by_id(stock_id).await?;
        let prev_loc = stock.bin_location.clone();
        let mut active: inventory_stock::ActiveModel = stock.into();
        active.stock_status = Set("报废".to_string());
        active.quality_status = Set("不合格".to_string());
        // 在 bin_location 追加报废原因（保留原有库位信息便于追溯）
        let new_loc = match &prev_loc {
            Some(prev) if !prev.is_empty() => format!("{} [SCRAP:{}]", prev, reason),
            _ => format!("[SCRAP:{}]", reason),
        };
        active.bin_location = Set(Some(new_loc));
        active.updated_at = Set(Utc::now());

        crate::services::audit_log_service::AuditLogService::update_with_audit::<
            inventory_stock::Entity,
            _,
            _,
        >(&*self.db, "inventory_stock", active, user_id)
        .await
    }

    // ========== 缺陷 3 修复：批次 CRUD/调拨业务逻辑（原 inventory_batch_handler 内联逻辑下沉） ==========

    /// 批次列表查询（batch_no 非空记录，分页）
    pub async fn list_batches(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<inventory_stock::Model>, u64), AppError> {
        let page = page.clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
        let page_size = page_size.clamp(1, 100);
        let paginator = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::BatchNo.ne(""))
            .paginate(&*self.db, page_size);
        let batches = paginator
            .fetch_page(page.clamp(1, 1000).saturating_sub(1))
            .await
            .map_err(|e| AppError::database(format!("获取批次列表失败：{}", e)))?;
        let total = paginator
            .num_items()
            .await
            .map_err(|e| AppError::database(format!("获取批次总数失败：{}", e)))?;
        Ok((batches, total))
    }

    /// 创建批次（入库，面料行业版）
    pub async fn create_batch_fabric(
        &self,
        args: CreateBatchFabricArgs,
    ) -> Result<inventory_stock::Model, AppError> {
        let CreateBatchFabricArgs {
            batch_no,
            product_id,
            warehouse_id,
            color_no,
            dye_lot_no,
            grade,
            quantity_meters,
            quantity_kg,
            gram_weight,
            width,
            production_date,
            expiry_date,
        } = args;
        let meters = Decimal::from_f64_retain(quantity_meters).unwrap_or(Decimal::ZERO);
        let kg = Decimal::from_f64_retain(quantity_kg).unwrap_or(Decimal::ZERO);
        let batch = inventory_stock::ActiveModel {
            id: Set(0),
            warehouse_id: Set(warehouse_id),
            product_id: Set(product_id),
            batch_no: Set(batch_no),
            color_no: Set(color_no),
            dye_lot_no: Set(dye_lot_no),
            grade: Set(if grade.is_empty() {
                "一等品".to_string()
            } else {
                grade
            }),
            quantity_on_hand: Set(meters),
            quantity_available: Set(meters),
            quantity_reserved: Set(Decimal::ZERO),
            quantity_incoming: Set(Decimal::ZERO),
            reorder_point: Set(Decimal::ZERO),
            max_stock_point: Set(Decimal::ZERO),
            reorder_quantity: Set(Decimal::ZERO),
            last_count_date: Set(None),
            last_movement_date: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            // 面料行业字段
            quantity_meters: Set(meters),
            quantity_kg: Set(kg),
            gram_weight: Set(gram_weight.and_then(Decimal::from_f64_retain)),
            width: Set(width.and_then(Decimal::from_f64_retain)),
            production_date: Set(production_date),
            expiry_date: Set(expiry_date),
            stock_status: Set("正常".to_string()),
            quality_status: Set("合格".to_string()),
            location_id: Set(None),
            shelf_no: Set(None),
            layer_no: Set(None),
            bin_location: Set(None),
            version: Set(0),
            quantity_shipped: Set(Decimal::ZERO),
            replenishment_strategy: Set("reorder_point".to_string()),
        };
        batch
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::bad_request(format!("创建批次失败：{}", e)))
    }

    /// 更新批次（部分字段）
    #[allow(clippy::too_many_arguments)]
    pub async fn update_batch_fields(
        &self,
        id: i32,
        color_no: Option<String>,
        dye_lot_no: Option<String>,
        grade: Option<String>,
        gram_weight: Option<f64>,
        width: Option<f64>,
        expiry_date: Option<chrono::DateTime<Utc>>,
        stock_status: Option<String>,
        quality_status: Option<String>,
    ) -> Result<inventory_stock::Model, AppError> {
        let existing = inventory_stock::Entity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("获取批次失败：{}", e)))?
            .ok_or_else(|| AppError::not_found("批次不存在"))?;

        let mut batch: inventory_stock::ActiveModel = existing.into();
        if let Some(color) = color_no {
            batch.color_no = Set(color);
        }
        if let Some(dye_lot) = dye_lot_no {
            batch.dye_lot_no = Set(Some(dye_lot));
        }
        if let Some(g) = grade {
            batch.grade = Set(g);
        }
        if let Some(gw) = gram_weight {
            batch.gram_weight = Set(Some(Decimal::from_f64_retain(gw).unwrap_or(Decimal::ZERO)));
        }
        if let Some(w) = width {
            batch.width = Set(Some(Decimal::from_f64_retain(w).unwrap_or(Decimal::ZERO)));
        }
        if let Some(exp) = expiry_date {
            batch.expiry_date = Set(Some(exp));
        }
        // 注意：inventory_stock 模型没有 remarks 字段，可以考虑使用其他方式存储
        if let Some(status) = stock_status {
            batch.stock_status = Set(status);
        }
        if let Some(quality) = quality_status {
            batch.quality_status = Set(quality);
        }
        batch.updated_at = Set(Utc::now());

        batch
            .update(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("更新批次失败：{}", e)))
    }

    /// 删除批次（审计日志下沉）
    pub async fn delete_batch_with_audit(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        // P0 8-3 修复：delete 操作补审计日志（批次 94 P2-10：真实操作人 user_id）
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            inventory_stock::Entity,
            _,
        >(&*self.db, "inventory_batch", id, Some(user_id))
        .await?;
        Ok(())
    }

    /// 批次转移（调拨）：扣减源批次，目标批次累加或新建（事务内完成）
    pub async fn transfer_batch(
        &self,
        id: i32,
        from_warehouse_id: i32,
        to_warehouse_id: i32,
        quantity_meters: f64,
        quantity_kg: f64,
    ) -> Result<(), AppError> {
        use sea_orm::TransactionTrait;

        let _ = from_warehouse_id; // 源仓库以批次记录自身 warehouse_id 为准
        let txn = (*self.db)
            .begin()
            .await
            .map_err(|e| AppError::database(format!("开启事务失败：{}", e)))?;

        let transfer_meters = Decimal::from_f64_retain(quantity_meters).unwrap_or(Decimal::ZERO);
        let transfer_kg = Decimal::from_f64_retain(quantity_kg).unwrap_or(Decimal::ZERO);

        // 1. 校验源批次存在且库存充足
        let source = inventory_stock::Entity::find_by_id(id)
            .one(&txn)
            .await
            .map_err(|e| AppError::database(format!("获取批次失败：{}", e)))?
            .ok_or_else(|| AppError::not_found("源批次不存在"))?;
        if source.quantity_available < transfer_meters {
            return Err(AppError::bad_request("库存数量不足"));
        }

        // 2. 扣减源批次
        let mut source_am: inventory_stock::ActiveModel = source.clone().into();
        source_am.quantity_on_hand = Set(source.quantity_on_hand - transfer_meters);
        source_am.quantity_available = Set(source.quantity_available - transfer_meters);
        source_am.updated_at = Set(Utc::now());
        source_am
            .update(&txn)
            .await
            .map_err(|e| AppError::bad_request(format!("更新源批次失败：{}", e)))?;

        // 3. 目标批次存在则累加，不存在则新建
        let target = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::WarehouseId.eq(to_warehouse_id))
            .filter(inventory_stock::Column::ProductId.eq(source.product_id))
            .filter(inventory_stock::Column::BatchNo.eq(source.batch_no.clone()))
            .filter(inventory_stock::Column::ColorNo.eq(source.color_no.clone()))
            .one(&txn)
            .await
            .map_err(|e| AppError::database(format!("查询目标批次失败：{}", e)))?;

        match target {
            Some(existing) => {
                let mut t: inventory_stock::ActiveModel = existing.clone().into();
                t.quantity_on_hand = Set(existing.quantity_on_hand + transfer_meters);
                t.quantity_available = Set(existing.quantity_available + transfer_meters);
                t.updated_at = Set(Utc::now());
                t.update(&txn)
                    .await
                    .map_err(|e| AppError::bad_request(format!("更新目标批次失败：{}", e)))?;
            }
            None => {
                let new_batch = inventory_stock::ActiveModel {
                    id: Set(0),
                    warehouse_id: Set(to_warehouse_id),
                    product_id: Set(source.product_id),
                    batch_no: Set(source.batch_no.clone()),
                    color_no: Set(source.color_no.clone()),
                    dye_lot_no: Set(source.dye_lot_no.clone()),
                    grade: Set(source.grade.clone()),
                    quantity_on_hand: Set(transfer_meters),
                    quantity_available: Set(transfer_meters),
                    quantity_reserved: Set(Decimal::ZERO),
                    quantity_incoming: Set(Decimal::ZERO),
                    reorder_point: Set(Decimal::ZERO),
                    max_stock_point: Set(Decimal::ZERO),
                    reorder_quantity: Set(Decimal::ZERO),
                    bin_location: Set(None),
                    last_count_date: Set(None),
                    last_movement_date: Set(None),
                    created_at: Set(Utc::now()),
                    updated_at: Set(Utc::now()),
                    quantity_meters: Set(transfer_meters),
                    quantity_kg: Set(transfer_kg),
                    gram_weight: Set(source.gram_weight),
                    width: Set(source.width),
                    production_date: Set(source.production_date),
                    expiry_date: Set(source.expiry_date),
                    stock_status: Set("正常".to_string()),
                    quality_status: Set("合格".to_string()),
                    location_id: Set(None),
                    shelf_no: Set(None),
                    layer_no: Set(None),
                    version: Set(0),
                    quantity_shipped: Set(Decimal::ZERO),
                    replenishment_strategy: Set("reorder_point".to_string()),
                };
                new_batch
                    .insert(&txn)
                    .await
                    .map_err(|e| AppError::bad_request(format!("创建目标批次失败：{}", e)))?;
            }
        }

        txn.commit()
            .await
            .map_err(|e| AppError::database(format!("提交事务失败：{}", e)))?;
        Ok(())
    }
}
