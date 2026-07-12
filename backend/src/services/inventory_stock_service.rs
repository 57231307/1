use crate::models::inventory_stock;
use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
use crate::utils::dual_unit_converter::DualUnitConverter;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set, TransactionTrait,
};
use std::sync::Arc;

/// 库存汇总项（用于返回汇总数据）
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct InventoryStockService {
    pub db: Arc<DatabaseConnection>,
}

/// 创建库存参数对象
///
/// 批次 338 v10 复审 P3 修复：引入参数对象消除 create_stock 的 too_many_arguments 警告。
/// 聚合创建库存记录所需的全部字段，避免函数签名携带 12 个参数。
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
///
/// 批次 338 v10 复审 P3 修复：引入参数对象消除 create_stock_fabric 的 too_many_arguments 警告。
/// 聚合创建面料库存记录所需的全部字段，避免函数签名携带 13 个参数。
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

impl InventoryStockService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn find_by_id(&self, id: i32) -> Result<inventory_stock::Model, AppError> {
        inventory_stock::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存记录 ID {} 不存在", id)))
    }

    /// 创建库存
    ///
    /// 批次 338 v10 复审 P3 修复：签名从 12 参数改为单一参数对象 `CreateStockArgs`，
    /// 消除 `clippy::too_many_arguments` 警告。
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
            .ok_or_else(|| {
                AppError::validation(format!("仓库不存在: {}", warehouse_id))
            })?;
        let _product = product::Entity::find_by_id(product_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::validation(format!("产品不存在: {}", product_id))
            })?;

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
        );
        let paginator = query.paginate(&*self.db, page_size);
        let (stock_list, total) =
            paginate_with_total(paginator, page.clamp(1, 1000)).await?;
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

    /// 创建库存（面料行业版）
    ///
    /// 批次 338 v10 复审 P3 修复：签名从 13 参数改为单一参数对象 `CreateStockFabricArgs`，
    /// 消除 `clippy::too_many_arguments` 警告。
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
        use crate::models::{product, warehouse};
        use sea_orm::EntityTrait;

        let _warehouse = warehouse::Entity::find_by_id(warehouse_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::validation(format!("仓库不存在: {}", warehouse_id))
            })?;
        let _product = product::Entity::find_by_id(product_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::validation(format!("产品不存在: {}", product_id))
            })?;

        // 自动计算公斤数（如果提供了克重和幅宽）
        let final_quantity_kg =
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
        };

        active_stock.insert(&*self.db).await.map_err(AppError::from)
    }
}
