//! 库存事务记录 + 汇总查询方法（record_transaction / list_transactions / summary / alerts）
//!
//! 拆分自 inventory_stock_service.rs：原 6 个事务记录与汇总方法独立成文件。

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::handlers::inventory_stock_handler_dto::InventorySummaryItem;
use crate::models::{inventory_stock, inventory_transaction};
use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
use crate::utils::error::AppError;

use super::inventory_stock_service::{InventoryStockService, InventorySummaryQueryResult};

impl InventoryStockService {
    // TODO(tech-debt): 库存流水记录字段较多，后续可通过 DTO 聚合参数以收敛签名长度，移除此标注。
    #[allow(clippy::too_many_arguments)]
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
    ) -> Result<inventory_transaction::Model, AppError> {
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

        let transaction = active_transaction.insert(&*self.db).await?;

        // 触发库存交易创建事件
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
        EVENT_BUS.publish(event);

        Ok(transaction)
    }

    /// 查询库存流水
    #[allow(clippy::too_many_arguments)]
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
    ) -> Result<(Vec<inventory_transaction::Model>, u64), AppError> {
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

        // 并行执行分页查询：同时获取总数和当前页数据，提升性能
        let paginator = query.paginate(&*self.db, page_size);
        let (total, transactions) =
            tokio::try_join!(paginator.num_items(), paginator.fetch_page(page))?;

        Ok((transactions, total))
    }

    /// 获取库存汇总（按批次 + 色号）
    ///
    /// # 参数
    /// - `warehouse_id`: 仓库ID筛选
    /// - `product_id`: 产品ID筛选
    /// - `batch_no`: 批次号筛选
    /// - `color_no`: 色号筛选
    /// - `grade`: 等级筛选
    /// - `page`: 页码（从1开始）
    /// - `page_size`: 每页大小
    ///
    /// # 返回
    /// 返回分页结果，包含数据列表和总记录数
    // TODO(tech-debt): 业务上要求按多个维度筛选+分页，可选参数较多；后续可通过
    // InventorySummaryQuery DTO 聚合参数以收敛签名长度，移除此标注。
    #[allow(clippy::too_many_arguments)]
    pub async fn get_inventory_summary(
        &self,
        warehouse_id: Option<i32>,
        product_id: Option<i32>,
        batch_no: Option<String>,
        color_no: Option<String>,
        grade: Option<String>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<InventorySummaryItem>, u64), AppError> {
        use sea_orm::QuerySelect;

        let mut query = inventory_stock::Entity::find()
            .inner_join(crate::models::product::Entity)
            .inner_join(crate::models::warehouse::Entity)
            .select_only()
            .column_as(inventory_stock::Column::ProductId, "product_id")
            .column_as(crate::models::product::Column::Name, "product_name")
            .column_as(inventory_stock::Column::WarehouseId, "warehouse_id")
            .column_as(crate::models::warehouse::Column::Name, "warehouse_name")
            .column_as(inventory_stock::Column::BatchNo, "batch_no")
            .column_as(inventory_stock::Column::ColorNo, "color_no")
            .column_as(inventory_stock::Column::Grade, "grade")
            .column_as(
                inventory_stock::Column::QuantityMeters.sum(),
                "total_quantity_meters",
            )
            .column_as(
                inventory_stock::Column::QuantityKg.sum(),
                "total_quantity_kg",
            )
            .group_by(inventory_stock::Column::ProductId)
            .group_by(crate::models::product::Column::Name)
            .group_by(inventory_stock::Column::WarehouseId)
            .group_by(crate::models::warehouse::Column::Name)
            .group_by(inventory_stock::Column::BatchNo)
            .group_by(inventory_stock::Column::ColorNo)
            .group_by(inventory_stock::Column::Grade)
            .order_by_asc(inventory_stock::Column::BatchNo)
            .order_by_asc(inventory_stock::Column::ColorNo);

        // 添加过滤条件
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

        // 添加库存状态和质量状态过滤
        query = query
            .filter(inventory_stock::Column::StockStatus.eq("正常"))
            .filter(inventory_stock::Column::QualityStatus.eq("合格"));

        // 查询总记录数
        let total = query.clone().count(&*self.db).await?;

        // 查询分页数据
        let result = query
            .into_model::<InventorySummaryQueryResult>()
            .paginate(&*self.db, page_size)
            .fetch_page(page.saturating_sub(1))
            .await?;

        let items = result
            .into_iter()
            .map(|r| InventorySummaryItem {
                product_id: r.product_id,
                product_name: r.product_name,
                batch_no: r.batch_no,
                color_no: r.color_no,
                grade: r.grade,
                warehouse_name: r.warehouse_name,
                total_quantity_meters: r.total_quantity_meters,
                total_quantity_kg: r.total_quantity_kg,
            })
            .collect();

        Ok((items, total))
    }

    /// 按产品查询库存
    pub async fn get_stock_by_product(
        &self,
        product_id: i32,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<inventory_stock::Model>, u64), AppError> {
        let paginator = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::ProductId.eq(product_id))
            .order_by_asc(inventory_stock::Column::Id)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let stocks = paginator.fetch_page(page).await?;

        Ok((stocks, total))
    }

    /// 获取库存告警
    pub async fn get_stock_alerts(
        &self,
        query: serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        let warehouse_id = query
            .get("warehouse_id")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);
        let product_id = query
            .get("product_id")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);

        // 简化实现：返回低库存告警
        let mut stock_query = inventory_stock::Entity::find()
            .inner_join(crate::models::product::Entity)
            .inner_join(crate::models::warehouse::Entity);

        if let Some(wid) = warehouse_id {
            stock_query = stock_query.filter(inventory_stock::Column::WarehouseId.eq(wid));
        }
        if let Some(pid) = product_id {
            stock_query = stock_query.filter(inventory_stock::Column::ProductId.eq(pid));
        }

        let stocks = stock_query.all(&*self.db).await?;

        let alert_list: Vec<serde_json::Value> = stocks
            .into_iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "product_id": s.product_id,
                    "warehouse_id": s.warehouse_id,
                    "quantity_on_hand": s.quantity_on_hand.to_string(),
                    "quantity_available": s.quantity_available.to_string(),
                    "quantity_reserved": s.quantity_reserved.to_string(),
                    "alert_type": "normal",
                })
            })
            .collect();

        Ok(serde_json::json!({
            "list": alert_list,
            "total": alert_list.len(),
        }))
    }
}
