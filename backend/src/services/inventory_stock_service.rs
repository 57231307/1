use crate::models::inventory_stock;
use crate::models::inventory_transaction;
use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
use crate::utils::dual_unit_converter::DualUnitConverter;
use crate::utils::error::AppError;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
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
struct InventorySummaryQueryResult {
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
    db: Arc<DatabaseConnection>,
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

    #[allow(clippy::too_many_arguments)]
    pub async fn create_stock(
        &self,
        warehouse_id: i32,
        product_id: i32,
        batch_no: String,
        color_no: String,
        quantity_meters: Decimal,
        quantity_kg: Decimal,
        grade: String,
        dye_lot_no: Option<String>,
        gram_weight: Option<Decimal>,
        width: Option<Decimal>,
        stock_status: String,
        quality_status: String,
    ) -> Result<inventory_stock::Model, AppError> {
        let active_stock = inventory_stock::ActiveModel {
            id: Default::default(),
            warehouse_id: Set(warehouse_id),
            product_id: Set(product_id),
            quantity_on_hand: Set(quantity_meters),
            quantity_available: Set(quantity_meters),
            quantity_reserved: Set(Decimal::ZERO),
            quantity_incoming: Set(Decimal::ZERO),
            reorder_point: Set(Decimal::ZERO),
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

    /// 更新库存数量（带乐观锁）
    pub async fn update_stock_quantity_with_optimistic_lock(
        &self,
        id: i32,
        quantity_meters: Decimal,
        quantity_kg: Decimal,
        expected_version: i32,
    ) -> Result<inventory_stock::Model, AppError> {
        // 使用乐观锁条件更新：只有 version 匹配时才更新
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
                sea_orm::sea_query::Expr::col(inventory_stock::Column::Version).add(1).into(),
            )
            .col_expr(
                inventory_stock::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
            )
            .filter(inventory_stock::Column::Id.eq(id))
            .filter(inventory_stock::Column::Version.eq(expected_version))
            .exec(&*self.db)
            .await?;

        // 检查乐观锁是否成功
        if update_result.rows_affected == 0 {
            return Err(AppError::business(format!(
                "并发冲突：库存记录 ID {} 已被其他用户修改，期望版本 {}",
                id, expected_version
            )));
        }

        // 返回更新后的记录
        inventory_stock::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存记录 ID {} 不存在", id)))
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

        // 并行执行分页查询：同时获取总数和当前页数据，提升性能
        let paginator = query.paginate(&*self.db, page_size);
        let (total, stock_list) =
            tokio::try_join!(paginator.num_items(), paginator.fetch_page(page))?;

        Ok((stock_list, total))
    }

    pub async fn check_low_stock(
        &self,
        warehouse_id: Option<i32>,
        product_id: Option<i32>,
        batch_no: Option<String>,
    ) -> Result<Vec<inventory_stock::Model>, AppError> {
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

        let low_stock_items = query.all(&*self.db).await?;

        // 触发低库存预警事件
        for item in &low_stock_items {
            let event = BusinessEvent::LowStockAlert {
                product_id: item.product_id,
                warehouse_id: item.warehouse_id,
                current_quantity: item.quantity_available,
                reorder_point: item.reorder_point,
                reorder_quantity: item.reorder_quantity,
            };
            EVENT_BUS.publish(event);
            tracing::info!(
                "触发低库存预警事件: 产品ID={}, 仓库ID={}, 当前库存={}, 补货点={}, 补货量={}",
                item.product_id,
                item.warehouse_id,
                item.quantity_available,
                item.reorder_point,
                item.reorder_quantity
            );
        }

        Ok(low_stock_items)
    }

    pub async fn delete_stock(&self, id: i32) -> Result<(), AppError> {
        let stock = self.find_by_id(id).await?;
        let mut active_model: inventory_stock::ActiveModel = stock.into();
        active_model.stock_status = Set("已删除".to_string());
        active_model.updated_at = Set(Utc::now());
        active_model.update(&*self.db).await?;
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

    /// 查询库存（支持批次、色号、等级过滤）
    #[allow(clippy::too_many_arguments)]
    pub async fn list_stock_fabric(
        &self,
        page: u64,
        page_size: u64,
        warehouse_id: Option<i32>,
        product_id: Option<i32>,
        batch_no: Option<String>,
        color_no: Option<String>,
        grade: Option<String>,
    ) -> Result<(Vec<inventory_stock::Model>, u64), AppError> {
        let mut query = inventory_stock::Entity::find();

        if let Some(wid) = warehouse_id {
            query = query.filter(inventory_stock::Column::WarehouseId.eq(wid));
        }

        if let Some(pid) = product_id {
            query = query.filter(inventory_stock::Column::ProductId.eq(pid));
        }

        if let Some(batch) = batch_no {
            query = query.filter(inventory_stock::Column::BatchNo.eq(batch));
        }

        if let Some(color) = color_no {
            query = query.filter(inventory_stock::Column::ColorNo.eq(color));
        }

        if let Some(g) = grade {
            query = query.filter(inventory_stock::Column::Grade.eq(g));
        }

        // 并行执行分页查询：同时获取总数和当前页数据，提升性能
        let paginator = query.paginate(&*self.db, page_size);
        let (total, stock_list) =
            tokio::try_join!(paginator.num_items(), paginator.fetch_page(page))?;

        Ok((stock_list, total))
    }

    // ========== 双计量单位自动计算辅助方法 ==========

    /// 自动计算公斤数（如果提供了克重和幅宽）
    fn calculate_quantity_kg(
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
    #[allow(clippy::too_many_arguments)]
    pub async fn create_stock_fabric(
        &self,
        warehouse_id: i32,
        product_id: i32,
        batch_no: String,
        color_no: String,
        dye_lot_no: Option<String>,
        grade: String,
        quantity_meters: Decimal,
        quantity_kg: Decimal,
        gram_weight: Option<Decimal>,
        width: Option<Decimal>,
        location_id: Option<i32>,
        shelf_no: Option<String>,
        layer_no: Option<String>,
    ) -> Result<inventory_stock::Model, AppError> {
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
