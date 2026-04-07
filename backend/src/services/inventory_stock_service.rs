use crate::models::inventory_stock;
use crate::models::inventory_transaction;
use crate::utils::dual_unit_converter::DualUnitConverter;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use std::sync::Arc;

/// 库存汇总项（用于返回汇总数据）
#[derive(Debug, Clone)]
#[allow(dead_code)]
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

/// 库存服务（面料行业版）
#[derive(Debug, Clone)]
pub struct InventoryStockService {
    db: Arc<DatabaseConnection>,
}

impl InventoryStockService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn find_by_id(&self, id: i32) -> Result<inventory_stock::Model, sea_orm::DbErr> {
        inventory_stock::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("库存记录 ID {} 不存在", id)))
    }

    #[allow(dead_code)]
    pub async fn find_by_product_and_warehouse(
        &self,
        product_id: i32,
        warehouse_id: i32,
    ) -> Result<Option<inventory_stock::Model>, sea_orm::DbErr> {
        inventory_stock::Entity::find()
            .filter(inventory_stock::Column::ProductId.eq(product_id))
            .filter(inventory_stock::Column::WarehouseId.eq(warehouse_id))
            .one(&*self.db)
            .await
    }

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
    ) -> Result<inventory_stock::Model, sea_orm::DbErr> {
        let active_stock = inventory_stock::ActiveModel {
            id: Set(0),
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
            location_id: Set(None),
            shelf_no: Set(None),
            layer_no: Set(None),
            bin_location: Set(None),
            stock_status: Set(stock_status),
            quality_status: Set(quality_status),
        };

        active_stock.insert(&*self.db).await
    }

    #[allow(dead_code)]
    pub async fn update_stock_quantity(
        &self,
        id: i32,
        quantity_meters: Decimal,
        quantity_kg: Decimal,
    ) -> Result<inventory_stock::Model, sea_orm::DbErr> {
        let mut stock: inventory_stock::ActiveModel = inventory_stock::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("库存记录 ID {} 不存在", id)))?
            .into();

        stock.quantity_on_hand = Set(quantity_meters);
        stock.quantity_available = Set(quantity_meters);
        stock.quantity_meters = Set(quantity_meters);
        stock.quantity_kg = Set(quantity_kg);
        stock.updated_at = Set(chrono::Utc::now());

        stock.update(&*self.db).await
    }

    pub async fn list_stock(
        &self,
        page: u64,
        page_size: u64,
        warehouse_id: Option<i32>,
        product_id: Option<i32>,
    ) -> Result<(Vec<inventory_stock::Model>, u64), sea_orm::DbErr> {
        let mut query = inventory_stock::Entity::find();

        if let Some(wid) = warehouse_id {
            query = query.filter(inventory_stock::Column::WarehouseId.eq(wid));
        }

        if let Some(pid) = product_id {
            query = query.filter(inventory_stock::Column::ProductId.eq(pid));
        }

        let paginator = query.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let stock_list = paginator.fetch_page(page).await?;

        Ok((stock_list, total))
    }

    pub async fn check_low_stock(
        &self,
        warehouse_id: Option<i32>,
        product_id: Option<i32>,
        batch_no: Option<String>,
    ) -> Result<Vec<inventory_stock::Model>, sea_orm::DbErr> {
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

        query.all(&*self.db).await
    }

    #[allow(dead_code)]
    pub async fn delete_stock(&self, id: i32) -> Result<(), sea_orm::DbErr> {
        inventory_stock::Entity::delete_many()
            .filter(inventory_stock::Column::Id.eq(id))
            .exec(&*self.db)
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
    ) -> Result<Vec<inventory_stock::Model>, sea_orm::DbErr> {
        let mut query = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::BatchNo.eq(batch_no))
            .filter(inventory_stock::Column::ColorNo.eq(color_no));

        if let Some(wid) = warehouse_id {
            query = query.filter(inventory_stock::Column::WarehouseId.eq(wid));
        }

        query.all(&*self.db).await
    }

    /// 查询库存（支持批次、色号、等级过滤）
    #[allow(dead_code)]
    pub async fn list_stock_fabric(
        &self,
        page: u64,
        page_size: u64,
        warehouse_id: Option<i32>,
        product_id: Option<i32>,
        batch_no: Option<String>,
        color_no: Option<String>,
        grade: Option<String>,
    ) -> Result<(Vec<inventory_stock::Model>, u64), sea_orm::DbErr> {
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

        let paginator = query.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let stock_list = paginator.fetch_page(page).await?;

        Ok((stock_list, total))
    }

    // ========== 双计量单位自动计算辅助方法 ==========

    /// 自动计算公斤数（如果提供了克重和幅宽）
    fn calculate_quantity_kg(
        quantity_meters: Decimal,
        gram_weight: Option<Decimal>,
        width: Option<Decimal>,
    ) -> Decimal {
        if let Some(gram_weight) = gram_weight {
            if let Some(width) = width {
                // 使用双计量单位转换器进行精确计算
                match DualUnitConverter::meters_to_kg(quantity_meters, gram_weight, width) {
                    Ok(kg) => return kg,
                    Err(_) => {
                        // 如果计算失败，回退到直接传入的公斤数
                        eprintln!("双计量单位换算失败，使用原始公斤数");
                    }
                }
            }
        }
        Decimal::ZERO
    }

    /// 更新或创建库存事务支持
    #[allow(clippy::too_many_arguments)]
    pub async fn update_or_create_stock_with_txn<C: sea_orm::ConnectionTrait>(
        &self,
        txn: &C,
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
    ) -> Result<inventory_stock::Model, sea_orm::DbErr> {
        let calculated_kg = Self::calculate_quantity_kg(quantity_meters, gram_weight, width);
        let final_quantity_kg = if calculated_kg != Decimal::ZERO {
            calculated_kg
        } else {
            quantity_kg
        };

        // 尝试查找现有库存（相同仓库、产品、批次、颜色、缸号、等级）
        let mut query = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::WarehouseId.eq(warehouse_id))
            .filter(inventory_stock::Column::ProductId.eq(product_id))
            .filter(inventory_stock::Column::BatchNo.eq(batch_no.clone()))
            .filter(inventory_stock::Column::ColorNo.eq(color_no.clone()))
            .filter(inventory_stock::Column::Grade.eq(grade.clone()));

        if let Some(dl) = &dye_lot_no {
            query = query.filter(inventory_stock::Column::DyeLotNo.eq(dl.clone()));
        } else {
            query = query.filter(inventory_stock::Column::DyeLotNo.is_null());
        }

        if let Some(existing_stock) = query.one(txn).await? {
            let mut active_stock: inventory_stock::ActiveModel = existing_stock.into();
            let old_qty_meters = active_stock.quantity_meters.as_ref().clone();
            let old_qty_kg = active_stock.quantity_kg.as_ref().clone();
            let old_on_hand = active_stock.quantity_on_hand.as_ref().clone();
            let old_available = active_stock.quantity_available.as_ref().clone();

            active_stock.quantity_meters = Set(old_qty_meters + quantity_meters);
            active_stock.quantity_kg = Set(old_qty_kg + final_quantity_kg);
            active_stock.quantity_on_hand = Set(old_on_hand + quantity_meters);
            active_stock.quantity_available = Set(old_available + quantity_meters);
            active_stock.updated_at = Set(Utc::now());

            active_stock.update(txn).await
        } else {
            let active_stock = inventory_stock::ActiveModel {
                id: Set(0),
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
                location_id: Set(location_id),
                shelf_no: Set(shelf_no),
                layer_no: Set(layer_no),
                bin_location: Set(None),
                stock_status: Set("正常".to_string()),
                quality_status: Set("合格".to_string()),
            };
            active_stock.insert(txn).await
        }
    }

    /// 创建库存（面料行业版）事务支持
    #[allow(clippy::too_many_arguments)]
    pub async fn create_stock_fabric_with_txn<C: sea_orm::ConnectionTrait>(
        &self,
        txn: &C,
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
    ) -> Result<inventory_stock::Model, sea_orm::DbErr> {
        let calculated_kg = Self::calculate_quantity_kg(quantity_meters, gram_weight, width);
        let final_quantity_kg = if calculated_kg != Decimal::ZERO {
            calculated_kg
        } else {
            quantity_kg
        };

        let active_stock = inventory_stock::ActiveModel {
            id: Set(0),
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
            location_id: Set(location_id),
            shelf_no: Set(shelf_no),
            layer_no: Set(layer_no),
            bin_location: Set(None),
            stock_status: Set("正常".to_string()),
            quality_status: Set("合格".to_string()),
        };

        active_stock.insert(txn).await
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
    ) -> Result<inventory_stock::Model, sea_orm::DbErr> {
        // 自动计算公斤数（如果提供了克重和幅宽）
        let calculated_kg = Self::calculate_quantity_kg(quantity_meters, gram_weight, width);
        let final_quantity_kg = if calculated_kg != Decimal::ZERO {
            calculated_kg
        } else {
            quantity_kg
        };

        let active_stock = inventory_stock::ActiveModel {
            id: Set(0),
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
            location_id: Set(location_id),
            shelf_no: Set(shelf_no),
            layer_no: Set(layer_no),
            bin_location: Set(None),
            stock_status: Set("正常".to_string()),
            quality_status: Set("合格".to_string()),
        };

        active_stock.insert(&*self.db).await
    }

    /// 记录库存流水（面料行业）事务支持
    #[allow(clippy::too_many_arguments)]
    pub async fn record_transaction_with_txn<C: sea_orm::ConnectionTrait>(
        &self,
        txn: &C,
        transaction_type: String,
        product_id: i32,
        warehouse_id: i32,
        batch_no: String,
        color_no: String,
        dye_lot_no: Option<String>,
        grade: String,
        quantity_meters: Decimal,
        quantity_kg: Decimal,
        source_bill_type: Option<String>,
        source_bill_no: Option<String>,
        source_bill_id: Option<i32>,
        quantity_before_meters: Option<Decimal>,
        quantity_before_kg: Option<Decimal>,
        quantity_after_meters: Option<Decimal>,
        quantity_after_kg: Option<Decimal>,
        notes: Option<String>,
        created_by: Option<i32>,
    ) -> Result<inventory_transaction::Model, sea_orm::DbErr> {
        let active_transaction = inventory_transaction::ActiveModel {
            id: Set(0),
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

        active_transaction.insert(txn).await
    }

    /// 记录库存流水（面料行业）
    #[allow(clippy::too_many_arguments)]
    #[allow(dead_code)]
    pub async fn record_transaction(
        &self,
        transaction_type: String,
        product_id: i32,
        warehouse_id: i32,
        batch_no: String,
        color_no: String,
        dye_lot_no: Option<String>,
        grade: String,
        quantity_meters: Decimal,
        quantity_kg: Decimal,
        source_bill_type: Option<String>,
        source_bill_no: Option<String>,
        source_bill_id: Option<i32>,
        quantity_before_meters: Option<Decimal>,
        quantity_before_kg: Option<Decimal>,
        quantity_after_meters: Option<Decimal>,
        quantity_after_kg: Option<Decimal>,
        notes: Option<String>,
        created_by: Option<i32>,
    ) -> Result<inventory_transaction::Model, sea_orm::DbErr> {
        let active_transaction = inventory_transaction::ActiveModel {
            id: Set(0),
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

        active_transaction.insert(&*self.db).await
    }

    /// 查询库存流水
    pub async fn list_transactions(
        &self,
        page: u64,
        page_size: u64,
        batch_no: Option<String>,
        color_no: Option<String>,
        product_id: Option<i32>,
        warehouse_id: Option<i32>,
        transaction_type: Option<String>,
        start_date: Option<chrono::NaiveDateTime>,
        end_date: Option<chrono::NaiveDateTime>,
    ) -> Result<(Vec<inventory_transaction::Model>, u64), sea_orm::DbErr> {
        let mut query = inventory_transaction::Entity::find()
            .order_by(inventory_transaction::Column::CreatedAt, Order::Asc);

        if let Some(batch) = batch_no {
            query = query.filter(inventory_transaction::Column::BatchNo.eq(batch));
        }

        if let Some(color) = color_no {
            query = query.filter(inventory_transaction::Column::ColorNo.eq(color));
        }

        if let Some(pid) = product_id {
            query = query.filter(inventory_transaction::Column::ProductId.eq(pid));
        }

        if let Some(wid) = warehouse_id {
            query = query.filter(inventory_transaction::Column::WarehouseId.eq(wid));
        }

        if let Some(transaction_type) = transaction_type {
            query =
                query.filter(inventory_transaction::Column::TransactionType.eq(transaction_type));
        }

        if let Some(start_date) = start_date {
            query = query.filter(inventory_transaction::Column::CreatedAt.gte(start_date));
        }

        if let Some(end_date) = end_date {
            query = query.filter(inventory_transaction::Column::CreatedAt.lte(end_date));
        }

        let paginator = query.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let transactions = paginator.fetch_page(page).await?;

        Ok((transactions, total))
    }

    /// 获取库存汇总（按批次 + 色号）
    pub async fn get_inventory_summary(
        &self,
        warehouse_id: Option<i32>,
        product_id: Option<i32>,
        batch_no: Option<String>,
        color_no: Option<String>,
        grade: Option<String>,
    ) -> Result<Vec<InventorySummaryItem>, sea_orm::DbErr> {
        use sea_orm::ConnectionTrait;

        let mut where_clauses: Vec<String> = vec![
            "s.stock_status = '正常'".to_string(),
            "s.quality_status = '合格'".to_string(),
        ];

        let mut values = vec![];

        if let Some(wid) = warehouse_id {
            where_clauses.push(format!("w.id = ${}", values.len() + 1));
            values.push(wid.into());
        }

        if let Some(pid) = product_id {
            where_clauses.push(format!("p.id = ${}", values.len() + 1));
            values.push(pid.into());
        }

        if let Some(batch) = batch_no {
            where_clauses.push(format!("s.batch_no = ${}", values.len() + 1));
            values.push(batch.into());
        }

        if let Some(color) = color_no {
            where_clauses.push(format!("s.color_no = ${}", values.len() + 1));
            values.push(color.into());
        }

        if let Some(g) = grade {
            where_clauses.push(format!("s.grade = ${}", values.len() + 1));
            values.push(g.into());
        }

        let where_clause = where_clauses.join(" AND ");

        let sql = format!(
            r#"
        SELECT
            p.id AS product_id,
            p.name AS product_name,
            w.name AS warehouse_name,
            s.batch_no,
            s.color_no,
            s.grade,
            SUM(s.quantity_meters) AS total_quantity_meters,
            SUM(s.quantity_kg) AS total_quantity_kg
        FROM inventory_stock s
        INNER JOIN products p ON s.product_id = p.id
        INNER JOIN warehouses w ON s.warehouse_id = w.id
        WHERE {}
        GROUP BY p.id, p.name, w.name, s.batch_no, s.color_no, s.grade
        ORDER BY s.batch_no, s.color_no
    "#,
            where_clause
        );

        let result = self
            .db
            .query_all(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                values,
            ))
            .await?;

        Ok(result
            .into_iter()
            .map(|row| InventorySummaryItem {
                product_id: row.try_get("", "product_id").unwrap_or(0),
                product_name: row.try_get("", "product_name").unwrap_or("".to_string()),
                specification: None,
                color_no: row.try_get("", "color_no").unwrap_or("".to_string()),
                batch_no: row.try_get("", "batch_no").unwrap_or("".to_string()),
                grade: row.try_get("", "grade").unwrap_or("".to_string()),
                warehouse_id: 0,
                warehouse_name: row.try_get("", "warehouse_name").unwrap_or("".to_string()),
                quantity: Decimal::ZERO,
                unit: String::new(),
                total_value: None,
                total_quantity_meters: row
                    .try_get("", "total_quantity_meters")
                    .unwrap_or(Decimal::ZERO),
                total_quantity_kg: row
                    .try_get("", "total_quantity_kg")
                    .unwrap_or(Decimal::ZERO),
            })
            .collect())
    }
}
