use crate::models::sales_order;
use sea_orm::{EntityTrait, Set, QueryFilter, ColumnTrait, ActiveModelTrait, PaginatorTrait};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct SalesOrderService {
    db: Arc<DatabaseConnection>,
}

impl SalesOrderService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn find_by_id(&self, id: i32) -> Result<sales_order::Model, sea_orm::DbErr> {
        sales_order::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("订单 ID {} 不存在", id)))
    }

    pub async fn find_by_order_no(&self, order_no: &str) -> Result<Option<sales_order::Model>, sea_orm::DbErr> {
        sales_order::Entity::find()
            .filter(sales_order::Column::OrderNo.eq(order_no))
            .one(&*self.db)
            .await
    }

    pub async fn create_order(
        &self,
        order_no: String,
        customer_id: i32,
        order_date: DateTime<Utc>,
        required_date: DateTime<Utc>,
        shipping_address: Option<String>,
        billing_address: Option<String>,
        notes: Option<String>,
        created_by: Option<i32>,
    ) -> Result<sales_order::Model, sea_orm::DbErr> {
        let active_order = sales_order::ActiveModel {
            id: Set(0),
            order_no: Set(order_no),
            customer_id: Set(customer_id),
            order_date: Set(order_date),
            required_date: Set(required_date),
            ship_date: Set(None),
            status: Set("pending".to_string()),
            subtotal: Set(Decimal::ZERO),
            tax_amount: Set(Decimal::ZERO),
            discount_amount: Set(Decimal::ZERO),
            shipping_cost: Set(Decimal::ZERO),
            total_amount: Set(Decimal::ZERO),
            paid_amount: Set(Decimal::ZERO),
            balance_amount: Set(Decimal::ZERO),
            shipping_address: Set(shipping_address),
            billing_address: Set(billing_address),
            notes: Set(notes),
            created_by: Set(created_by),
            approved_by: Set(None),
            approved_at: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        active_order.insert(&*self.db).await
    }

    pub async fn update_order_status(
        &self,
        id: i32,
        status: String,
        approved_by: Option<i32>,
    ) -> Result<sales_order::Model, sea_orm::DbErr> {
        let mut order: sales_order::ActiveModel = sales_order::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("订单 ID {} 不存在", id)))?
            .into();

        order.status = Set(status);
        order.approved_by = Set(approved_by);
        order.approved_at = Set(Some(Utc::now()));
        order.updated_at = Set(Utc::now());

        order.update(&*self.db).await
    }

    pub async fn list_orders(
        &self,
        page: u64,
        page_size: u64,
        customer_id: Option<i32>,
        status: Option<String>,
    ) -> Result<(Vec<sales_order::Model>, u64), sea_orm::DbErr> {
        let mut query = sales_order::Entity::find();

        if let Some(cid) = customer_id {
            query = query.filter(sales_order::Column::CustomerId.eq(cid));
        }

        if let Some(s) = status {
            query = query.filter(sales_order::Column::Status.eq(s));
        }

        let paginator = query.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let orders = paginator.fetch_page(page).await?;

        Ok((orders, total))
    }

    pub async fn delete_order(&self, id: i32) -> Result<(), sea_orm::DbErr> {
        sales_order::Entity::delete_many()
            .filter(sales_order::Column::Id.eq(id))
            .exec(&*self.db)
            .await?;
        Ok(())
    }
}
