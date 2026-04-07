use crate::models::{sales_delivery, sales_delivery_item};
use crate::models::dto::sales_delivery_dto::{CreateSalesDeliveryRequest, SalesDeliveryQueryParams};
use crate::utils::error::AppError;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use std::sync::Arc;

pub struct SalesDeliveryService {
    db: Arc<DatabaseConnection>,
}

impl SalesDeliveryService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn generate_delivery_no(&self) -> Result<String, AppError> {
        let today = Utc::now().format("%Y%m%d").to_string();
        let prefix = format!("SD{}", today);

        let count = sales_delivery::Entity::find()
            .filter(sales_delivery::Column::DeliveryNo.starts_with(&prefix))
            .count(&*self.db)
            .await?;

        Ok(format!("{}{:03}", prefix, count + 1))
    }

    pub async fn create_delivery(
        &self,
        req: CreateSalesDeliveryRequest,
        user_id: i32,
    ) -> Result<sales_delivery::Model, AppError> {
        let txn = (*self.db).begin().await?;

        let delivery_no = self.generate_delivery_no().await?;

        let delivery = sales_delivery::ActiveModel {
            delivery_no: Set(delivery_no),
            order_id: Set(req.order_id),
            customer_id: Set(req.customer_id),
            delivery_date: Set(req.delivery_date),
            warehouse_id: Set(req.warehouse_id),
            status: Set(req.status),
            total_quantity: Set(req.total_quantity),
            total_amount: Set(req.total_amount),
            remarks: Set(req.remarks),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        for item_req in req.items {
            sales_delivery_item::ActiveModel {
                delivery_id: Set(delivery.id),
                product_id: Set(item_req.product_id),
                batch_no: Set(item_req.batch_no),
                color_no: Set(item_req.color_no),
                quantity: Set(item_req.quantity),
                unit_price: Set(item_req.unit_price),
                amount: Set(item_req.amount),
                remarks: Set(item_req.remarks),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }

        txn.commit().await?;

        Ok(delivery)
    }

    pub async fn list_deliveries(
        &self,
        params: SalesDeliveryQueryParams,
    ) -> Result<(Vec<sales_delivery::Model>, u64), AppError> {
        let mut query = sales_delivery::Entity::find();

        if let Some(delivery_no) = params.delivery_no {
            query = query.filter(sales_delivery::Column::DeliveryNo.contains(&delivery_no));
        }

        if let Some(order_id) = params.order_id {
            query = query.filter(sales_delivery::Column::OrderId.eq(order_id));
        }

        if let Some(customer_id) = params.customer_id {
            query = query.filter(sales_delivery::Column::CustomerId.eq(customer_id));
        }

        if let Some(warehouse_id) = params.warehouse_id {
            query = query.filter(sales_delivery::Column::WarehouseId.eq(warehouse_id));
        }

        if let Some(status) = params.status {
            query = query.filter(sales_delivery::Column::Status.eq(status));
        }

        if let Some(start_date) = params.start_date {
            query = query.filter(sales_delivery::Column::DeliveryDate.gte(start_date));
        }

        if let Some(end_date) = params.end_date {
            query = query.filter(sales_delivery::Column::DeliveryDate.lte(end_date));
        }

        let page = params.page.unwrap_or(1);
        let page_size = params.page_size.unwrap_or(20);

        let paginator = query
            .order_by(sales_delivery::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page - 1).await?;

        Ok((items, total))
    }
}
